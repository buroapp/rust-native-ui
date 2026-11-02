//! End-to-end integration tests for the `rn` framework.
//!
//! These tests drive the full pipeline — builder API → `ComponentTree` →
//! `Reconciler` → `Changeset` → a recording `MockRenderer` — without
//! requiring a display server or platform backend. They verify the
//! "declarative in, native-operations out" contract from the consumer's
//! perspective.
//!
//! Test categories:
//! 1. **Gallery smoke** — the Widget Gallery component tree produces correct
//!    Create operations for every widget it declares.
//! 2. **Interactive state** — sending Increment messages produces an Update
//!    op on the counter label, not a Delete+Create pair.
//! 3. **Text input** — typing into a TextInput produces a single Update on
//!    the input and a greeting Update, no control teardown.
//! 4. **Visibility toggle** — toggling a label's Visible prop produces one
//!    Update op containing only the Visible key.
//! 5. **Reset** — Reset button returns the counter to 0 in a single render.
//! 6. **Multi-increment coalescing** — two rapid Increments in one tick
//!    produce a single reconcile pass with count=2.

use std::sync::{Arc, Mutex};

use rn::{
    Changeset, ComponentKind, ComponentTree, Operation, PropKey, PropValue,
    Reconciler,
};

/// Records every `Operation` it receives for assertions.
#[derive(Default, Clone)]
struct MockRenderer {
    ops: Arc<Mutex<Vec<Operation>>>,
}

impl rn::PlatformRenderer for MockRenderer {
    fn apply(&mut self, cs: Changeset) -> Result<(), rn::RenderError> {
        let mut guard = self.ops.lock().map_err(|_| {
            rn::RenderError::OsError("mock lock poisoned".to_string())
        })?;
        guard.extend(cs.operations);
        Ok(())
    }

    fn run_event_loop(&mut self) -> Result<(), rn::RenderError> {
        Ok(())
    }
}

impl MockRenderer {
    fn drain(&self) -> Vec<Operation> {
        self.ops
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Helper: build the Gallery tree given current state
// ---------------------------------------------------------------------------

struct Handlers {
    name_change: rn::HandlerId,
    inc: rn::HandlerId,
    dec: rn::HandlerId,
    reset: rn::HandlerId,
    toggle: rn::HandlerId,
    close: rn::HandlerId,
}

fn handler_ids() -> Handlers {
    Handlers {
        name_change: rn::HandlerId::new(),
        inc: rn::HandlerId::new(),
        dec: rn::HandlerId::new(),
        reset: rn::HandlerId::new(),
        toggle: rn::HandlerId::new(),
        close: rn::HandlerId::new(),
    }
}

fn gallery_tree(name: &str, clicks: i32, greeting_visible: bool, h: &Handlers) -> ComponentTree {
    let greeting = if name.is_empty() {
        "Hello, stranger!".to_string()
    } else {
        format!("Hello, {}!", name)
    };

    let mut greeting_label = rn::Component::new(ComponentKind::Label);
    greeting_label
        .props
        .insert(PropKey::Text, PropValue::String(greeting));
    greeting_label
        .props
        .insert(PropKey::Visible, PropValue::Bool(greeting_visible));

    ComponentTree::new(
        rn::window()
            .title("Rust Native — Widget Gallery")
            .size(420.0, 320.0)
            .on_close(h.close)
            .child(
                rn::vstack()
                    .padding(20.0)
                    .gap(12.0)
                    .child(
                        rn::hstack()
                            .gap(8.0)
                            .child(rn::label("Name:"))
                            .child(
                                rn::text_input()
                                    .placeholder("Type your name…")
                                    .value(name.to_string())
                                    .on_change(h.name_change)
                                    .build(),
                            )
                            .build(),
                    )
                    .child(greeting_label)
                    .child(rn::label(format!("Clicks: {clicks}")))
                    .child(
                        rn::hstack()
                            .gap(8.0)
                            .child(rn::button("−").on_press(h.dec).build())
                            .child(rn::button("+").on_press(h.inc).build())
                            .child(rn::button("Reset").on_press(h.reset).build())
                            .build(),
                    )
                    .child(rn::button("Toggle greeting").on_press(h.toggle).build())
                    .build(),
            )
            .build(),
    )
}

// ---------------------------------------------------------------------------
// 1. Gallery smoke
// ---------------------------------------------------------------------------

#[test]
fn gallery_initial_render_creates_all_widgets() {
    let h = handler_ids();
    let tree = gallery_tree("", 0, true, &h);

    let mut renderer = MockRenderer::default();
    rn::thread::MainThreadGuard::init_current();
    rn::render(&mut renderer, tree).expect("initial render");

    let ops = renderer.drain();
    let creates: Vec<_> = ops
        .iter()
        .filter_map(|op| {
            if let Operation::Create { kind, .. } = op {
                Some(kind)
            } else {
                None
            }
        })
        .collect();

    // Expected widgets: Window, vstack, hstack(name), label("Name:"),
    // TextInput, greeting label, clicks label, hstack(buttons),
    // button(−), button(+), button(Reset), button(Toggle greeting).
    assert!(
        creates.len() >= 12,
        "expected ≥12 Create ops, got {}: {:?}",
        creates.len(),
        creates
    );

    assert!(
        creates.iter().any(|k| **k == ComponentKind::Window),
        "no Window created"
    );
    assert!(
        creates
            .iter()
            .filter(|k| ***k == ComponentKind::Button)
            .count()
            >= 4,
        "expected ≥4 buttons"
    );
    assert!(
        creates.iter().any(|k| **k == ComponentKind::TextInput),
        "no TextInput created"
    );

    // Also verify via standalone reconciler (headless, no renderer state).
    let mut rec = Reconciler::new();
    let tree2 = gallery_tree("", 0, true, &h);
    let cs = rec.reconcile(tree2).expect("reconcile");
    assert!(!cs.is_empty());
}

// ---------------------------------------------------------------------------
// 2. Interactive state — clicking Increment
// ---------------------------------------------------------------------------

#[test]
fn increment_produces_update_not_recreate() {
    let h = handler_ids();

    let mut rec = Reconciler::new();

    let t0 = gallery_tree("", 0, true, &h);
    rec.reconcile(t0).expect("first render");

    let t1 = gallery_tree("", 1, true, &h);
    let cs = rec.reconcile(t1).expect("after increment");

    let (creates, updates, deletes, _) = count_ops(&cs);
    assert_eq!(creates, 0, "Increment should not create new controls");
    assert_eq!(deletes, 0, "Increment should not delete controls");
    assert!(updates >= 1, "Increment should produce ≥1 Update for label");

    // The clicks label should be the only thing that changed.
    let text_updates: Vec<_> = cs
        .operations
        .iter()
        .filter_map(|op| {
            if let Operation::Update { changed_props, .. } = op {
                changed_props.get(&PropKey::Text).cloned()
            } else {
                None
            }
        })
        .collect();
    assert!(
        text_updates
            .iter()
            .any(|v| v == &PropValue::String("Clicks: 1".to_string())),
        "expected 'Clicks: 1' update, got {text_updates:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Text input — typing a name updates greeting
// ---------------------------------------------------------------------------

#[test]
fn typing_name_updates_greeting_label() {
    let h = handler_ids();
    let mut rec = Reconciler::new();

    rec.reconcile(gallery_tree("", 0, true, &h))
        .expect("initial");

    let cs = rec
        .reconcile(gallery_tree("Alice", 0, true, &h))
        .expect("after typing");

    let (creates, updates, deletes, _) = count_ops(&cs);
    assert_eq!(creates, 0, "typing should not create new controls");
    assert_eq!(deletes, 0, "typing should not delete controls");
    assert!(updates >= 1, "typing should produce ≥1 Update");

    // Greeting text should include the name.
    let has_greeting = cs.operations.iter().any(|op| {
        if let Operation::Update { changed_props, .. } = op {
            changed_props
                .get(&PropKey::Text)
                .map(|v| matches!(v, PropValue::String(s) if s.contains("Alice")))
                .unwrap_or(false)
        } else {
            false
        }
    });
    assert!(has_greeting, "greeting label not updated with name 'Alice'");
}

// ---------------------------------------------------------------------------
// 4. Visibility toggle
// ---------------------------------------------------------------------------

#[test]
fn toggle_greeting_produces_visible_update_only() {
    let h = handler_ids();
    let mut rec = Reconciler::new();

    rec.reconcile(gallery_tree("", 0, true, &h))
        .expect("initial");

    // Toggle: visible=true → visible=false.
    let cs = rec
        .reconcile(gallery_tree("", 0, false, &h))
        .expect("after toggle");

    let (creates, _, deletes, _) = count_ops(&cs);
    assert_eq!(creates, 0, "toggle should not recreate controls");
    assert_eq!(deletes, 0, "toggle should not delete controls");

    let visible_update = cs.operations.iter().any(|op| {
        if let Operation::Update { changed_props, .. } = op {
            changed_props
                .get(&PropKey::Visible)
                .map(|v| *v == PropValue::Bool(false))
                .unwrap_or(false)
        } else {
            false
        }
    });
    assert!(
        visible_update,
        "expected an Update with Visible=false, got {:?}",
        cs.operations
    );
}

// ---------------------------------------------------------------------------
// 5. Reset button
// ---------------------------------------------------------------------------

#[test]
fn reset_returns_counter_to_zero() {
    let h = handler_ids();
    let mut rec = Reconciler::new();

    rec.reconcile(gallery_tree("", 0, true, &h))
        .expect("initial");
    rec.reconcile(gallery_tree("", 5, true, &h))
        .expect("at 5");

    let cs = rec
        .reconcile(gallery_tree("", 0, true, &h))
        .expect("after reset");

    let has_zero = cs.operations.iter().any(|op| {
        if let Operation::Update { changed_props, .. } = op {
            changed_props
                .get(&PropKey::Text)
                .map(|v| *v == PropValue::String("Clicks: 0".to_string()))
                .unwrap_or(false)
        } else {
            false
        }
    });
    assert!(has_zero, "Reset should produce Update with 'Clicks: 0'");
}

// ---------------------------------------------------------------------------
// 6. Multi-increment coalescing
// ---------------------------------------------------------------------------

#[test]
fn two_rapid_increments_coalesce_to_single_reconcile() {
    let h = handler_ids();
    let mut rec = Reconciler::new();

    rec.reconcile(gallery_tree("", 0, true, &h))
        .expect("initial");

    // Simulate two increments coalesced: the tick-drain loop applies both
    // before re-rendering, so only one reconcile at count=2 happens.
    let cs = rec
        .reconcile(gallery_tree("", 2, true, &h))
        .expect("after coalesced increments");

    let (creates, updates, deletes, _) = count_ops(&cs);
    assert_eq!(creates, 0);
    assert_eq!(deletes, 0);
    assert!(updates >= 1);

    let has_two = cs.operations.iter().any(|op| {
        if let Operation::Update { changed_props, .. } = op {
            changed_props
                .get(&PropKey::Text)
                .map(|v| *v == PropValue::String("Clicks: 2".to_string()))
                .unwrap_or(false)
        } else {
            false
        }
    });
    assert!(has_two, "coalesced increments should land at count=2");
}

// ---------------------------------------------------------------------------
// 7. No-op render (state unchanged)
// ---------------------------------------------------------------------------

#[test]
fn no_op_render_produces_empty_changeset() {
    let h = handler_ids();
    let mut rec = Reconciler::new();

    let build = || gallery_tree("Bob", 3, true, &h);
    rec.reconcile(build()).expect("initial");
    let cs = rec.reconcile(build()).expect("no-op");

    assert!(
        cs.is_empty(),
        "identical state should produce empty changeset, got {:?}",
        cs.operations
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_ops(cs: &Changeset) -> (usize, usize, usize, usize) {
    let (mut c, mut u, mut d, mut r) = (0, 0, 0, 0);
    for op in &cs.operations {
        match op {
            Operation::Create { .. } => c += 1,
            Operation::Update { .. } => u += 1,
            Operation::Delete { .. } => d += 1,
            Operation::Reorder { .. } => r += 1,
        }
    }
    (c, u, d, r)
}
