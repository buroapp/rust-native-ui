# Public API Contract: `rn` crate

This document describes the stable public surface of the `rn` crate that consumers
depend on. It is the contract that governs semver versioning — breaking changes here
require a MAJOR bump.

---

## Entry Points

### `rn::render`

```rust
pub fn render<R: PlatformRenderer>(
    renderer: &mut R,
    tree: ComponentTree,
) -> Result<(), RnError>
```

Reconciles `tree` against the previously rendered tree and applies the resulting
`Changeset` to `renderer`. On first call, creates all controls from scratch.

**Errors**: `RnError::Render(RenderError)` if the platform renderer fails to apply
the changeset.

---

### `rn::run`

```rust
pub fn run<R: PlatformRenderer>(
    renderer: &mut R,
) -> Result<(), RnError>
```

Starts the platform event loop. Blocks until the root window is closed.
Events are dispatched to handlers registered via `rn::on`.

---

### `rn::on`

```rust
pub fn on(
    id: HandlerId,
    handler: impl Fn(Event) + Send + 'static,
)
```

Registers a callback for `HandlerId`. Replaces any previously registered handler
for the same id. Thread-safe.

---

## Builder API (Component construction)

```rust
// Window
rn::window()                          -> ComponentBuilder
  .title(impl Into<String>)           -> ComponentBuilder
  .size(width: f32, height: f32)      -> ComponentBuilder
  .on_close(HandlerId)                -> ComponentBuilder
  .child(Component)                   -> ComponentBuilder
  .build()                            -> Component

// Label
rn::label(text: impl Into<String>)    -> Component

// Button
rn::button(label: impl Into<String>)  -> ComponentBuilder
  .on_press(HandlerId)                -> ComponentBuilder
  .enabled(bool)                      -> ComponentBuilder
  .build()                            -> Component

// TextInput
rn::text_input()                      -> ComponentBuilder
  .placeholder(impl Into<String>)     -> ComponentBuilder
  .value(impl Into<String>)           -> ComponentBuilder
  .on_change(HandlerId)               -> ComponentBuilder
  .build()                            -> Component

// Layout containers
rn::vstack()                          -> ComponentBuilder   // vertical
rn::hstack()                          -> ComponentBuilder   // horizontal
  .gap(f32)                           -> ComponentBuilder
  .padding(f32)                       -> ComponentBuilder
  .child(Component)                   -> ComponentBuilder   // repeatable
  .build()                            -> Component

// Keyed wrapper (explicit reconciler key)
rn::keyed(key: impl Into<String>, component: Component) -> Component
```

---

## `ComponentTree`

```rust
pub struct ComponentTree { /* opaque */ }

impl ComponentTree {
    pub fn new(root: Component) -> Self
}
```

---

## `HandlerId`

```rust
pub struct HandlerId(u64);

impl HandlerId {
    pub fn new() -> Self   // auto-incrementing, unique per process
}
```

---

## `Event`

```rust
pub struct Event {
    pub source: NodeId,
    pub kind: EventKind,
    pub payload: EventPayload,
}

pub enum EventKind { Press, Change, Close, Focus, Blur, KeyDown, KeyUp }

pub enum EventPayload { None, Text(String), Key(KeyCode) }
```

---

## `PlatformRenderer` trait

```rust
pub trait PlatformRenderer {
    fn apply(&mut self, changeset: Changeset) -> Result<(), RenderError>;
    fn run_event_loop(&mut self) -> Result<(), RenderError>;
}
```

Implemented in platform backend crates. Not required in consumer code — consumers
use `rn::render` and `rn::run` which accept `impl PlatformRenderer`.

---

## Error Types

```rust
pub enum RnError {
    Render(RenderError),
    Reconcile(ReconcileError),
    WrongThread,                 // FR-012: render() called off main thread
}

pub enum RenderError {
    OsError(String),
    InvalidNode(NodeId),
    WrongThread,
}

pub enum ReconcileError {
    DuplicateKey(String),
    OrphanedNode(NodeId),
}
```

---

## Threading Contract (FR-011, FR-012)

- `rn::render` and `rn::run` MUST be called from the platform main thread.
- Calling from any other thread returns `Err(RnError::WrongThread)` — no panic.
- Background threads post state updates by cloning a `crossbeam_channel::Sender`
  obtained via `rn::update_sender()` and sending a message wrapped in
  `UpdateMessage::new(value)`; the main event loop drains the receiver each tick
  and calls `rn::render` with the new tree.
- `HandlerId` registration (`rn::on`) is thread-safe (`Send + Sync` internally)
  so handlers may be registered before the event loop starts, from any thread.

```rust
// The type-erased message envelope (rn/src/lib.rs)
pub struct UpdateMessage(/* opaque */);

impl UpdateMessage {
    pub fn new<T: Send + 'static>(value: T) -> Self;
    pub fn downcast<T: Send + 'static>(self) -> Result<T, Self>;
}

// Returns a sender onto the single framework-wide update channel.
// Messages must be wrapped in `UpdateMessage::new(value)`.
pub fn update_sender() -> crossbeam_channel::Sender<UpdateMessage>
```

Usage pattern (counter example):

```rust
let tx = rn::update_sender();
rn::on(press_id, move |_| {
    let _ = tx.send(rn::UpdateMessage::new(Msg::Increment));
});

// In the set_tick closure:
for msg in rn::drain_updates() {
    if let Ok(Msg::Increment) = msg.downcast::<Msg>() {
        count += 1;
    }
}
```

Returns a sender keyed to the current main-thread message queue. The main event
loop drains the matching receiver each tick via `rn::drain_updates()`.

---

## Panic & Logging Contract (FR-013, FR-014)

- Every registered handler is dispatched inside `std::panic::catch_unwind`.
  A panic in a handler is logged via `log::error!` and the event loop continues.
- The framework uses the `log` facade crate. Consumers choose a logger
  implementation (e.g. `env_logger`, `simplelog`) and initialise it themselves.
- Framework log sites:
  - `trace!` — reconciler per-render timing and node counts
  - `debug!` — changeset summaries
  - `warn!` — recoverable native-call failures
  - `error!` — unrecoverable platform errors, handler panics

---

## Stability Guarantees

| Surface | Stability |
|---------|-----------|
| `rn::render`, `rn::run`, `rn::on`, `rn::update_sender` | Stable (semver protected) |
| Builder API (`rn::window()` etc.) | Stable |
| `PlatformRenderer` trait | Stable |
| `Event`, `EventKind`, `EventPayload` | Stable |
| `RnError`, `RenderError`, `ReconcileError` | Stable |
| Threading contract (main-thread-only) | Stable (FR-011/FR-012) |
| Panic & logging contract | Stable (FR-013/FR-014) |
| `NodeId` (opaque) | Stable (value not guaranteed) |
| `Changeset`, `Operation` | Internal — not part of public API |
| Platform backend crates (`rn-macos` etc.) | Internal — subject to change |
| `rn::backend` module | Internal — `#[doc(hidden)]`, subject to change |
