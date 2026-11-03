# Feature Specification: Platform Parity — Windows and Linux Backends

**Feature Branch**: `003-platform-parity`

**Created**: 2026-11-03

**Status**: Draft

**Input**: User description: "next phase"

**Context**: The framework currently supports only one of its three declared desktop platforms. The other two are placeholders that produce an empty window. The project constitution declares all three desktop platforms first-class and requires verification on all three. This feature closes that gap.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Same app, native Windows controls (Priority: P1)

A developer builds an application against the framework's public API on macOS. Without changing a line of their application code, they compile and run the same application on Windows and see native Win32 controls — real buttons, edit fields, combo boxes, trackbars, switches — not custom-drawn imitations.

**Why this priority**: Without a working Windows backend the framework is a macOS-only library wearing a cross-platform API. This is the single largest gap and the one the constitution flags as non-negotiable.

**Independent Test**: Build and launch the existing settings demonstration application on a Windows machine using only its documented feature flag. The window appears and every visible control responds to input.

**Acceptance Scenarios**:

1. **Given** an application written against the public API, **When** it is compiled with the Windows feature enabled, **Then** it compiles with zero changes to application source.
2. **Given** the compiled Windows application, **When** it launches, **Then** every component in its tree is drawn by an operating-system-provided control rather than by custom painting.
3. **Given** the running application, **When** the user interacts with a button, toggle, text field, and drop-down, **Then** each produces the same application-level event as it does on macOS.

---

### User Story 2 - Same app, native Linux controls (Priority: P1)

The same developer compiles the same application on Linux and sees native GTK4 widgets with the desktop's own theme, spacing conventions, and input behaviour.

**Why this priority**: Equal to Windows — the constitution gives all three platforms the same standing, and a two-of-three backend set is no better than one-of-three for a Linux-targeting user.

**Independent Test**: Build and launch the existing settings demonstration application on a Linux machine with the Linux feature flag. The window appears under both a Wayland and an X11 session.

**Acceptance Scenarios**:

1. **Given** an application written against the public API, **When** compiled with the Linux feature enabled, **Then** it compiles with zero changes to application source.
2. **Given** the compiled Linux application, **When** launched under Wayland, **Then** the window renders and accepts keyboard and pointer input.
3. **Given the compiled Linux application, **When** launched under X11, **Then** the window renders and accepts keyboard and pointer input.
4. **Given** the running application, **When** the desktop theme changes between light and dark, **Then** native controls update without application code.

---

### User Story 3 - Layout is identical across platforms (Priority: P2)

A developer positions components using the framework's layout primitives. The resulting geometry is materially the same on every platform, so a layout tuned on one platform does not break on another.

**Why this priority**: Cross-platform value collapses if each backend re-derives its own geometry. This story protects the investment already made in the shared layout engine.

**Independent Test**: Render the same component tree on all three platforms and compare the computed bounds of every node.

**Acceptance Scenarios**:

1. **Given** the same component tree and window size, **When** rendered on any two supported platforms, **Then** each node's position and size agree within a small fixed tolerance.
2. **Given** a layout using flexible growth and stretch alignment, **When** rendered on each platform, **Then** the space distribution is equivalent.

---

### User Story 4 - Automated cross-platform verification (Priority: P2)

A maintainer opens a pull request that changes the public API or a shared crate. Continuous integration builds and tests on all three platforms and reports a failure if any backend stops compiling or is missing an implementation.

**Why this priority**: Parity that is not enforced decays immediately. The stubs exist precisely because nothing failed when they were left empty.

**Independent Test**: Introduce a deliberate change to a shared type and confirm that all three platform jobs fail; introduce a platform-specific regression and confirm only that platform's job fails.

**Acceptance Scenarios**:

1. **Given** a pull request touching a shared crate, **When** CI runs, **Then** build and test jobs execute on macOS, Windows, and Linux.
2. **Given a new component kind added to the shared type system, **When** CI runs before any backend implements it, **Then** the build fails and names the unimplemented platforms.
3. **Given a fully implemented change, **When** CI runs, **Then** all three platform jobs pass.

---

### User Story 5 - Native accessibility on every platform (Priority: P3)

A screen-reader user runs an application on Windows or Linux and can navigate its controls, hear their labels and roles, and operate them by keyboard, in the same way they can on macOS.

**Why this priority**: Native controls provide accessibility largely for free, but only if the backend maps the framework's semantic properties onto each platform's accessibility model instead of discarding them.

**Independent Test**: Inspect the platform accessibility tree of a running application and confirm each interactive control exposes a role, a name, and a value.

**Acceptance Scenarios**:

1. **Given** a running application, **When** its platform accessibility tree is inspected, **Then** every interactive control exposes a role and an accessible name.
2. **Given** a control with a bound text label, **When** the accessibility tree is inspected, **Then** the label text is the control's accessible name rather than the control being unnamed.
3. **Given** a keyboard-only user on any platform, **When** they press the platform's standard focus-traversal keys, **Then** focus moves through the interactive controls in tree order.

---

### Edge Cases

- **No display server present**: a Linux CI runner without a display must skip windowed tests rather than fail the build.
- **Missing GTK4 development libraries**: the Linux feature must fail at compile time with an actionable message, not at link time with an opaque one.
- **High-DPI / fractional scaling**: a layout specified in logical units must render at the same apparent size on every platform, including fractional scale factors.
- **Wayland vs X11**: window decorations, global menus, and screen coordinates differ between the two Linux display protocols; both must work.
- **Text metrics differ**: the same string occupies different widths in each platform's default UI font; layout must tolerate this without clipping labels.
- **Platform idioms with no direct equivalent**: macOS has unified title-bar/toolbar and native sidebar vibrancy; Windows and Linux have no identical concept and need a faithful local idiom rather than a copy of the macOS look.
- **Unimplemented component reached at runtime**: an application using a component the current platform has not implemented must produce a clear diagnostic rather than an invisible or zero-sized control.
- **Headless / server builds**: building for a platform whose native libraries are absent must not break compilation of the shared crates.

## Requirements *(mandatory)*

### Functional Requirements

**Backend completeness**

- **FR-001**: The system MUST provide a Windows backend that renders every component kind the framework exposes, using operating-system-provided controls.
- **FR-002**: The system MUST provide a Linux backend that renders every component kind the framework exposes, using GTK4-provided widgets.
- **FR-003**: Each backend MUST declare which component kinds it implements, in a form that a build check can verify, so a missing implementation fails the build rather than degrading silently at runtime.
- **FR-004**: Reaching an unimplemented component at runtime MUST produce a diagnostic naming the component and the platform.

**API stability**

- **FR-005**: The public API crate MUST compile unchanged for all three target platforms; no application source file may require platform conditionals to build.
- **FR-006**: No platform-specific type may appear in the public API surface.
- **FR-007**: Platform backends MUST be selectable by build feature, and at most one backend may be active in a given build.

**Behavioural parity**

- **FR-008**: A single shared layout computation MUST be the sole source of geometry on all platforms; backends MUST NOT compute their own positions or sizes.
- **FR-009**: Component properties carrying application state (toggled state, selected index, text value, value and range) MUST be applied on update on every platform, not only at creation.
- **FR-010**: Event dispatch MUST preserve the same semantics across backends, including bubbling for nested interactive containers and identical payload shapes.
- **FR-011**: Scrolling and large-list virtualisation MUST operate on every platform, mounting only the visible window of items rather than the whole list.
- **FR-012**: Text editing MUST produce equivalent change and submit events on every platform.

**Appearance**

- **FR-013**: Backends MUST NOT custom-draw a control where a native control exists for that purpose.
- **FR-014**: Backends MUST adopt the host platform's current light or dark appearance and update when the user changes it.
- **FR-015**: Backends MUST render correctly at the host platform's display scaling, including fractional scale factors.
- **FR-016**: Platform chrome concepts (title bar, toolbar, sidebar) MUST be expressed with that platform's own idiom rather than by imitating another platform's appearance.

**Accessibility**

- **FR-017**: Every interactive control MUST expose an accessible role and an accessible name on every supported platform.
- **FR-018**: Where a control is associated with a text label, that label MUST become the control's accessible name.
- **FR-019**: Keyboard focus traversal MUST reach every interactive control in component-tree order on every platform.

**Quality gates**

- **FR-020**: Continuous integration MUST build and test on macOS, Windows, and Linux for every change to a shared crate or the public API.
- **FR-021**: Applications MUST NOT crash the process on recoverable input or rendering errors on any platform; failures MUST surface as a diagnostic instead.
- **FR-022**: The existing example applications MUST build and run on all three platforms without platform-specific source branches.

### Key Entities

- **Backend**: A platform-specific implementation of the rendering contract, selected by build feature, responsible for creating and updating native controls and for emitting application events.
- **Component Kind**: The platform-neutral enumeration of every control the framework can render. Its size defines the completeness obligation for each backend.
- **Capability Declaration**: The per-backend record of which component kinds are implemented, used to fail builds on gaps.
- **Layout Result**: The shared, platform-independent set of computed bounds that every backend consumes without modification.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All three bundled demonstration applications build and launch on all three supported platforms — nine of nine combinations.
- **SC-002**: Every component kind the framework exposes is rendered by a native control on each supported platform — 100% coverage per platform, enforced by the build.
- **SC-003**: For an identical component tree and window size, node bounds agree across platforms within two logical pixels.
- **SC-004**: A change to a shared crate produces build results for all three platforms on every pull request, with no manual dispatch.
- **SC-005**: Zero application source files require platform conditionals to build on any supported platform.
- **SC-006**: A list of ten thousand items maintains a bounded number of live native controls (under one hundred) on every platform, verified by instrumentation.
- **SC-007**: Every interactive control in the demonstration applications exposes an accessible role and name on all three platforms.
- **SC-008**: A window renders and accepts input under both Wayland and X11 Linux sessions.

## Assumptions

- The component inventory is frozen for this feature: parity means implementing the existing set, not adding new components. Components added later inherit the completeness obligation.
- Windows targets Windows 10 version 22H2 and later, using the platform's classic native control set, consistent with the framework's existing choice to bind the system toolkit directly rather than adopt a declarative UI markup layer.
- Linux targets GTK 4.8 and later, as fixed by the project constitution.
- macOS behaviour is the reference for event semantics but not for visual appearance: Windows and Linux are expected to look like Windows and Linux.
- Animation and transition parity are out of scope; only static appearance, layout, input, and accessibility must match.
- Mobile platforms remain out of scope.
- Continuous-integration runners capable of building and testing all three platforms are available, and Linux windowed tests may run headless or skip.
- The layout computation already produces platform-neutral geometry and requires no redesign for this feature.
