# Architecture

Rust Native is a declarative UI framework: applications describe a `ComponentTree`,
and the framework turns successive trees into minimal operations that drive real
OS controls.

Open [`architecture.html`](architecture.html) for the interactive version of this
document's diagram (pan, zoom, dark/light, click a component for its role).

## Crate map

| Crate | Lines | Responsibility | Depends on |
|---|---:|---|---|
| `rn-core` | 1128 | The shared vocabulary: `ComponentKind`, `PropKey`, `PropValue`, `Event`, `EventPayload`, `HandlerId`, the `PlatformRenderer` trait, design tokens. | nothing platform-specific |
| `rn-layout` | 206 | taffy-backed flexbox. `LayoutSpec` in, `LayoutResult` (a rect per node) out. | `taffy` |
| `rn` | 1548 | The public API: 72 builders, `ComponentTree`, the reconciler, handler registry, main-thread guard, update channel, `FlatList` engine. | `rn-core`, `rn-layout` |
| `rn-macos` | 3360 | Real AppKit renderer: `NSView` construction, the native handle store, geometry application, event trampolines. | `rn-core`, `objc2`, `objc2-app-kit` |
| `rn-windows` | 31 | Scaffold. Compiles, renders nothing. | — |
| `rn-linux` | 173 | Scaffold. Compiles, renders nothing. | — |

The dependency direction never points back at a backend. `rn` and `rn-core` contain
no AppKit, Win32 or GTK types; an application depending on `rn` gets `objc2` only
transitively, through the backend crate it selects with a cargo feature.

## The render pipeline

```
ComponentTree ──reconcile──► Operation[{Create,Update,Delete,Reorder}] ──► PlatformRenderer
      ▲                                                                          │
      │                                                                     build/update
   rerender                                                                      │
      │                                                                     native views
      └──────── handlers ◄── Event ◄── AppKit target/action ◄─────────────────────┘
                                         │
                           rn-layout ────┴──► LayoutResult → apply_rects → setFrame
```

1. **Build.** Application code calls builders (`rn::window()`, `rn::vstack()`,
   `rn::button(..)`) to produce an immutable `ComponentTree`. Building is pure: no
   OS calls happen here, which is why the reconciler is testable without a display.
2. **Reconcile.** `rn::render` diffs the new tree against the previous one and
   emits only what changed. A label whose text changed produces one `Update` that
   mutates one `stringValue`. Reordering a keyed list produces `Reorder`, not
   delete-and-recreate, so native selection and focus survive.
3. **Layout.** `rn-layout` maps each node's props to a taffy style and computes a
   rect per node. It is the single source of geometry.
4. **Render.** The backend creates the corresponding native view for each node and
   stores it in a handle map keyed by `NodeId`.
5. **Apply geometry.** After layout, every native view gets its frame from the
   `LayoutResult`, converted from the tree's top-left origin to AppKit's
   bottom-left origin.
6. **Events.** A native control fires its target/action; the trampoline turns it
   into an `Event`, dispatches it to the registered handler, and the handler sends
   a message through the update channel. On the next tick the application rebuilds
   and calls `rerender`.

## Layout authority

This is the most important invariant in the codebase, and the one that has caused
the most bugs.

**taffy owns geometry. Native containers must not arrange their children.**

Every defect traced back to AppKit and taffy both believing they owned layout:

- `NSSplitView` arranges its own subviews. Panes got one rect from taffy and
  another from AppKit, so children were positioned against a frame that never
  matched, and every pane collapsed. SplitView is now a plain container; sharing
  space is expressed as a layout rule (`flex_grow`), not as a second layout engine.
- Scroll views were initially given children directly. Subviews of `NSScrollView`
  are **not** part of the scrolled document, so they were clipped away and every
  scrollable pane rendered empty. Children now attach to `documentView`.

Two further rules follow from the coordinate systems:

- **Scrolled content lives in a flipped document view.** AppKit is bottom-left
  origin by default, so a tall document opens scrolled to its *bottom* — which is
  why lists started at their last item. Flipping the document fixes that and
  removes the need to invert y for anything inside a scroll view.
- **The root node fills the window**, otherwise the tree lays out at its intrinsic
  size inside a much larger window.

`ContentArea` is a scroll container for the same reason: it is a window's main
surface, so content taller than the pane has to stay reachable.

One deliberate exception: **`TabView` pages are not flex children.** `NSTabView`
sizes the page it shows, so handing pages to taffy would compute a second,
competing rect for each one. Build spec emits no children for a `TabView`, pages
resize with the tab view, and `apply_rects` never touches them.

**Selection is tree state, not scroll position.** A row selected programmatically
may sit far outside the visible part of a long list, so the renderer scrolls the
selection into view — only when the selection actually changes, so an unrelated
relayout cannot yank the user's scroll position back.

## Native controls, not imitations

A component must be the AppKit object it claims to be. The alternative — painting
a convincing imitation — is worse than not having the component, because the
imitation silently lacks system behaviour, accessibility and state.

This has driven three rewrites:

- `AlertDialog` was two grey labels in a box. It is now a real `NSAlert` presented
  as a sheet: system text, real buttons, response routed back to the handler.
- `Sheet` was a grey box in the parent view. It is now a real `NSWindow` presented
  with `beginSheet`, and because it has its own window its container is that
  window's content view.
- `SidebarItem` rows were unnamed in the accessibility tree, because the row text
  lives in a sibling label that VoiceOver will not associate with the button. The
  overlay button now carries the text as its accessibility label.

## Concurrency

- **Main thread only.** `MainThreadGuard` enforces it and returns
  `Err(RnError::WrongThread)` rather than panicking.
- **Handlers run wherever the event arrives**, so anything they touch must be
  `Send + Sync`. State is normally shared as `Arc<Mutex<State>>`.
- **Updates cross threads by channel.** `rn::update_sender()` produces a
  `crossbeam_channel::Sender<UpdateMessage>` that background workers can hold;
  `rn::set_tick` drains it on the main thread and calls `rerender`.
- **Dispatch is panic-safe.** A panicking handler does not unwind through AppKit's
  event loop.

## Component inventory

- ~57 `ComponentKind` variants, all documented and rendered in
  [`examples/storybook`](../examples/storybook).
- ~80 `PropKey` variants, 15 `PropValue` variants, 13 `EventPayload` variants.
- 72 public builders in `rn`.

Built-in layouts mirror the `macos_ui` set: `window` (with the modern
full-size-content look via `.modern(true)`), `scaffold`, `sidebar` +
`sidebar_item`, `toolbar`, `list_tile`, `tab_view` + `tab_item`, `content_area`,
`split_view` and `safe_area_view`. The sliver toolbar is not a separate
component: a `toolbar` placed as a child of a scroll view scrolls away with the
content, which is what the pattern means. A modern window renders content from
the window's top edge, so an app must inset its own content past the traffic
lights; the framework does not yet expose a titlebar inset.

The storybook is not a demo — it is the regression surface. `catalog.rs` contains a
test that fails when `rn-core` gains a component kind with no catalog entry, so the
inventory cannot silently drift.

## Where to change what

| Task | File |
|---|---|
| Add a component kind | `rn-core/src/lib.rs` (`ComponentKind`, `PropKey`), `rn/src/lib.rs` (builder), `rn-macos/src/renderer.rs` (construction + `build_spec` sizing), `examples/storybook/src/catalog.rs` (entry — a test fails without it) |
| Change layout behaviour | `rn-layout/src/lib.rs` (`LayoutSpec` → taffy), then `build_spec` in the renderer |
| Fix a rendering bug | `rn-macos/src/renderer.rs` — construction arms, `apply_rects`, `attach_to_parent` |
| Add an event | `rn-core` (`EventKind`/`EventPayload`), target trampolines in the renderer, `rn::on` wiring |
| Add a backend | Implement `PlatformRenderer` for the platform; `rn-windows` and `rn-linux` are the scaffolds |

## Testing

- `cargo test --workspace --features rn/macos` — reconciler, layout, storybook
  catalogue tests. No display required.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean is the bar.
- Visual verification is by running the examples; `RUST_LOG=debug` reports computed
  geometry, document-vs-viewport sizes and presented sheets, which is how the
  clipped-content and collapsed-sheet bugs were found.

## Known gaps

- Windows and Linux backends are scaffolds (31 and 173 lines).
- `Tooltip` is a static label, not a hover tooltip.
- `SplitView` dividers are not draggable.
- `SidebarItem` has no icon support.
- Some label and badge text truncates in the showcase.
