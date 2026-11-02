# Research: Core Framework (001-core-framework)

## Layout Engine

**Decision**: Use `taffy` (pure Rust, no platform deps, Flexbox + CSS Grid)

**Rationale**: Layout computation must be consistent across all three platforms —
delegating to each platform's native layout (Auto Layout on macOS, DPI-aware layout
on Windows, GTK's constraint layout on Linux) produces different results for the same
component tree. `taffy` runs before the renderer and produces absolute pixel rects
that the platform renderer applies to native controls. This is the same approach used
by Dioxus and Bevy UI. `taffy` has zero unsafe code and no platform dependencies.

**Alternatives considered**:
- Native per-platform layout: rejected — inconsistent results, hard to test headlessly.
- `stretch` (predecessor to taffy): deprecated, superseded by taffy.
- Custom layout engine: rejected per Principle V (YAGNI).

---

## Reconciler Algorithm

**Decision**: Key-based tree diff, O(n) with stable position heuristics

**Rationale**: React's original O(n³) tree diff is too slow; React's heuristic O(n)
algorithm works by assuming nodes at the same tree position with the same type are
the same node, and using explicit keys for dynamic lists. The same heuristic applies
here. For a 500-node tree this produces well under 1ms diff time, well within the
8ms budget.

**Algorithm**:
1. Walk old and new trees simultaneously in depth-first order.
2. At each position: if type matches → emit `Update(changed props only)`.
3. If type differs → emit `Delete(old)` + `Create(new)`.
4. If a key is present and matches a key elsewhere in the same parent → emit
   `Move(node, new_position)` before `Update`.
5. Emit nothing for nodes with identical props.

**Alternatives considered**:
- Myers diff adapted to trees: more accurate for reordering but O(n²) in worst case.
- Full structural hash comparison: O(n) but high constant factor; heuristic is faster
  for typical UI trees.

---

## Component Identity / Keys

**Decision**: Position-based by default; explicit string keys for dynamic lists

**Rationale**: Most UI trees are static structure — the same widget types at the same
positions every render. Position-based identity is zero-overhead for this case. Dynamic
lists (e.g., a list of items from state) require explicit keys to prevent the reconciler
from treating a reorder as a delete + create sequence.

**API**: `Component::keyed("my-key", component)` wraps any component with an explicit
key. No key = position-based identity.

---

## State Management Model

**Decision**: Consumer-owned state; framework is stateless between renders

**Rationale**: The framework does not own or observe application state. The consumer
builds a `ComponentTree` from their current state and calls `rn::render(tree)`. The
framework diffs the new tree against the previous one and updates native controls.
This is the simplest correct model — no hooks, no signals, no observables inside the
framework itself. Consumers can use whatever state management they choose.

**Alternatives considered**:
- Reactive signals (like Leptos/SolidJS): adds a runtime observable graph inside the
  framework. Powerful but complex; violates Principle V for v1.
- Hooks (React model): requires a fiber-style coroutine runtime. Out of scope for v1.

---

## macOS Backend Crate

**Decision**: `objc2` + `objc2-app-kit` over `cacao`

**Rationale**: `objc2` provides complete, safe Objective-C runtime bindings with
ARC-compatible memory management and is actively maintained by the `madsmtm` org.
`objc2-app-kit` covers all AppKit widgets we need. `cacao` is built on an older
`objc` crate, has incomplete coverage of AppKit, and its abstraction model conflicts
with our trait-based design. Direct `objc2` gives us full control over the native
control lifecycle.

**Alternatives considered**:
- `cacao`: higher level but incomplete and uses older `objc` bindings.
- Raw `objc` crate: older, unsafe-heavy, superseded by `objc2`.

---

## Windows Backend Crate

**Decision**: `windows` crate (Microsoft official) targeting Win32 + HWND controls

**Rationale**: WinUI 3 (via `windows-rs` WinRT APIs) is more modern but requires
MSIX packaging and has limited Rust binding maturity. Win32 HWND controls are
universally available on Windows 10+, well-documented, and the `windows` crate
provides safe-ish wrappers for all required APIs. Win32 controls also have
excellent accessibility support via MSAA/UIA out of the box.

**Alternatives considered**:
- WinUI 3 via WinRT: more modern look but complex deployment; deferred to v2.
- `native-windows-gui`: higher-level wrapper but adds a dependency layer we don't
  need given our own trait abstraction.

---

## Linux Backend Crate

**Decision**: `gtk4` crate (gtk-rs project)

**Rationale**: GTK 4 / libadwaita is the most widely available native widget toolkit
on Linux with strong accessibility support (AT-SPI2). The `gtk4` crate from gtk-rs
provides safe Rust bindings with GObject integration. GTK4 is available on all major
Linux distributions as of 2024.

**Alternatives considered**:
- Qt via `qttypes`/`ritual`: complex build system integration, less idiomatic Rust.
- `iced` as Linux backend: defeats the purpose (custom rendering, not native).

---

## Event Routing

**Decision**: Platform renderer emits events via `crossbeam-channel`; framework
routes to component-registered handlers by `NodeId`

**Rationale**: `crossbeam-channel` is the de facto standard for multi-producer
multi-consumer channels in Rust, works across threads, and has near-zero overhead.
Platform event loops run on the main thread and emit into the channel; the
application's render loop drains the channel, invokes handlers, rebuilds the tree,
and submits a new render. This keeps the event loop single-threaded while allowing
background work.

**Alternatives considered**:
- `std::sync::mpsc`: single-producer limitation is too restrictive for multi-source
  platform events.
- Callback registration directly on native controls: couples consumer code to platform
  types, violating Principle III.
