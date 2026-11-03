# Tasks: React Native Component Parity + Shadcn Design System

**Input**: Design documents from `/specs/002-react-native-parity/`

**Feature**: `002-react-native-parity` | **Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US9)
- All file paths are relative to the workspace root `~/Developer/rust-native-ui/`

---

## Phase 1: Setup

**Purpose**: Add new example binaries and Cargo workspace entries.

- [x] T001 Add `examples/settings-demo` to `Cargo.toml` workspace members in `Cargo.toml`
- [x] T002 Add `examples/mail-demo` to `Cargo.toml` workspace members in `Cargo.toml`
- [x] T003 [P] Create `examples/settings-demo/Cargo.toml` with `rn` dep + `macos` feature
- [x] T004 [P] Create `examples/mail-demo/Cargo.toml` with `rn` dep + `macos` feature
- [x] T005 [P] Create `examples/settings-demo/src/main.rs` stub (empty `fn main`)
- [x] T006 [P] Create `examples/mail-demo/src/main.rs` stub (empty `fn main`)

**Checkpoint**: `cargo build --workspace --features macos` compiles with zero errors.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend the core `rn` crate with all new `ComponentKind` variants, prop keys, prop values, event payloads, and the design system token types. Everything else depends on this.

**⚠️ CRITICAL**: No user story implementation can begin until this phase is complete.

- [x] T007 Add new `ComponentKind` variants to `rn/src/component.rs`: `ScrollView`, `FlatList`, `TouchableOpacity`, `SafeAreaView`, `ActivityIndicator`, `Modal`, `Popover`, `Text` (rich), `Card`, `Badge`, `Skeleton`, `SegmentedControl`, `SearchBar`, `Sidebar`, `SidebarItem`, `GroupedSection`, `SplitView`, `Toolbar`, `Switch`, `StatusBar`
- [x] T008 [P] Add new `PropKey` variants to `rn/src/component.rs`: `Direction`, `Scrollable`, `ContentOffset`, `OnScroll`, `ItemHeight`, `BufferRows`, `EndReachThreshold`, `OnEndReached`, `PressOpacity`, `StopBubbling`, `SafeAreaInsets`, `IndicatorSize`, `ModalVisible`, `ModalPosition`, `CloseOnBackdrop`, `AnchorId`, `SwitchValue`, `StatusBarVisible`, `TextSpans`, `TextWrap`, `TextAlign`, `MaxLines`, `Padding`, `Gap`, `BorderRadius`, `Shadow`, `Variant`, `SkeletonVariant`, `SelectedIndex`, `Options`, `SearchPlaceholder`, `SearchValue`, `SidebarWidth`, `Collapsed`, `IconName`, `SidebarBadge`, `SectionTitle`, `SplitOrientation`, `DividerWidth`, `MinPaneSizes`, `PaneSizes`, `ToolbarActions`, `StyleToken`
- [x] T009 [P] Add new `PropValue` variants to `rn/src/component.rs`: `ScrollDirection(ScrollDirection)`, `ItemHeight(ItemHeight)`, `ModalPosition(ModalPosition)`, `SplitOrientation(SplitOrientation)`, `TextSpans(Vec<StyledSpan>)`, `ToolbarActions(Vec<ToolbarAction>)`, `PaneSizes(Vec<f32>)`, `Style(Style)`, `Insets(Insets)`
- [x] T010 [P] Add new `EventPayload` variants to `rn/src/event.rs`: `Scroll(ScrollEvent)`, `Search(String)`, `Segment(i32)`, `SplitResize(Vec<f32>)`, `ToolbarAction(String)` — include `ScrollEvent` struct
- [x] T011 Add design system token types to `rn/src/style.rs` (new file): `ColorToken`, `SpacingToken`, `RadiusToken`, `FontToken`, `FocusRingToken`, `FocusRingStyle`, `ShadowStyle`, `Style` builder with all chainable methods, preset constructors (`primary()`, `secondary()`, `destructive()`, `ghost()`, `outline()`)
- [x] T012 [P] Add auxiliary types to `rn/src/component.rs` or `rn/src/types.rs`: `ScrollDirection`, `ItemHeight`, `ModalPosition`, `SplitOrientation`, `StyledSpan` (with builder), `TextAlign`, `ToolbarAction`, `BadgeVariant`, `BadgeSize`, `SkeletonVariant`, `IndicatorSize`, `Insets`, `Point`
- [x] T013 Re-export all new public types from `rn/src/lib.rs` and add stub builder functions for all new components (returning `ComponentBuilder::new(ComponentKind::X)`)

**Checkpoint**: `cargo build -p rn` compiles. `cargo test -p rn --tests` passes (existing 8 tests still green).

---

## Phase 3: User Story 7 — Shadcn Design System (Priority: P1)

**Goal**: Style tokens applied to native controls on all three platforms. `Card`, `Badge`, `Skeleton` components render natively.

**Independent Test**: `cargo run -p showcase --features macos -- --design-system` shows primary/secondary/destructive/ghost button variants, Card with shadow, Badge pills with color variants, Skeleton shimmer animation.

- [x] T014 [US7] Implement `Style` prop application in `rn-macos/src/renderer.rs`: map `ColorToken` → `NSColor`, `SpacingToken` → padding/margin, `RadiusToken` → `CALayer.cornerRadius`, `FontToken` → `NSFont`, `FocusRingToken` → focus ring appearance on all existing widget types
- [x] T015 [P] [US7] Implement `Card` component in `rn-macos/src/renderer.rs`: `NSBox` with `.primaryBox` appearance, `CALayer` shadow, corner radius from `RadiusToken`, background from `ColorToken::Card`
- [x] T016 [P] [US7] Implement `Badge` component in `rn-macos/src/renderer.rs`: `NSTextField` non-editable + `cell.backgroundColor` from variant token + `layer.cornerRadius` = `RadiusToken::Full`
- [x] T017 [P] [US7] Implement `Skeleton` component in `rn-macos/src/renderer.rs`: `NSView` + `CAGradientLayer` shimmer animation using `CABasicAnimation` on `locations` keypath
- [x] T018 [P] [US7] Implement `Card` stub in `rn-windows/src/renderer.rs` (layered child window with rounded corners via `DwmExtendFrameIntoClientArea` clip region)
- [x] T019 [P] [US7] Implement `Card` stub in `rn-linux/src/renderer.rs` (`GtkFrame` with CSS radius)
- [x] T020 [P] [US7] Add `card()`, `badge()`, `skeleton()` builder functions to `rn/src/lib.rs` with full method chains per `contracts/public-api.md`
- [x] T021 [US7] Add design system showcase section to `examples/showcase/src/main.rs`: four `Button` style variants, one `Card` with children, five `Badge` variants, two `Skeleton` rectangles

**Checkpoint**: `cargo run -p showcase --features macos -- --design-system` shows all design system components natively rendered. Clippy clean.

---

## Phase 4: User Story 1 — RN Mental Model (Priority: P1)

**Goal**: `ScrollView`, extended `Text`, `ActivityIndicator` render natively on macOS using the `rn` vocabulary.

**Independent Test**: `cargo run -p showcase --features macos` includes a scrollable container, multi-line text, and a spinner — all native AppKit controls.

- [x] T022 [US1] Implement `ScrollView` in `rn-macos/src/renderer.rs`: `NSScrollView` + `NSClipView` + content `NSView`, honour `direction` prop, wire `on_scroll` via `NSScrollView` notification → `EventPayload::Scroll`
- [x] T023 [P] [US1] Implement `ScrollView` stub in `rn-windows/src/renderer.rs`: `WS_VSCROLL`/`WS_HSCROLL` child window
- [x] T024 [P] [US1] Implement `ScrollView` stub in `rn-linux/src/renderer.rs`: `GtkScrolledWindow`
- [x] T025 [P] [US1] Implement `ActivityIndicator` in `rn-macos/src/renderer.rs`: `NSProgressIndicator` indeterminate spinning, respect `color` token, `size` variant (sm/md/lg → 16/24/32pt), `visible` prop toggles `isHidden`
- [x] T026 [P] [US1] Implement `ActivityIndicator` stubs in `rn-windows/src/renderer.rs` (PBS_MARQUEE) and `rn-linux/src/renderer.rs` (`GtkSpinner`)
- [x] T027 [P] [US1] Implement `Switch` in `rn-macos/src/renderer.rs`: `NSSwitch` (macOS 10.15+), wire `on_change` → `EventPayload::Bool`
- [x] T028 [P] [US1] Add `scroll_view()`, `activity_indicator()`, `switch()` builder functions to `rn/src/lib.rs` per `contracts/public-api.md`
- [x] T029 [US1] Add ScrollView + ActivityIndicator + Switch to `examples/showcase/src/main.rs` in a new "RN Components" section

**Checkpoint**: Showcase compiles and shows native ScrollView (scrollable), ActivityIndicator (spinning), Switch (toggle). `cargo test -p rn --tests` still green.

---

## Phase 5: User Story 2 — Virtualized FlatList (Priority: P1)

**Goal**: `FlatList` with 10,000 items scrolls at 60fps using ≤5MB memory (constant live component count).

**Independent Test**: `cargo run -p settings-demo --features macos -- --bench-flatlist` shows 10k-item list scrolling smoothly; console reports `live_nodes ≤ 25`.

- [x] T030 [US2] Implement `FlatList` virtualization engine in `rn/src/flat_list.rs`: viewport state machine (`visible_range`, `buffer_range`, `live_pool`, `height_cache`, `total_height`, `scroll_offset`); transitions for `OnScroll`, `OnDataAppend`, `OnDataUpdate`, `OnItemHeightChange`
- [x] T031 [US2] Implement `FlatList` in `rn-macos/src/renderer.rs`: `NSScrollView` host + `NSView` recycle pool; translate scroll events to `FlatList` state machine; mount/unmount items via reconciler; report `live_node_count` at `log::debug!` level
- [x] T032 [P] [US2] Implement `FlatList` stub in `rn-windows/src/renderer.rs`: HWND pool over `WS_VSCROLL` host
- [x] T033 [P] [US2] Implement `FlatList` stub in `rn-linux/src/renderer.rs`: `GtkListView` with `GtkListItemFactory`
- [x] T034 [P] [US2] Add `flat_list()` builder function to `rn/src/lib.rs` per `contracts/public-api.md`
- [x] T035 [US2] Add FlatList headless state machine tests to `tests/reconciler/tests.rs`: scroll → recycle → mount cycle, data append preserving scroll position, out-of-viewport update queuing
- [x] T036 [US2] Add 10k-item FlatList bench section to `examples/settings-demo/src/main.rs` with `--bench-flatlist` argument; print `live_nodes` and approximate memory to console each second

**Checkpoint**: `cargo test -p rn --tests` passes (new FlatList state tests green). `cargo run -p settings-demo --features macos -- --bench-flatlist` shows constant live_nodes ≤ 25.

---

## Phase 6: User Story 3 — Safe Area & Screen Insets (Priority: P2)

**Goal**: `SafeAreaView` correctly insets content below the macOS title bar on all platforms.

**Independent Test**: `cargo run -p showcase --features macos -- --safe-area` shows content starting below the traffic-light buttons row with console-printed inset values.

- [x] T037 [US3] Implement `SafeAreaView` in `rn-macos/src/renderer.rs`: query `NSWindow.contentLayoutGuide.layoutFrame` at render time, apply `top` inset to content `NSView.frame.origin.y`, expose computed `Insets` as read-only prop
- [x] T038 [P] [US3] Implement `SafeAreaView` stub in `rn-windows/src/renderer.rs`: `GetSystemMetrics(SM_CYCAPTION)` for top inset
- [x] T039 [P] [US3] Implement `SafeAreaView` stub in `rn-linux/src/renderer.rs`: query `GdkMonitor.workarea` vs window bounds
- [x] T040 [P] [US3] Add `safe_area_view()` builder to `rn/src/lib.rs`
- [x] T041 [US3] Add SafeAreaView section to `examples/showcase/src/main.rs` (`--safe-area` flag, prints insets to console)

**Checkpoint**: `cargo run -p showcase --features macos -- --safe-area` shows insets `top: ≥ 28.0` (title bar height). Content does not overlap traffic-light buttons.

---

## Phase 7: User Story 4 — Touchable & Pressable Interactions (Priority: P2)

**Goal**: `TouchableOpacity` wraps any child, dims on press, fires `on_press`, and supports event bubbling.

**Independent Test**: `cargo run -p showcase --features macos -- --event-bubbling` shows nested wrappers; inner handler fires before outer; console prints `handler: inner`, `handler: middle`, `handler: outer`.

- [x] T042 [US4] Implement `TouchableOpacity` in `rn-macos/src/renderer.rs`: transparent `NSView` overlay that handles `mouseDown`/`mouseUp`; on `mouseDown` sets `alphaValue = opacity`; on `mouseUp` restores alpha and fires handler; intercepts events before children via `hitTest` override
- [x] T043 [P] [US4] Implement `TouchableOpacity` stubs in `rn-windows/src/renderer.rs` (`WM_LBUTTONDOWN`/`WM_LBUTTONUP` + `UpdateLayeredWindow` alpha) and `rn-linux/src/renderer.rs` (`GtkEventBox` + `gtk_widget_set_opacity`)
- [x] T044 [US4] Implement event bubbling in `rn/src/event.rs`: add `stop_bubbling` flag to `Event`; update `rn-macos` renderer to walk parent handler chain after firing inner handler, stopping when `stop_bubbling = true`
- [x] T045 [P] [US4] Add `touchable_opacity()` builder to `rn/src/lib.rs`
- [x] T046 [US4] Add event bubbling test to `tests/reconciler/tests.rs`: nested touch targets; verify bubble order and `stop_bubbling` halts propagation
- [x] T047 [US4] Add event bubbling section to `examples/showcase/src/main.rs` (`--event-bubbling` flag)

**Checkpoint**: Nested touchables fire in inner→outer order. `stop_bubbling` halts chain. Opacity restores correctly on `mouseUp`.

---

## Phase 8: User Story 5 — Rich Text & Paragraph Layout (Priority: P2)

**Goal**: `Text` component renders styled spans (bold/italic/color/size) using `NSAttributedString` on macOS; text reflows on window resize.

**Independent Test**: `cargo run -p showcase --features macos` shows a `Text` with mixed bold/italic/red spans rendered as attributed string; resizing the window causes text to reflow.

- [x] T048 [US5] Implement rich `Text` component in `rn-macos/src/renderer.rs`: `NSTextView` (non-editable) with `NSAttributedString` built from `Vec<StyledSpan>`; map `ColorToken` → `NSColor`, `FontToken` → `NSFont`, bold/italic via `NSFontDescriptor` symbolic traits; `wrap` prop controls `NSTextView.textContainer.widthTracksTextView`; window resize reflows via `setNeedsLayout`
- [x] T049 [P] [US5] Implement rich `Text` stub in `rn-windows/src/renderer.rs`: `EDIT` or `RichEdit` control with `CHARFORMAT2` per-span styling
- [x] T050 [P] [US5] Implement rich `Text` stub in `rn-linux/src/renderer.rs`: `GtkLabel` with Pango markup from `StyledSpan` list
- [x] T051 [P] [US5] Add `text()` and `text_plain()` builder functions + `StyledSpan` builder to `rn/src/lib.rs` per `contracts/public-api.md`
- [x] T052 [US5] Add rich text section to `examples/showcase/src/main.rs`: three spans (normal, bold red, italic muted) in one `Text` component

**Checkpoint**: Showcase shows attributed text. Bold/red/italic spans render correctly via Accessibility Inspector (text attributes visible). Text reflows on window resize.

---

## Phase 9: User Story 6 — Modal & Popover (Priority: P2)

**Goal**: `Modal` shows as a native macOS sheet/panel above content; `Popover` anchors to a target view.

**Independent Test**: `cargo run -p showcase --features macos` includes a "Show Modal" button; pressing it opens a native modal sheet; clicking outside with `close_on_backdrop=true` dismisses it.

- [x] T053 [US6] Implement `Modal` in `rn-macos/src/renderer.rs`: `NSPanel` presented as floating window or `beginSheet:` depending on `position` prop; backdrop `NSView` overlay for `close_on_backdrop`; wire `on_dismiss`; manage `ModalState` (Hidden → Presenting → Visible → Dismissing) with proper native animations
- [x] T054 [P] [US6] Implement `Popover` in `rn-macos/src/renderer.rs`: `NSPopover` anchored to `anchor_id` node's `NSView`, `offset` applied via `positioningRect`, wire `on_dismiss` via `NSPopoverDelegate`
- [x] T055 [P] [US6] Implement `Modal` stub in `rn-windows/src/renderer.rs` (`CreateDialogParam` modal pattern) and `rn-linux/src/renderer.rs` (`GtkDialog` with `gtk_dialog_run`)
- [x] T056 [P] [US6] Implement `Popover` stub in `rn-windows/src/renderer.rs` (popup window) and `rn-linux/src/renderer.rs` (`GtkPopover`)
- [x] T057 [P] [US6] Add `modal()` and `popover()` builder functions to `rn/src/lib.rs`
- [x] T058 [US6] Add Modal state machine test to `tests/reconciler/tests.rs`: `Hidden → Presenting → Visible → Dismissing → Hidden` transitions
- [x] T059 [US6] Add Modal + Popover section to `examples/showcase/src/main.rs`: "Show Modal" button opens a modal with "Close" button; "Show Popover" button anchors a popover

**Checkpoint**: Modal opens as native panel/sheet, closes on backdrop click, fires `on_dismiss`. Popover anchors correctly. Modal state machine test passes.

---

## Phase 10: User Story 8 — macOS Settings App Layout (Priority: P2)

**Goal**: `Sidebar`, `SidebarItem`, `GroupedSection`, `SegmentedControl`, `SearchBar` render natively; `settings-demo` example replicates macOS Settings app structure.

**Independent Test**: `cargo run -p settings-demo --features macos` opens a window with native sidebar navigation, grouped settings panels, segmented control, switch rows, and a functional search bar.

- [x] T060 [US8] Implement `Sidebar` in `rn-macos/src/renderer.rs`: `NSSplitView` with leading pane styled as sidebar (`NSSidebarViewController` style: vibrancy background via `NSVisualEffectView`, `NSOutlineView` for items, appropriate row height 40pt)
- [x] T061 [US8] Implement `SidebarItem` in `rn-macos/src/renderer.rs`: `NSTableCellView` row with SF Symbol `NSImageView` + `NSTextField` label; `selected` prop → highlight via `NSTableView.selectRowIndexes`; wire `on_select` → `EventPayload::Index`
- [x] T062 [P] [US8] Implement `GroupedSection` in `rn-macos/src/renderer.rs`: `NSBox` with `cornerRadius = 10`, `fillColor = NSColor.controlBackgroundColor`, `borderType = .noBorder`; title label above if `title` prop set; padding from `SpacingToken`
- [x] T063 [P] [US8] Implement `SegmentedControl` in `rn-macos/src/renderer.rs`: `NSSegmentedControl` with segments from `options` prop; wire `on_change` → `EventPayload::Segment(i32)`; also update existing `SegmentedControl` in showcasce if already present
- [x] T064 [P] [US8] Implement `SearchBar` in `rn-macos/src/renderer.rs`: `NSSearchField` with `placeholderString` from prop; wire `controlTextDidChange` → `EventPayload::Search(String)`; wire clear button → `on_clear`
- [x] T065 [P] [US8] Add stubs for `Sidebar`, `SidebarItem`, `GroupedSection`, `SegmentedControl`, `SearchBar` in `rn-windows/src/renderer.rs` and `rn-linux/src/renderer.rs`
- [x] T066 [P] [US8] Add `sidebar()`, `sidebar_item()`, `grouped_section()`, `segmented_control()`, `search_bar()` builder functions to `rn/src/lib.rs`
- [x] T067 [US8] Build `examples/settings-demo/src/main.rs`: Settings-style app with 5 sidebar items (General, Appearance, Privacy, Notifications, Advanced), each showing a content panel with grouped sections containing `Switch` rows, `SegmentedControl`, and `SearchBar` at the top; sidebar collapses below 160px

**Checkpoint**: `cargo run -p settings-demo --features macos` opens. Sidebar selection switches panels. GroupedSection has rounded-corner tinted background. SearchBar updates in real time. SegmentedControl fires on_change.

---

## Phase 11: User Story 9 — macOS Mail App Layout (Priority: P2)

**Goal**: `SplitView` with 3 resizable panes and `Toolbar` render natively; `mail-demo` example replicates macOS Mail app structure with independent scrolling.

**Independent Test**: `cargo run -p mail-demo --features macos` opens a three-pane window; divider is draggable; message list (100 items, FlatList) scrolls independently of message detail pane; Toolbar shows Reply/Forward/Delete actions.

- [x] T068 [US9] Implement `SplitView` in `rn-macos/src/renderer.rs`: `NSSplitView` with `isVertical` from `orientation` prop; `splitView:constrainMinCoordinate:ofSubviewAt:` delegate for `min_pane_sizes`; wire resize → `EventPayload::SplitResize(Vec<f32>)`; smooth resize animation via `NSSplitViewItem.animator()`
- [x] T069 [P] [US9] Implement `SplitView` stubs in `rn-windows/src/renderer.rs` (CSplitter pattern) and `rn-linux/src/renderer.rs` (`GtkPaned` chained)
- [x] T070 [P] [US9] Implement `Toolbar` in `rn-macos/src/renderer.rs`: `NSToolbar` with `NSToolbarItem` per `ToolbarAction`; SF Symbol image from `icon_name`; label from `label`; wire `itemClicked` → `EventPayload::ToolbarAction(String)`; separator items for `ToolbarAction.separator = true`
- [x] T071 [P] [US9] Implement `Toolbar` stubs in `rn-windows/src/renderer.rs` (TBBUTTON array) and `rn-linux/src/renderer.rs` (`GtkHeaderBar` with `gtk_header_bar_pack_start`)
- [x] T072 [P] [US9] Add `split_view()` and `toolbar()` builder functions to `rn/src/lib.rs`
- [x] T073 [US9] Build `examples/mail-demo/src/main.rs`: Mail-style three-pane app — sidebar (5 folder items via `Sidebar`), message list (100 items via `FlatList`, independently scrollable), message detail (`ScrollView` + rich `Text` + `Toolbar` with Reply/Forward/Delete actions); divider draggable, min pane sizes enforced

**Checkpoint**: `cargo run -p mail-demo --features macos` opens. Pane divider is draggable. Message list scrolls independently of detail pane. Toolbar buttons fire ToolbarAction events. Min pane width enforced.

---

## Phase 12: Polish & Cross-Cutting Concerns

**Purpose**: Final wiring, StatusBar, reconciler test coverage, docs, and CI validation.

- [x] T074 [P] Implement `StatusBar` in `rn-macos/src/renderer.rs`: `NSWindow.titlebarAppearsTransparent` + `NSWindow.backgroundColor` from `ColorToken`; no-op in `rn-windows` and `rn-linux`
- [x] T075 [P] Add `status_bar()` builder to `rn/src/lib.rs`
- [x] T076 [P] Add new component reconciler tests to `tests/reconciler/tests.rs`: `Style` token round-trip (PropValue::Style survives diff), `FlatList` scroll-recycle-mount, `Modal` visibility state machine, `SegmentedControl` on_change event, `SplitView` resize event, `TouchableOpacity` bubble chain
- [x] T077 [P] Update `examples/showcase/src/main.rs` to include all new components in a comprehensive "All Components" demo (merge existing sections with new ones)
- [x] T078 Run `cargo clippy --workspace --all-targets -- -D warnings` and fix any new warnings introduced by this feature
- [x] T079 Run full quickstart validation: execute all 7 scenarios from `specs/002-react-native-parity/quickstart.md` and confirm each passes; document results
- [x] T080 Commit all changes: `git add -A && git commit -m "feat(002): RN parity + Shadcn design system"` and push to `github.com/buroapp/rust-native-ui`

**Checkpoint**: `cargo test --workspace` all green. `cargo clippy --workspace -- -D warnings` clean. Settings demo and Mail demo both run natively.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — **BLOCKS all user stories**
- **US7 Design System (Phase 3)**: Depends on Phase 2 — unblocks styled variants for all subsequent phases
- **US1, US2 (Phases 4–5)**: Depend on Phase 2; can run in parallel with US7
- **US3–US6 (Phases 6–9)**: Depend on Phase 2 (not on each other — all independently testable)
- **US8, US9 (Phases 10–11)**: Depend on Phase 2; US8 benefits from US7 (design tokens); US9 benefits from US2 (FlatList). Can start before US7/US2 are complete if feature flags gate styled variants
- **Polish (Phase 12)**: Depends on all prior phases

### User Story Dependencies

- **US7 Design System (P1)**: Independent of other stories. Start after Foundational.
- **US1 RN Model (P1)**: Independent. Start after Foundational.
- **US2 FlatList (P1)**: Builds on `rn/src/flat_list.rs` — independent of other user stories.
- **US3 SafeAreaView (P2)**: Independent.
- **US4 TouchableOpacity (P2)**: Requires event bubbling (new in T044); independent of other stories.
- **US5 Rich Text (P2)**: Independent.
- **US6 Modal (P2)**: Independent.
- **US8 Settings (P2)**: Uses `Switch` (T027, Phase 4), `SegmentedControl`, `SearchBar`, `GroupedSection`, `Sidebar` — plan Phase 4 before Phase 10 if staffing is sequential.
- **US9 Mail (P2)**: Uses `FlatList` (Phase 5), `SplitView`, `Toolbar` — plan Phase 5 before Phase 11 if sequential.

### Within Each User Story

- macOS implementation before Windows/Linux stubs (macOS is the reference implementation)
- Component type/builder in `rn` crate before platform renderer
- Platform renderer before example app integration
- Example app integration before declaring the story complete

---

## Parallel Opportunities

```bash
# Phase 2: All foundational tasks can run in parallel
T007 (ComponentKind) || T008 (PropKey) || T009 (PropValue) || T010 (EventPayload) || T011 (Style types) || T012 (Aux types)
# After T007–T012: T013 (re-exports) must complete

# Phase 3: Per-platform and builder tasks can run in parallel
T014 (macOS Style) || T015 (macOS Card) || T016 (macOS Badge) || T017 (macOS Skeleton)
T018 (Win Card) || T019 (Linux Card) || T020 (builders)

# Phase 4+5: Design system and RN model can run in parallel after Phase 2
Phase 3 (US7) || Phase 4 (US1) || Phase 5 (US2)

# Phases 6–9 can all run in parallel after Phase 2
Phase 6 (US3) || Phase 7 (US4) || Phase 8 (US5) || Phase 9 (US6)
```

---

## Implementation Strategy

### MVP Scope (P1 Stories Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US7 — Design System (tokens, Card, Badge, Skeleton)
4. Complete Phase 4: US1 — ScrollView, ActivityIndicator, Switch
5. Complete Phase 5: US2 — FlatList virtualization
6. **STOP and VALIDATE**: Run quickstart scenarios 1, 2 — confirm 10k-item 60fps list and design system tokens
7. Deploy/demo showcase + settings bench

### Incremental Delivery

1. **Foundation + Design System** → polished native controls with Shadcn tokens
2. **+ ScrollView + FlatList** → scrollable content, virtualized lists, 60fps
3. **+ SafeAreaView + TouchableOpacity + Rich Text** → complete RN layout primitives
4. **+ Modal + Popover** → overlay patterns
5. **+ Settings layout** → sidebar + grouped sections → macOS Settings demo
6. **+ Mail layout** → split-view + toolbar → macOS Mail demo

---

## Notes

- `[P]` tasks touch different files and have no incomplete dependencies — safe to run concurrently
- macOS is the reference implementation; Windows/Linux stubs just need to compile and not panic
- All `unsafe` in `rn-macos` must be in clearly labelled blocks with a one-line safety comment
- `unwrap()`/`expect()` are prohibited — use `?` or `log::error!` + early return
- Commit after each checkpoint to preserve incremental progress
- The constitution requires all three platforms ship simultaneously or be gated — each task covers all three backends

---

## Phase 13: Convergence

**Purpose**: Close the gap between the spec/plan intent and the current implementation. All 20 new component kinds currently render as empty NSView stubs. This phase implements each fully in the macOS renderer and completes the two demo example apps.

- [x] T081 [US7] Implement `Card` in `rn-macos/src/renderer.rs`: `NSBox` with `.primaryBox` style, `CALayer` corner radius from `RadiusToken`, shadow via `CALayer.shadowColor/shadowOffset/shadowRadius`, background from `ColorToken::Card` → `NSColor.controlBackgroundColor` per FR-020 (missing)
- [x] T082 [P] [US7] Implement `Badge` in `rn-macos/src/renderer.rs`: non-editable `NSTextField`, `cell.backgroundColor` from `BadgeVariant` → `NSColor`, `wantsLayer = true`, `layer.cornerRadius = RadiusToken::Full.px()` per FR-021 (missing)
- [x] T083 [P] [US7] Implement `Skeleton` in `rn-macos/src/renderer.rs`: `NSView` with `wantsLayer = true`, `CAGradientLayer` shimmer with `CABasicAnimation` on `locations` keypath, colors from `SkeletonVariant` per FR-022 (missing)
- [x] T084 [P] [US7] Implement `Style` token rendering pass in `rn-macos/src/renderer.rs`: in `update_node` match `(_, PropKey::StyleBgColor | PropKey::StyleFgColor | PropKey::StyleRadius | PropKey::StyleFont | PropKey::StyleOpacity)` and apply to the native view's `NSColor`, `NSFont`, `CALayer.cornerRadius`, `alphaValue` per FR-018/FR-019 (missing)
- [x] T085 [US1] Implement `ScrollView` in `rn-macos/src/renderer.rs`: `NSScrollView` + `NSClipView` + content `NSView`; set scroll direction from `PropKey::Direction`; wire `NSScrollView.postsBoundsChangedNotifications` → `on_scroll` handler → `EventPayload::Scroll(ScrollEvent)` per FR-001/FR-016 (missing)
- [x] T086 [P] [US1] Implement `ActivityIndicator` in `rn-macos/src/renderer.rs`: `NSProgressIndicator` with `isIndeterminate = true`; call `startAnimation`/`stopAnimation` from `PropKey::Visible`; size from `PropKey::IndicatorSize`; color from `PropKey::Color` via `NSProgressIndicator.controlTint` per FR-010 (missing)
- [x] T087 [P] [US1] Implement `Switch` in `rn-macos/src/renderer.rs`: `NSSwitch` (macOS 10.15+); `state = NSControlStateValue(value ? 1 : 0)`; wire `action` → `EventPayload::Bool(state == 1)`; handle `PropKey::SwitchValue` updates in `update_node` per FR-013 (missing)
- [x] T088 [US2] Implement `FlatList` virtualization engine in `rn/src/flat_list.rs`: `FlatListState` struct with `visible_range`, `buffer_range`, `live_pool`, `height_cache`, `total_height`, `scroll_offset`; methods `on_scroll(offset) -> RecycleOps`, `on_data_append(n) -> MountOps`, `on_data_update(idx)`, `on_item_height_change(idx, h)` per FR-002/FR-003/FR-004 (missing)
- [x] T089 [US2] Implement `FlatList` in `rn-macos/src/renderer.rs`: `NSScrollView` host + `NSView` recycle pool; use `FlatListState` to determine mount/unmount; translate scroll delegate → `FlatListState::on_scroll`; wire `on_end_reached` when `scroll_offset >= total_height - end_reach_threshold`; log `live_node_count` at `debug!` level per FR-002/FR-004/FR-017 (missing)
- [x] T090 [US4] Implement `TouchableOpacity` in `rn-macos/src/renderer.rs`: transparent `NSView` overlay subclass with custom `mouseDown`/`mouseUp`; on `mouseDown` set `layer.opacity = press_opacity`, fire `on_press` handler on `mouseUp`, restore `opacity = 1.0`; the overlay intercepts events via overriding `hitTest:` per FR-005/FR-006 (missing)
- [x] T091 [US4] Implement event bubbling in `rn-core/src/lib.rs` and `rn-macos/src/renderer.rs`: add `stop_bubbling: bool` field to `Event`; in the macOS dispatch path after firing a handler, walk up the shadow tree parent chain and fire any `OnPress` handlers on parent nodes until `stop_bubbling = true` or root is reached per FR-015 (missing)
- [x] T092 [US3] Implement `SafeAreaView` in `rn-macos/src/renderer.rs`: query `window.contentLayoutGuide.layoutFrame` at creation time; apply top inset as `NSView.frame.origin.y` offset on content child; expose insets as read-only via `PropKey::StylePadding` update per FR-007 (missing)
- [x] T093 [US5] Implement `RichText` in `rn-macos/src/renderer.rs`: non-editable `NSTextView` (no scroll wrapper); build `NSMutableAttributedString` from `PropValue::Spans(Vec<StyledSpan>)`; map `ColorToken` → `NSColor`, `FontToken` → `NSFont`, bold/italic → `NSFontDescriptor` symbolic traits, underline → `NSUnderlineStyleAttributeName`; set `textContainer.widthTracksTextView = wrap` per FR-008/FR-009 (missing)
- [x] T094 [US6] Implement `Modal` in `rn-macos/src/renderer.rs`: `NSPanel` floating window; `beginSheet:completionHandler:` when `position = Center` and parent window available; backdrop overlay `NSView` for `close_on_backdrop`; wire `on_dismiss` via completion handler; manage `ModalState` machine (Hidden → Presenting → Visible → Dismissing) per FR-011 (missing)
- [x] T095 [P] [US6] Implement `Popover` in `rn-macos/src/renderer.rs`: `NSPopover` anchored to `anchor_id` node's `NSView` using `showRelativeToRect:ofView:preferredEdge:`; wire `popoverDidClose:` → `on_dismiss` handler per FR-012 (missing)
- [x] T096 [US8] Implement `SegmentedControl` in `rn-macos/src/renderer.rs`: `NSSegmentedControl` with segments from `PropValue::StringList(options)`; set `selectedSegment` from `PropKey::SelectedIndex`; wire `action` → `EventPayload::Segment(selectedSegment)` per FR-023 (missing)
- [x] T097 [P] [US8] Implement `SearchBar` in `rn-macos/src/renderer.rs`: `NSSearchField`; set `placeholderString` from `PropKey::Placeholder`; wire `controlTextDidChange:` → `EventPayload::Search(string)`; wire clear button → `on_clear` handler per FR-024 (missing)
- [x] T098 [P] [US8] Implement `Sidebar` and `SidebarItem` in `rn-macos/src/renderer.rs`: `Sidebar` → `NSScrollView` with `NSOutlineView` inside, vibrancy background via `NSVisualEffectView`, appropriate row height 40pt; `SidebarItem` → `NSTableCellView` with SF Symbol `NSImageView` + `NSTextField` + optional badge label; `selected` prop → `NSTableView.selectRowIndexes`; wire `onSelect` → `EventPayload::Index` per FR-025/FR-026 (missing)
- [x] T099 [P] [US8] Implement `GroupedSection` in `rn-macos/src/renderer.rs`: `NSBox` with `borderType = .noBorder`, `cornerRadius = 10`, `fillColor = NSColor.controlBackgroundColor`; optional title `NSTextField` above if `PropKey::SectionTitle` set; children laid out with padding from `PropKey::StylePadding` per FR-027 (missing)
- [x] T100 [US9] Implement `SplitView` in `rn-macos/src/renderer.rs`: `NSSplitView`; set `isVertical` from `PropKey::SplitOrientation`; implement `NSSplitViewDelegate` methods `constrainMinCoordinate:ofSubviewAt:` using `PropKey::MinPaneSizes`; wire `splitViewDidResizeSubviews:` → `EventPayload::SplitResize(pane_sizes)`; animate divider via `NSSplitViewItem.animator()` per FR-028 (missing)
- [x] T101 [P] [US9] Implement `Toolbar` in `rn-macos/src/renderer.rs`: `NSToolbar`; build `NSToolbarItem` per `ToolbarAction`; SF Symbol image from `icon_name` via `NSImage.imageWithSystemSymbolName:`; wire `itemClicked:` → `EventPayload::ToolbarAction(action_id)`; insert `NSToolbarItem.separatorToolbarItem` for `separator = true` per FR-029 (missing)
- [x] T102 [US1] Implement `StatusBar` in `rn-macos/src/renderer.rs`: set `NSWindow.titlebarAppearsTransparent` and `NSWindow.backgroundColor` from `PropKey::BackgroundColor` → `NSColor`; no-op when prop absent per FR-014 (partial)
- [x] T103 [US2] Add FlatList state machine tests to `tests/reconciler/tests.rs`: scroll → recycle → mount cycle; data append preserving scroll offset; out-of-viewport update queuing; on_end_reached fires at threshold per T035 (missing)
- [x] T104 [P] [US4] Add event bubbling test to `tests/reconciler/tests.rs`: nested touch targets; verify inner handler fires before outer; `stop_bubbling = true` halts chain per T046 (missing)
- [x] T105 [P] [US6] Add Modal state machine test to `tests/reconciler/tests.rs`: Hidden → Presenting → Visible → Dismissing → Hidden transitions per T058 (missing)
- [x] T106 [US8] Build `examples/settings-demo/src/main.rs`: full Settings-style demo with 5 `SidebarItem` categories; each panel uses `GroupedSection` + `Switch` rows + `SegmentedControl` + `SearchBar` at top; sidebar collapses below 160px; all new phase-2 components exercised per SC-007 (missing)
- [x] T107 [US9] Build `examples/mail-demo/src/main.rs`: full Mail-style demo with `Sidebar` (5 folder items), `FlatList` (100 message items), `RichText` detail pane, `Toolbar` (Reply/Forward/Delete), `SplitView` 3-pane divider; independently scrollable panes per SC-008 (missing)
- [x] T108 [P] Update `examples/showcase/src/main.rs`: add design system section (Card, Badge, Skeleton, button style variants), RichText section, ScrollView section, ActivityIndicator section per T077 (missing)
- [x] T109 Rename `rn_switch` → `switch` in `rn/src/lib.rs` and update any callers in examples/ to maintain naming consistency with all other builder functions (e.g. `scroll_view`, `sidebar_item`) per Constitution V / naming convention (partial)

---

## Phase 14: Convergence

**Purpose**: Close remaining gaps from Phase 13 — clippy clean, event bubbling, scroll events, FlatList render_item, text wrap, and test coverage.

- [x] T110 Fix `cargo clippy --workspace --all-targets -- -D warnings` failures in `rn-macos/src/renderer.rs`: remove unused imports (`NSFont`, `NSFontDescriptor`, `NSProgressIndicatorStyle`, `NSTextAlignment`, `NSToolbar`, `NSToolbarItem`, `NSAttributedString`, `NSMutableAttributedString`); remove unnecessary `unsafe` blocks at lines ~844/892/906; fix unused variable `view` at line ~986 per T078 / Constitution III (partial)
- [x] T111 Add `stop_bubbling: bool` field to `Event` struct in `rn-core/src/lib.rs`; in `rn-macos/src/renderer.rs` dispatch path, after firing a handler check `event.stop_bubbling` and if false walk up shadow tree parent chain firing `OnPress` handlers until `stop_bubbling = true` or root is reached per FR-015 / US4/AC3 (missing)
- [x] T112 Wire `on_scroll` handler in `ScrollView` implementation in `rn-macos/src/renderer.rs`: add a `NSScrollView` bounds-change observer (via `addObserver:forKeyPath:options:context:` or `NotificationCenter` for `NSView.boundsDidChangeNotification`) that fires `EventPayload::Scroll(ScrollEvent { offset_x, offset_y, velocity_x: 0.0, velocity_y: 0.0 })` to the `OnScroll` handler per FR-016 (partial)
- [x] T113 Add `render_item: Option<HandlerId>` and `key_extractor: Option<HandlerId>` params to `flat_list()` builder in `rn/src/lib.rs`; add `PropKey::RenderItem` and `PropKey::KeyExtractor` PropKey variants to `rn-core/src/lib.rs`; in `rn-macos/src/renderer.rs` FlatList implementation call the `render_item` handler with the item index to get a Component for each visible slot per FR-003 / US2/AC1 (missing)
- [x] T114 [P] Apply `PropKey::TextWrap` in `RichText` implementation in `rn-macos/src/renderer.rs`: set `tv.textContainer()?.setWidthTracksTextView(wrap)` and `tv.textContainer()?.setHeightTracksTextView(false)` where `wrap = bool_prop(&props, PropKey::TextWrap, true)` per FR-009 / US5/AC2 (partial)
- [x] T115 [P] Add FlatList integration tests to `tests/reconciler/tests.rs`: (1) verify `FlatListState::compute_ops` produces Mount ops for initial viewport, (2) verify scroll-then-recycle produces Unmount+Mount pairs, (3) verify `on_data_append` preserves scroll offset per T103 (missing)
- [x] T116 [P] Add TouchableOpacity event bubbling test to `tests/reconciler/tests.rs`: create a component tree with nested `TouchableOpacity` nodes; verify that pressing the inner node fires handlers on both inner and outer nodes; verify `stop_bubbling = true` on inner prevents outer handler per T104 (missing)
- [x] T117 [P] Add showcase design system section to `examples/showcase/src/main.rs`: add a "Design System" section containing `Card` with a label child, three `Badge` variants (default/destructive/success), two `Skeleton` placeholders (100×16, 150×16), `ScrollView` with 10 label children, `ActivityIndicator`, and `Switch` per T108 (missing)
- [x] T118 [P] Add Modal visibility state machine test to `tests/reconciler/tests.rs`: verify that a `Modal` component with `ModalVisible: false` is hidden, with `ModalVisible: true` is visible, and that toggling fires the correct reconciler Update operation per T105 (missing)

---

## Phase 15: Convergence

**Purpose**: Close the final gaps — event bubbling walk, FlatList virtualization connected to macOS renderer, RichText attributed strings, and press-opacity on TouchableOpacity.

- [ ] T119 Implement parent-chain walk in `rn/src/lib.rs` `dispatch_handler` (or in `rn-macos/src/renderer.rs` after dispatch): after firing a handler, check `event.stop_bubbling`; if `false`, look up the source `NodeId` in the shadow tree, walk parent nodes, and fire any `OnPress` `HandlerId` found on `ComponentKind::TouchableOpacity` ancestors until `stop_bubbling = true` or the root is reached. The shadow tree parent lookup is available via `state.shadow.get(node_id).map(|s| s.parent)` per FR-015 / US4/AC3 (partial)
- [ ] T120 Connect `FlatListState` to the macOS FlatList renderer in `rn-macos/src/renderer.rs`: on Create, instantiate `FlatListState` and store it alongside the NSScrollView (add a new `NativeHandle::FlatListWidget(Retained<NSScrollView>, FlatListState)` variant); on each tick poll, call `flat_list_state.on_scroll(current_offset)` and for each `RecycleOp::Mount { data_index, pool_slot }`, call the `render_item` HandlerId with `EventPayload::Index(data_index as i32)` and add the returned component as a child at the correct Y offset; for `RecycleOp::Unmount`, hide the pool slot NSView per FR-002/FR-004 / US2/AC1 (partial)
- [ ] T121 [P] Implement `NSAttributedString`/`NSMutableAttributedString` span rendering in `RichText` in `rn-macos/src/renderer.rs`: for each `StyledSpan` build an `NSAttributedString` with `NSForegroundColorAttributeName` from `ColorToken` → `NSColor`, bold/italic via `NSFontDescriptorSymbolicTraits`, underline via `NSUnderlineStyleAttributeName`, then join via `NSMutableAttributedString.append()`; replace the current `setString()` call with `setAttributedString()` on the `NSTextView.textStorage()` per FR-008 / US5/AC1 (partial)
- [ ] T122 [P] Implement press-opacity feedback in `TouchableOpacity` in `rn-macos/src/renderer.rs`: subclass `NSView` via `declare_class!` with `mouseDown`/`mouseUp` overrides; on `mouseDown`, read `PropKey::PressOpacity` from shadow props and set `view.layer.opacity`; on `mouseUp`, restore `opacity = 1.0` and fire the `OnPress` handler via `dispatch_event`; replace the current plain `NSView::initWithFrame` with this subclass per FR-005/FR-006 / US4/AC1 (partial)
- [ ] T123 [P] Add `--bench-flatlist` mode to `examples/settings-demo/src/main.rs`: when the argument is present, render a `FlatList` with 10,000 items (row height 44px, viewport ~400px); on each scroll-position tick log `live_nodes: N` and approximate RSS memory to stdout so SC-001/SC-002 can be manually validated per SC-001/SC-002 / US2 (missing)
