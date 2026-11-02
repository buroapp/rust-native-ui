//! Rust Native (`rn`) — declarative cross-platform native UI framework.
//!
//! Consumers describe UI as a pure `ComponentTree`. The framework reconciles
//! consecutive trees into a minimal `Changeset` and dispatches to a per-platform
//! renderer that drives native OS controls.
//!
//! See `specs/001-core-framework/` in the repository for the full specification,
//! plan, and public-API contract.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![deny(unreachable_pub)]

pub mod component;
pub mod error;
pub mod event;
pub mod reconciler;
pub mod renderer;
pub mod thread;

use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub use component::{Axis, Component, ComponentBuilder, ComponentKind, ComponentTree, PropKey, PropValue, Props};
pub use error::{ReconcileError, RenderError, RnError};
pub use event::{Event, EventKind, EventPayload, HandlerId, KeyCode};
pub use reconciler::{Changeset, Operation, Reconciler};
pub use renderer::PlatformRenderer;
pub use rn_core::NodeId;

// ---------------------------------------------------------------------------
// Builder entry points (contracts/public-api.md)
// ---------------------------------------------------------------------------

/// Start building a `Window` component.
pub fn window() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Window)
}

/// Build a `Label` component with the given text.
pub fn label(text: impl Into<String>) -> Component {
    ComponentBuilder::new(ComponentKind::Label)
        .prop(PropKey::Text, PropValue::String(text.into()))
        .build()
}

/// Start building a `Button` component with the given label text.
pub fn button(label: impl Into<String>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Button).prop(PropKey::Text, PropValue::String(label.into()))
}

/// Start building a `TextInput` component.
pub fn text_input() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::TextInput)
}

/// Start building a vertical layout container.
pub fn vstack() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::LayoutContainer(Axis::Vertical))
}

/// Start building a horizontal layout container.
pub fn hstack() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::LayoutContainer(Axis::Horizontal))
}

/// Wrap a component with an explicit reconciler key (for dynamic lists).
pub fn keyed(key: impl Into<String>, mut component: Component) -> Component {
    component.key = Some(key.into());
    component
}

// ---------------------------------------------------------------------------
// Handler registry (FR-013 contract)
// ---------------------------------------------------------------------------

type HandlerFn = Box<dyn Fn(Event) + Send + Sync + 'static>;

fn handlers() -> &'static Mutex<HashMap<HandlerId, HandlerFn>> {
    static HANDLERS: OnceLock<Mutex<HashMap<HandlerId, HandlerFn>>> = OnceLock::new();
    HANDLERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a callback for a `HandlerId`. Replaces any previously registered
/// handler for the same id. Thread-safe.
pub fn on(id: HandlerId, handler: impl Fn(Event) + Send + Sync + 'static) {
    let Ok(mut map) = handlers().lock() else {
        log::error!("rn::on: handler registry mutex poisoned; dropping registration");
        return;
    };
    map.insert(id, Box::new(handler));
}

/// Dispatch an event to its registered handler, panic-safe (FR-013).
///
/// A panic inside the consumer handler is caught, logged at `error` level, and
/// swallowed — the event loop continues. Framework state is never mutated as
/// a result of a handler panic.
///
/// Returns `true` if a handler was registered and ran (whether or not it
/// panicked); `false` if no handler was registered for the event's `handler`
/// id.
pub fn dispatch_handler(event: Event) -> bool {
    let Ok(map) = handlers().lock() else {
        log::error!("rn::dispatch_handler: handler registry mutex poisoned");
        return false;
    };
    let Some(handler) = map.get(&event.handler) else {
        log::trace!(
            "rn::dispatch_handler: no handler for {:?} on {:?}",
            event.kind,
            event.handler
        );
        return false;
    };
    let result = catch_unwind(AssertUnwindSafe(|| handler(event.clone())));
    if let Err(payload) = result {
        log::error!("handler {:?} panicked: {}", event.handler, panic_payload_str(&payload));
    }
    true
}

fn panic_payload_str(payload: &Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

// ---------------------------------------------------------------------------
// Render entry points (FR-011, FR-012, FR-014)
// ---------------------------------------------------------------------------

fn reconciler_cell() -> &'static Mutex<Reconciler> {
    static RECONCILER: OnceLock<Mutex<Reconciler>> = OnceLock::new();
    RECONCILER.get_or_init(|| Mutex::new(Reconciler::new()))
}

/// Reconcile `tree` against the previously rendered tree and apply the resulting
/// `Changeset` to `renderer`.
///
/// Must be called from the platform main thread (FR-011). Non-main callers get
/// `Err(RnError::WrongThread)` — no panic (FR-012).
pub fn render<R: PlatformRenderer>(renderer: &mut R, tree: ComponentTree) -> Result<(), RnError> {
    thread::MainThreadGuard::check()?;

    let start = Instant::now();
    let changeset = {
        let Ok(mut rec) = reconciler_cell().lock() else {
            return Err(RnError::Render(RenderError::OsError(
                "reconciler mutex poisoned".to_string(),
            )));
        };
        rec.reconcile(tree).map_err(RnError::Reconcile)?
    };
    let reconcile_time = start.elapsed();

    log::trace!(
        "rn::render: reconcile took {:?}, {} ops",
        reconcile_time,
        changeset.operations.len()
    );
    log::debug!("rn::render: {}", changeset.summary());

    renderer.apply(changeset).map_err(RnError::Render)?;
    Ok(())
}

/// Start the platform event loop. Must be called from the main thread.
pub fn run<R: PlatformRenderer>(renderer: &mut R) -> Result<(), RnError> {
    thread::MainThreadGuard::check()?;
    renderer.run_event_loop().map_err(RnError::Render)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Cross-thread update channel (FR-011)
// ---------------------------------------------------------------------------

/// A type-erased update message. Consumers wrap their state-update values via
/// `UpdateMessage::new(value)` and recover them via `downcast` on the receiving
/// side.
pub struct UpdateMessage(Box<dyn Any + Send>);

impl UpdateMessage {
    /// Wrap a value for cross-thread posting.
    pub fn new<T: Send + 'static>(value: T) -> Self {
        UpdateMessage(Box::new(value))
    }

    /// Attempt to recover the inner value.
    pub fn downcast<T: Send + 'static>(self) -> Result<T, Self> {
        match self.0.downcast::<T>() {
            Ok(boxed) => Ok(*boxed),
            Err(any) => Err(UpdateMessage(any)),
        }
    }
}

type UpdateChannel = (
    crossbeam_channel::Sender<UpdateMessage>,
    crossbeam_channel::Receiver<UpdateMessage>,
);

fn update_channel() -> &'static UpdateChannel {
    static CHANNEL: OnceLock<UpdateChannel> = OnceLock::new();
    CHANNEL.get_or_init(crossbeam_channel::unbounded)
}

/// Returns a sender keyed to the main-thread message queue. Background threads
/// clone this and post `UpdateMessage::new(value)` to request a re-render.
pub fn update_sender() -> crossbeam_channel::Sender<UpdateMessage> {
    update_channel().0.clone()
}

/// Framework-internal: drain the update channel. Called by backends from the
/// main event loop each tick to collect pending state updates and coalesce
/// multiple posts into a single render (FR-011, analyze G1).
#[doc(hidden)]
pub fn drain_updates() -> Vec<UpdateMessage> {
    let (_, rx) = update_channel();
    rx.try_iter().collect()
}
