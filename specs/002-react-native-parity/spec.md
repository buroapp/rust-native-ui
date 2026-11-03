# Feature Specification: React Native Component Parity

**Feature Branch**: `002-react-native-parity`

**Created**: 2026-11-02

**Status**: Draft

**Input**: User description: "extend rn to having same components as React Native, plus Shadcn-style design system with enough fidelity to replicate macOS Settings app and Mail app"

## User Scenarios & Testing

### User Story 1 — Build a Full Desktop App from a RN Mental Model (Priority: P1)

A Rust developer who knows React Native wants to build a desktop application using `rn`. They expect to reach for the same component vocabulary — `ScrollView`, `FlatList`, `SafeAreaView`, `TextInput`, `TouchableOpacity`, `ActivityIndicator`, `Text` — and find working native equivalents on their platform. The mental model is the same: declarative JSX-like Rust, state-driven rendering, minimal updates.

**Why this priority**: The framework's value proposition is "React Native but desktop-native." Without the core RN component surface, developers must learn a completely new vocabulary and the framework is just another widget toolkit with a reconciler on top.

**Independent Test**: A developer can build a complete app — a data table with scrollable rows, text editing, loading states, and nested scroll views — and it runs natively on macOS with all controls being real OS widgets.

**Acceptance Scenarios**:

1. **Given** an RN-style component tree using `ScrollView`, `FlatList`, and `Text`, **When** the app renders on macOS, **Then** the scroll view drives native `NSScrollView`, the list drives a custom `NSTableView` with virtualized rows, and text renders with native `NSTextView` support for attributed strings.

2. **Given** a `TouchableOpacity` wrapping nested children, **When** the user presses it, **Then** the press event bubbles to the outermost handler, visual feedback shows a highlight, and no child control loses focus.

3. **Given** an `ActivityIndicator` with a defined color and size, **When** it renders on any platform, **Then** it uses the platform's native spinner widget (NSProgressIndicator indeterminate, Win32 ProgressBar marquee, GTK4 Spinner) with the requested color mapped to the closest native palette.

---

### User Story 2 — Virtualized Long Lists (Priority: P1)

A developer needs to display thousands of items in a scrollable list. They expect `rn::flat_list` to virtualize — only render the items visible in the viewport plus a small buffer. Scrolling must remain at 60fps even with 10,000 items.

**Why this priority**: This is the single most common RN pattern (data tables, message feeds, file browsers) and the single hardest to get right in a declarative framework. Without it, the framework is unusable for real apps.

**Independent Test**: A list of 10,000 text items scrolls smoothly at 60fps. Scrolling up and down reuses the same 15-20 row components. Memory stays constant (~2MB) regardless of list length.

**Acceptance Scenarios**:

1. **Given** a `FlatList` with 10,000 items and a viewport height of 400px, **When** the user scrolls 1000px, **Then** only ~15 row components are alive in the native tree at any moment, and total memory is ≤5MB.

2. **Given** a `FlatList` with an `item_height` property and variable-content items, **When** new items are appended at runtime, **Then** the viewport preserves its scroll position without jumping.

3. **Given** a `FlatList` where the user is rapidly scrolling, **When** a state update modifies a non-visible item's data, **Then** the update is queued and applied after the scroll settles (no dropped frames during scroll).

---

### User Story 3 — Safe Area & Screen Insets (Priority: P2)

A developer places `SafeAreaView` at the root of their UI. On macOS, this avoids macOS title bars and menubar. On Windows, it avoids the title bar and caption buttons. On Linux, it avoids any window manager chrome or GNOME panel. The framework abstracts the platform-specific inset calculation.

**Why this priority**: Desktop platforms don't have a unified safe-area model, but developers coming from RN (where it's standard) will expect it. Without it, layouts will collide with OS chrome on platforms that have non-rectangular chrome.

**Independent Test**: An app with a `SafeAreaView` wrapping content at the edges renders content entirely within the platform's safe zone across macOS, Windows, and Linux.

**Acceptance Scenarios**:

1. **Given** a window with `SafeAreaView` at the root, **When** the app launches on macOS, **Then** the content inset accounts for the macOS title bar height (36px) and window controls.

2. **Given** the same window, **When** the app launches on Windows 11, **Then** the content inset accounts for the Windows 11 title bar height.

3. **Given** a window that is maximized, **When** `SafeAreaView` is rendered, **Then** the inset matches the platform's taskbar/panel height.

---

### User Story 4 — Touchable & Pressable Interactions (Priority: P2)

A developer wraps a component in `TouchableOpacity` or `TouchHighlight`. When the user presses, the component fades or highlights. On release, the event fires. The interaction works consistently across all platforms using native hover/press feedback.

**Why this priority**: This is the fundamental interaction primitive for clickable content in RN. Without it, every button-like component needs a custom implementation on every platform.

**Independent Test**: Wrapping any child in `TouchableOpacity` adds press-highlight visual feedback and fires `on_press` on release, without requiring native widget-specific code in the consumer.

**Acceptance Scenarios**:

1. **Given** a `TouchableOpacity` with `opacity: 0.5` wrapping a `Text` and `Image`, **When** the user presses it, **Then** the children visually dim by the specified opacity during the press.

2. **Given** a `TouchableOpacity` on macOS, **When** the user presses and releases, **Then** the visual feedback matches the native "tinted" press effect of `NSButton`-like controls.

3. **Given** nested `TouchableOpacity` components, **When** the innermost is pressed, **Then** the event is delivered to all handlers in the chain (from outer to inner), matching RN's event bubble semantics.

---

### User Story 5 — Rich Text & Paragraph Layout (Priority: P2)

A developer needs to render multi-line text with varying styles (bold, italic, color, font size) within a single text component. On each platform, the underlying native text widget supports attributed/attributable strings. The framework abstracts this into a single `Text` component with styled spans.

**Why this priority**: RN's `Text` is the cornerstone of layout. Without attributed text support, the framework is limited to monospaced or single-style labels — unacceptable for any real app.

**Acceptance Scenarios**:

1. **Given** a `Text` component with a mix of `style={{ bold: true, color: "red" }}` spans, **When** it renders on macOS, **Then** the text uses `NSAttributedString` with the correct font attributes.

2. **Given** a `Text` component with a long string that wraps across lines, **When** the window is resized, **Then** the text reflows to fit the available width.

---

### User Story 6 — Modal & Popover Dialogs (Priority: P2)

A developer shows a modal dialog or a popover that sits above the app content. The modal is centered, blocks interaction with the background, and optionally has a backdrop overlay. On each platform, this uses the native modal dialog or popover API.

**Acceptance Scenarios**:

1. **Given** a `Modal` with `visible: true` and children, **When** it renders, **Then** it appears as a native modal dialog on macOS (sheet or window), a Win32 modal dialog, or a GTK4 dialog window.

2. **Given** a `Modal` is visible, **When** the user clicks outside the modal (on the backdrop), **Then** if `close_on_backdrop_press` is set, the modal closes and fires `on_dismiss`.

3. **Given** a `Popover`, **When** it is rendered attached to a specific anchor point, **Then** it appears at the platform-native popover position with appropriate animation.

---

### User Story 7 — Shadcn-Style Design System (Priority: P1)

A developer wants their app to look polished, consistent, and familiar without writing platform-specific styling code. They expect a Shadcn-like design system: color tokens (primary, secondary, destructive, muted, background, foreground), a spacing scale (0–8px steps), typography tokens (heading sizes, body text), border radii, and focus rings. Each platform's native renderer applies these tokens to the underlying native controls — so a `Button` styled "secondary" looks like a macOS secondary button, a Win32 outlined button, or a GTK flat button, but all share the same semantic token values.

The framework exposes a `Style` type (or builder) that lets consumers apply tokens to any component:

```rust
rn::button("Save")
    .style(Style::secondary())
    .rounded(Radius::md)
    .build()
```

On macOS, this produces an `NSButton` with `bezelStyle` = secondary, `cornerRadius` = 6, `font` = system, `textColor` = primary text. On Windows, it produces a Win32 button with matching visual parameters. On Linux, GTK CSS variables are set to match.

**Why this priority**: Without a design system, every app looks like a default OS toolkit — inconsistent, unpolished, hard to build a brand on. Shadcn proves that a well-crafted token system + native controls > custom-rendered widgets for developer satisfaction and visual quality.

**Independent Test**: A developer applies the Shadcn token system to a button and sees platform-native styling (macOS secondary bezel, Windows flat button, GTK flat button) with consistent color and spacing values across all three platforms.

**Acceptance Scenarios**:

1. **Given** a `Button` styled with `Style::secondary()`, **When** it renders on macOS, **Then** the native `NSButton` uses the secondary bezel style with the platform's secondary color token and the specified border radius.

2. **Given** a `Button` styled with `Style::destructive()`, **When** it renders on all three platforms, **Then** the destructive color token (a red shade) is applied to the button text or border, matching the platform's native destructive styling conventions.

3. **Given** a `Card` component, **When** it renders on all three platforms, **Then** it wraps its children in a native container with appropriate padding, a subtle shadow, and rounded corners, consistent with the macOS Settings app aesthetic.

4. **Given** a `Skeleton` component, **When** it renders while data loads, **Then** it shows a placeholder rectangle with a platform-native shimmer animation, matching the visual placeholder used in macOS Mail and Settings.

5. **Given** a `Badge` with a color variant, **When** it renders on macOS, **Then** it uses a native text field with background tint and corner radius, matching the macOS Settings badge style.

---

### User Story 8 — macOS Settings App Layout (Priority: P2)

A developer wants to replicate the macOS Settings app layout — a persistent sidebar navigation on the left, a main content area on the right with grouped settings, and a search bar at the top. The framework must provide the structural components to build this: `Sidebar`, `SidebarItem`, `ContentArea`, `GroupedSection`, `SegmentedControl`, `SearchBar`, and the ability to nest them at any depth.

The macOS Settings app is the canonical desktop layout for system-level applications. It demonstrates the framework's ability to handle complex, persistent navigation patterns — not just single-page apps or forms.

**Why this priority**: This is the primary demonstration target for the design system + structural components. If the framework can reproduce a macOS Settings app natively, it proves the design system works at scale and the layout primitives are sufficient for serious desktop apps.

**Independent Test**: A developer builds a Settings-style app with a sidebar of categories, a content area with grouped settings panels, toggle switches, segmented controls, and a search bar. The layout adapts to window resizing, the sidebar can collapse, and all controls are native.

**Acceptance Scenarios**:

1. **Given** a `Sidebar` with five `SidebarItem` children, **When** the app renders on macOS, **Then** the sidebar uses a native `NSSidebarViewController` (or equivalent) with the correct inset, row height, and selection highlight style.

2. **Given** a `GroupedSection` with multiple controls, **When** it renders, **Then** the children are enclosed in a native container with rounded corners, subtle background tint, and internal padding, matching the macOS Settings grouped style.

3. **Given** a `SearchBar` at the top of the content area, **When** the user types, **Then** the search text updates in real time and the framework fires a `SearchFilter` event to the consumer's handler.

4. **Given** a `SegmentedControl` with three segments, **When** a segment is selected, **Then** the native segmented control reflects the selection and the framework fires an `on_change` event.

5. **Given** a window that is resized, **When** the sidebar width is constrained, **Then** the sidebar collapses to icon-only mode (or hides, per platform convention) and the content area expands to fill the available space.

---

### User Story 9 — macOS Mail App Layout (Priority: P2)

A developer wants to replicate the macOS Mail app layout — a three-pane layout with a sidebar (folder list) on the left, a message list in the middle, and a message detail view on the right. The framework must provide the structural components to build this: `SplitView` (with resizable panes), `MessageList`, `MessagePreview`, `Toolbar`, and the ability to nest scrollable containers.

The macOS Mail app is the canonical desktop layout for information-dense applications. It demonstrates the framework's ability to handle complex, multi-pane layouts with independent scrolling regions — a pattern common in productivity apps, email clients, and file browsers.

**Why this priority**: Three-pane layouts are the hardest desktop layout pattern to get right. If the framework supports this natively, it proves the layout primitives are sufficient for the most demanding real-world desktop apps.

**Independent Test**: A developer builds a Mail-style app with a folder sidebar, a message list (using `FlatList` with 100 items), and a message preview area. The panes are independently scrollable, the user can resize the divider between panes, and all content is native.

**Acceptance Scenarios**:

1. **Given** a `SplitView` with three panes (sidebar, message list, message detail), **When** the app renders on macOS, **Then** each pane uses native scrollable containers (`NSScrollView`) and the divider between panes is draggable to resize.

2. **Given** a `MessageList` with 100 items, **When** the user scrolls, **Then** the list uses virtualized rows (via `FlatList`) and scrolling remains at 60fps.

3. **Given** a `Toolbar` with multiple action buttons, **When** it renders at the top of the message detail pane, **Then** it uses the platform's native toolbar API (NSVisualEffectView + NSButtons on macOS, Win32 toolbar on Windows, GTK toolbar on Linux) with the specified icon and label for each action.

4. **Given** a `SplitView` whose divider is dragged by the user, **When** the divider crosses a resize threshold, **Then** the pane transition from narrow-to-wide (or wide-to-narrow) is animated smoothly on all platforms.

5. **Given** the `MessageDetail` pane contains rich text from the message body, **When** the text is longer than the pane, **Then** the `ScrollView` inside the pane scrolls independently of the other panes.

---

### Edge Cases

- What happens when a `FlatList` receives items that change size dynamically? (Viewport must track item height maps and update positions.)
- What happens when `TouchableOpacity` wraps an already-interactive native control (e.g., a button)? (The outer press highlight should layer on top; the inner control's native behavior takes precedence.)
- How does `SafeAreaView` behave on a window with no chrome (fullscreen)? (Insets are zero; content fills the full window.)
- What happens when text content is longer than the available width in a non-wrapping `Text`? (Platform default: horizontal scrolling or clipping.)
- What happens when a modal is shown from within a `FlatList` item? (Modal must be parented at the window level, not within the list's scroll view, to avoid clipping.)

## Requirements

### Functional Requirements

- **FR-001**: The framework MUST provide a `ScrollView` component that wraps its children in a native scrollable container, with a `direction` property (vertical, horizontal, both).

- **FR-002**: The framework MUST provide a `FlatList` component that renders a scrollable list of items using virtualization — only items in the visible viewport plus a configurable buffer are rendered as native controls.

- **FR-003**: `FlatList` MUST accept `data`, `item_height` (fixed or dynamic), `key_extractor`, and `render_item` (a function that returns a `Component` for each item).

- **FR-004**: `FlatList` MUST maintain a constant number of live native components regardless of total item count (virtualization).

- **FR-005**: The framework MUST provide a `TouchableOpacity` component that adds press-visual feedback and forwards the press event to an `on_press` handler.

- **FR-006**: `TouchableOpacity` MUST accept an `opacity` property (0.0–1.0) for the highlight effect during press.

- **FR-007**: The framework MUST provide a `SafeAreaView` component that insets its children by the platform's safe area (title bar, menubar, taskbar heights).

- **FR-008**: The framework MUST provide a `Text` component that supports styled spans with `font_weight`, `font_size`, `color`, `font_family`, and `italic` properties.

- **FR-009**: `Text` MUST wrap text at word boundaries when the available width is smaller than the content width (unless `wrap: false` is set).

- **FR-010**: The framework MUST provide an `ActivityIndicator` component that shows a platform-native indeterminate progress indicator (spinner) with configurable `color` and `size`.

- **FR-011**: The framework MUST provide a `Modal` component that presents a dialog or window above the app content, with `visible`, `on_dismiss`, `close_on_backdrop_press`, and optional `position` (center, top, bottom, popover).

- **FR-012**: The framework MUST provide a `Popover` component that positions itself relative to a given anchor point using the platform's native popover API.

- **FR-013**: The framework MUST provide a `Switch` component that provides an on/off toggle using the native toggle switch widget.

- **FR-014**: The framework MUST provide a `StatusBar` component that, on platforms that support it (Windows 11, macOS), can query or adjust the status bar color.

- **FR-015**: Event bubbling MUST follow React Native semantics: events bubble from the most specific (innermost) handler to the outermost, with `stop_bubbling` available to halt propagation.

- **FR-016**: `ScrollView` MUST support `content_offset` for programmatic scroll positioning and `on_scroll` for observing scroll position changes.

- **FR-017**: `FlatList` MUST support `on_end_reached` to be called when the user scrolls near the end of the data (with configurable `end_reach_threshold` in pixels).

- **FR-018**: The framework MUST provide a design system with token types: `ColorToken` (primary, secondary, destructive, muted, background, foreground), `SpacingToken` (xs=4, sm=8, md=12, lg=16, xl=20, xxl=24, xxxl=32), `RadiusToken` (none, sm=2, md=4, lg=6, xl=8, full), `FontToken` (body, heading-sm, heading-md, heading-lg, mono), and `FocusRingToken` (width, color, style).

- **FR-019**: A `Style` builder type MUST expose methods: `.color(ColorToken)`, `.spacing(SpacingToken)`, `.rounded(RadiusToken)`, `.font(FontToken)`, `.focus_ring(FocusRingToken)`, and `.on_press(HandlerId)` that chain and return a `Component`.

- **FR-020**: The framework MUST provide a `Card` component that wraps children in a native container with `background` token, `padding` token, `shadow` token, and `border_radius` token applied to the platform native widget.

- **FR-021**: The framework MUST provide a `Badge` component that renders a small labeled pill with `color_variant` (default, secondary, destructive, success, warning) and `size` (sm, md, lg) properties using native colored background + text styling.

- **FR-022**: The framework MUST provide a `Skeleton` component that renders a placeholder rectangle with shimmer animation using `color_variant` (muted, card) and `radius` (sm, md, lg) properties.

- **FR-023**: The framework MUST provide a `SegmentedControl` component that renders native segmented buttons with `options` (string list), `selected_index`, and `on_change` properties.

- **FR-024**: The framework MUST provide a `SearchBar` component that provides a native search input with `placeholder`, `on_change`, and `on_clear` properties.

- **FR-025**: The framework MUST provide a `Sidebar` component that renders a persistent left navigation panel with `width` property, using native sidebar widget on each platform.

- **FR-026**: The framework MUST provide a `SidebarItem` component that renders as a selectable item within a `Sidebar`, with `label`, `icon_name`, `on_select`, and `selected` properties.

- **FR-027**: The framework MUST provide a `GroupedSection` component that wraps children in a native container with grouped styling (rounded corners, background tint, internal padding, inter-item spacing).

- **FR-028**: The framework MUST provide a `SplitView` component that renders two or three resizable panes with draggable dividers, each pane containing native scrollable containers.

- **FR-029**: The framework MUST provide a `Toolbar` component that renders a native horizontal bar with action buttons, separators, and optional text labels, using the platform's native toolbar API.

### Key Entities

- **ScrollView**: A scrollable container that wraps its children in the platform's native scrollable widget. Properties: `direction`, `scrollable`, `content_offset`, `children`.

- **FlatList**: A virtualized list renderer. Properties: `data`, `item_height`, `key_extractor`, `render_item`, `on_end_reached`, `end_reach_threshold`, `scrollable`.

- **TouchableOpacity**: A pressable wrapper that adds visual feedback. Properties: `on_press`, `opacity`, `children`.

- **SafeAreaView**: A container that insets children by platform safe area values. Properties: `children`, `insets` (read-only from platform).

- **Text**: A text renderer with styled spans. Properties: `style` (font_weight, font_size, color, font_family, italic), `on_press`, `children` (nested styled spans).

- **ActivityIndicator**: A spinner/indicator. Properties: `color`, `size`, `visible`.

- **Modal**: A modal dialog/window. Properties: `visible`, `on_dismiss`, `close_on_backdrop_press`, `position`, `children`.

- **Popover**: A floating panel anchored to a point. Properties: `anchor_point`, `offset`, `children`.

- **Switch**: An on/off toggle. Properties: `value`, `on_change`.

- **StatusBar**: System bar integration. Properties: `visible`, `background_color` (where supported).

- **Style**: A builder for component appearance. Methods: `color()`, `spacing()`, `rounded()`, `font()`, `focus_ring()`, `on_press()`. Chainable.

- **ColorToken**: Semantic color values (primary, secondary, destructive, muted, background, foreground). Each platform maps to its native palette.

- **SpacingToken**: Fixed spacing values (xs=4, sm=8, md=12, lg=16, xl=20, xxl=24, xxxl=32) applied as padding/margin on components.

- **RadiusToken**: Border radius values (none, sm=2, md=4, lg=6, xl=8, full) applied to container widgets.

- **FontToken**: Typography tokens (body, heading-sm, heading-md, heading-lg, mono) mapped to platform system fonts.

- **FocusRingToken**: Keyboard focus indicator (width, color, style) applied to interactive components when focused.

- **Card**: A rounded container with shadow, background, and padding. Properties: `padding`, `shadow`, `border_radius`, `children`.

- **Badge**: A small labeled pill. Properties: `label`, `color_variant`, `size`, `on_press`.

- **Skeleton**: A loading placeholder with shimmer. Properties: `color_variant`, `radius`, `width`, `height`.

- **SegmentedControl**: Native segmented button group. Properties: `options`, `selected_index`, `on_change`.

- **SearchBar**: Native search input. Properties: `placeholder`, `on_change`, `on_clear`.

- **Sidebar**: Persistent left navigation panel. Properties: `width`, `children` (SidebarItem children).

- **SidebarItem**: Selectable sidebar entry. Properties: `label`, `icon_name`, `on_select`, `selected`.

- **GroupedSection**: Container for grouped UI elements. Properties: `children`.

- **SplitView**: Multi-pane resizable container. Properties: `panes` (2–3), `divider_width`, `min_pane_width`, `children`.

- **Toolbar**: Horizontal action bar. Properties: `actions` (button list), `children`.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A developer can build a scrollable data list with 10,000 items that scrolls at 60fps on minimum target hardware.

- **SC-002**: `FlatList` with 10,000 items uses ≤5MB of live memory at any time (constant regardless of item count).

- **SC-003**: A developer with React Native experience can port a simple RN app's layout to `rn` with the same component vocabulary within 2 days.

- **SC-004**: All new components render using genuine native OS widgets on all three supported platforms (macOS, Windows, Linux).

- **SC-005**: Event bubbling in `TouchableOpacity` and `Text` matches RN semantics: inner handlers fire before outer handlers, `stop_bubbling` halts propagation.

- **SC-006**: `SafeAreaView` correctly insets content on all three platforms without the developer needing to write platform-specific code.

- **SC-007**: A developer can build a macOS Settings-style app (sidebar, grouped sections, segmented controls, search) using the design system, and all controls render as native widgets with consistent styling across the three platforms.

- **SC-008**: A developer can build a macOS Mail-style app (three-pane SplitView, virtualized message list, rich text preview) using the framework, with independently scrollable panes and 60fps scrolling.

- **SC-009**: The design system color tokens produce visually consistent results across platforms: a `Style::secondary()` button on macOS, Windows, and Linux all share the same primary/secondary/muted color ratios, even though the native widget appearance differs.

## Assumptions

- The macOS backend (AppKit) is the reference implementation; Windows (Win32) and Linux (GTK4) backends will follow the same API and component behavior.

- Mobile platforms (iOS, Android) are explicitly out of scope per the constitution.

- The reconciler architecture (Phase 1) is already in place and extends seamlessly to new component types.

- Virtualization in `FlatList` uses a simple row-by-row approach (not a complex section-based model like RN's `SectionList`), which covers the common case of single-flat data lists.

- The framework does not include a navigation library (stack/tab/drawer navigators from RN). Those are scoped to a future feature (navigation).

- `StatusBar` support is best-effort: where the OS exposes the API (Windows 11, macOS), it works; where it does not (Linux, most window managers), it is a no-op.

- Shadcn design system tokens are applied at the platform renderer layer — the consumer never interacts with platform-specific styling APIs. Each backend maps the semantic token values to the closest native equivalent.

- macOS Settings and Mail app layouts are demonstration targets, not exact clones. The framework provides the building blocks; the developer composes them into their desired layout.

## Out of Scope

- `SectionList` / nested sections — covered by a future feature if demand arises.

- Navigation components (StackNavigator, TabNavigator, DrawerNavigator) — scoped to a separate feature.

- Pull-to-refresh — a component with that behavior is desirable but low priority for v2.

- Platform-specific status bar APIs (iOS not in scope; macOS is best-effort; Windows 11 is supported).

- Complex text features (text selection, rich text editing) — text rendering with basic styles is the scope; editing is deferred.

- Gesture recognizers beyond basic press/tap — swipe, pinch, long-press are out of scope.
