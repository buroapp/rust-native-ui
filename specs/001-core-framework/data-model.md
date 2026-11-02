# Data Model: Core Framework (001-core-framework)

## Core Types

### `NodeId`

```
NodeId
  value: u64          # monotonically incrementing, assigned by reconciler
```

Stable identity assigned to each node when first created. Used by the reconciler
to correlate virtual nodes across renders and by the platform renderer to look up
native control handles. Never exposed to consumer code.

---

### `Component`

```
Component
  kind:     ComponentKind      # discriminates the widget type
  props:    Props               # flattened property bag (key-value)
  key:      Option<String>      # explicit reconciler key (dynamic lists)
  children: Vec<Component>      # owned child components
```

Pure value type. Constructed by consumer code. No OS calls, no allocation beyond
the Vec.

**Validation rules**:
- `Window` kind MUST NOT appear as a child of another component.
- `LayoutContainer` MUST have at least one child.
- `key` uniqueness is scoped to siblings within the same parent.

---

### `ComponentKind`

```
ComponentKind (enum)
  Window
  Label
  Button
  TextInput
  LayoutContainer(Axis)   # Axis: Horizontal | Vertical
```

Extensible in future versions. v1 supports exactly these five kinds per FR-009.

---

### `Props`

```
Props
  entries: HashMap<PropKey, PropValue>
```

```
PropKey (enum)
  Text
  Enabled
  Visible
  Width
  Height
  MinWidth
  MinHeight
  Padding
  Gap
  OnPress       # Button handler id
  OnChange      # TextInput handler id
  OnClose       # Window handler id
```

```
PropValue (enum)
  String(String)
  Bool(bool)
  F32(f32)
  HandlerId(HandlerId)
```

---

### `ComponentTree`

```
ComponentTree
  root:     Component       # single root (Window)
  version:  u64             # monotonically incrementing render counter
```

Immutable snapshot. Produced by consumer code each render cycle.
Owned by the framework between renders for diffing.

---

### `Changeset`

```
Changeset
  operations: Vec<Operation>
```

```
Operation (enum)
  Create { parent_id: Option<NodeId>, node_id: NodeId, kind: ComponentKind, props: Props }
  Update { node_id: NodeId, changed_props: Props }
  Delete { node_id: NodeId }
  Reorder { parent_id: NodeId, child_order: Vec<NodeId> }
```

Produced by the `Reconciler`. Consumed by a `PlatformRenderer`.
Zero platform types — fully serialisable and testable in isolation.

---

```
RenderError (enum)
  OsError(String)
  InvalidNode(NodeId)
  WrongThread                          # FR-012: render() called off-main
```

```
ReconcileError (enum)
  DuplicateKey(String)
  OrphanedNode(NodeId)
```

```
RnError (enum)
  Render(RenderError)
  Reconcile(ReconcileError)
  WrongThread                          # FR-012: top-level mirror for ergonomics
```

### `Reconciler`

```
Reconciler
  previous:  Option<ComponentTree>   # owned; replaced each render
  node_map:  HashMap<NodeId, ...>    # internal: tracks allocated NodeIds
```

**Methods**:
```
reconcile(next: ComponentTree) -> Result<Changeset, ReconcileError>
```

Stateless from the consumer's perspective: call `reconcile` with the new tree,
receive a `Changeset`. The `Reconciler` owns the previous tree internally.

---

### `PlatformRenderer` (trait)

```
trait PlatformRenderer {
  fn apply(&mut self, changeset: Changeset) -> Result<(), RenderError>;
  fn run_event_loop(&mut self) -> Result<(), RenderError>;
}
```

Implemented once per platform backend (macOS / Windows / Linux).
Owns all native control handles internally; never exposes them.

---

### `Event`

```
Event
  source:  NodeId          # which node emitted it
  kind:    EventKind
  payload: EventPayload
```

```
EventKind (enum)
  Press
  Change
  Close
  Focus
  Blur
  KeyDown
  KeyUp
```

```
EventPayload (enum)
  None
  Text(String)
  Key(KeyCode)
```

Platform-agnostic. Emitted by the platform renderer into the event channel.

---

### `HandlerId`

```
HandlerId
  value: u64
```

Opaque identifier registered by the consumer via `rn::on(handler_id, callback)`.
Stored in Props. The platform renderer emits `Event { source, kind }` and the
framework dispatches to the registered callback. Consumer never receives a native
event type.

---

## State Transitions

```
Consumer calls rn::render(tree)
  │
  ├─ First render:  Reconciler produces full Create changeset
  │                 PlatformRenderer creates all native controls
  │
  └─ Subsequent:    Reconciler diffs previous vs next tree
                    PlatformRenderer applies minimal Changeset
                    Previous tree replaced with next tree
```

```
Native event fires on platform
  │
  └─ PlatformRenderer emits Event into crossbeam channel
     Framework drains channel
     Dispatches to HandlerId callback
     Consumer updates state → calls rn::render(new_tree) → cycle repeats
```

---

## Workspace Layout

```
rn/                          # crate: public API (platform-agnostic)
├── src/
│   ├── lib.rs
│   ├── component.rs         # Component, ComponentKind, Props, ComponentTree
│   ├── reconciler.rs        # Reconciler, Changeset, Operation
│   ├── event.rs             # Event, EventKind, EventPayload, HandlerId
│   └── renderer.rs          # PlatformRenderer trait
│
rn-macos/                    # crate: macOS backend (feature="macos")
├── src/
│   ├── lib.rs
│   └── renderer.rs          # MacosRenderer: PlatformRenderer impl
│
rn-windows/                  # crate: Windows backend (feature="windows")
├── src/
│   ├── lib.rs
│   └── renderer.rs          # WindowsRenderer: PlatformRenderer impl
│
rn-linux/                    # crate: Linux backend (feature="linux")
├── src/
│   ├── lib.rs
│   └── renderer.rs          # LinuxRenderer: PlatformRenderer impl
│
rn-layout/                   # crate: taffy-backed layout engine (shared)
├── src/
│   └── lib.rs               # compute_layout(tree) -> LayoutResult
│
examples/
├── hello-world/             # minimal 3-widget demo (Window+Label+Button)
└── counter/                 # state change + reconciler demo
│
tests/
└── reconciler/              # headless unit tests (no OS required)
```
