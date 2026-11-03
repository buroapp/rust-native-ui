# Phase 0 Research — Platform Parity

**Feature**: `003-platform-parity`
**Date**: 2026-11-03

Six unknowns were identified in the plan's Technical Context. Each is resolved below with the decision, the rationale, and what was rejected.

---

## R1. Win32 control class per component kind

**Decision**: Map each of the 57 kinds onto a specific Win32 class, using common controls (`comctl32`) where one exists. The mapping table lives in [data-model.md](./data-model.md#windows-control-mapping) and is normative for implementation.

The classes used are:

| Family | Win32 class |
|---|---|
| Static text, images, frames, hairlines | `STATIC` (`SS_LEFT`, `SS_BITMAP`, `SS_ETCHEDHORZ`, `SS_BLACKFRAME`) |
| Push, icon, cheque, radio, group-box, toggle-style buttons | `BUTTON` with the corresponding `BS_*` style |
| Single-line and multi-line text | `EDIT` (`ES_AUTOHSCROLL` / `ES_MULTILINE`) |
| Rich text | `RICHEDIT50W` from `Msftedit.dll` |
| Drop-downs | `COMBOBOX` (`CBS_DROPDOWNLIST`) |
| Slider, progress, spinner, level | `msctls_trackbar32`, `msctls_progress32`, `msctls_updown32` |
| Lists and sidebars | `SysListView32`, including `LVS_OWNERDATA` |
| Toolbar and status | `msctls_toolbar`, `msctls_statusbar32` |
| Tooltips | `tooltips_class32` |
| Date and time | `DATETIMEPICK_CLASS` (`DTS_TIMEFORMAT` for time-only) |
| Alerts | `TaskDialogIndirect` |
| Popovers, modals, sheets | `WS_POPUP` windows parented to the owner window |

**Rationale**: This is the standard, documented Win32 control set. Every entry is an OS-provided control, satisfying FR-001 and FR-013.

**Alternatives considered**:
- *WinUI 3 via the `windows` crate.* Rejected: it introduces a XAML runtime and a dependency on the Windows App SDK redistributable, and the constitution's principle I explicitly permits raw Win32 first. It would also make the backend structurally unlike the macOS one, which consumes a changeset against native handles.
- *Custom-drawn controls for the awkward cases.* Prohibited by FR-013.

### Components with no single Win32 control

Seven kinds have no one-to-one Win32 control. Each is composed from native primitives rather than custom-painted:

| Kind | Composition | Why this is not a violation |
|---|---|---|
| `Switch` | `BUTTON` with `BS_AUTOCHECKBOX` + `BS_PUSHLIKE` | Windows has no toggle-switch control; a push-like checkbox is the platform's own idiom for the same binary choice. FR-016 asks for the local idiom. |
| `SegmentedControl` | Row of `BUTTON` with `BS_AUTORADIOBUTTON \| BS_PUSHLIKE \| BS_GROUP` | Produces native segmented behaviour with native focus and keyboard handling. |
| `RatingIndicator` | Row of `BUTTON` `BS_AUTORADIOBUTTON` with star glyphs | Composed of native controls; no native star-rating control exists to substitute for. |
| `RelevanceIndicator` | `msctls_progress32` with `PBS_SMOOTH` | Native progress bar; a relevance meter is a progress meter with a different label. |
| `GraphicalTimePicker` | `DATETIMEPICK_CLASS` with `DTS_TIMEFORMAT` + `msctls_updown32` | Win32 has no clock-face picker. The platform idiom for picking a time is the DTP with time format. |
| `ColorWell` | `BUTTON` opening `ChooseColor` common dialog | Win32 has no inline colour-well control; a button that opens the OS colour picker is how Windows does colour selection. |
| `SplitView` | Two child windows + a `STATIC` divider with mouse capture | Win32 has no splitter control. Divider dragging uses standard window messages; no custom painting of content. |

The remaining `Card`, `Badge`, `Skeleton`, `GroupedSection`, `ContentArea`, `Scaffold`, `TitleBar`, `TouchableOpacity` and `SafeAreaView` are layout/chrome concepts rather than controls. They map to plain child windows, `SS_BLACKFRAME` statics, or window chrome (`WS_CAPTION`), which is the correct native expression.

---

## R2. GTK4 widget per component kind

**Decision**: Map onto stock GTK4 widgets, preferring libadwaita where GTK4 lacks a direct equivalent. Normative table in [data-model.md](./data-model.md#linux-widget-mapping).

GTK4 covers most kinds directly and better than Win32 does: `GtkSwitch`, `GtkSearchEntry`, `GtkPaned`, `GtkHeaderBar`, `GtkLevelBar`, `GtkColorButton`, `GtkListView` and `GtkListBox` all have exact counterparts.

**Rationale**: FR-002 and FR-013 require OS-provided widgets. GTK4's inventory is closer to the macOS set than Win32's, so fewer compositions are needed.

**Alternatives considered**:
- *Qt.* Rejected: the constitution fixes GTK 4.8+ as the Linux toolkit.
- *libadwaita as the primary dependency rather than GTK4 alone.* Rejected for the foundation, adopted selectively. `AdwPreferencesGroup` and `AdwNavigationSplitView` would map beautifully onto `GroupedSection` and `Sidebar`, but libadwaita pins a much narrower version range and drags in GNOME-specific theming that breaks on non-GNOME desktops. Stock GTK4 keeps the backend portable across desktops; libadwaita is used only where nothing else suffices.

### Components with no single GTK4 widget

| Kind | Composition | Note |
|---|---|---|
| `RatingIndicator` | Row of `GtkToggleButton` in a group | No native rating widget in GTK; native toggles composed. |
| `GraphicalTimePicker` | `GtkSpinButton` pair + `GtkCalendar`-style picker | GTK4 dropped the clock-face picker that GTK3 had. |
| `DatePicker` | `GtkCalendar` inside a `GtkPopover` behind a `GtkMenuButton` | GTK4 has no date-entry widget; this is the idiomatic GNOME composition. |
| `SegmentedControl` | Linked `GtkToggleButton` group with `linked` style class | The idiomatic GTK segmented control; `GtkStackSwitcher` requires a stack and is not a drop-in. |
| `Sheet` | Modal `GtkWindow` attached to the parent | GTK4 has no sheet concept; a transient modal window is the platform's equivalent. |
| `Card`, `Badge`, `Skeleton` | `GtkBox`/`GtkLabel` with CSS classes | Presentation classes, not controls. Themed by the desktop's stylesheet, not painted by us. |

---

## R3. Allocation-free render path with handle-based toolkits

**Decision**: Each backend keeps a pre-sized `Vec<Option<Handle>>` indexed by `NodeId.0`, grown geometrically and only when a node id exceeds current capacity. No `HashMap`, no per-frame allocation.

**Rationale**: Constitution IV prohibits allocation inside the layout and event-dispatch path. Both backends need node-id → native-handle lookup on every `apply`, and a `HashMap` would allocate on growth and incur hashing per lookup. A dense index is also faster because `NodeId` values are already dense and monotonic in this framework.

**Alternatives considered**:
- *`HashMap<NodeId, Handle>`* — the obvious choice, and what a first implementation would reach for. Rejected because of the allocation and hashing cost in the hot path, and because the reconciler already guarantees dense ids.
- *Slotmap* — extra dependency for no benefit given dense ids.
- *Arena with generation counters* — needed only if ids were reused; they are not in this architecture.

The vector is `Option`-wrapped so that deleted nodes leave a hole rather than shifting indices, and the reconciler's delete operation clears the slot.

---

## R4. Making completeness a compile-time error

**Decision**: Both backends `match` exhaustively on `ComponentKind` with no `_ =>` catch-all arm. Adding a variant to the enum without implementing it on a backend is a compile error on that backend.

Additionally, each backend exposes `pub(crate) fn capabilities() -> &'static [ComponentKind]` enumerating what it implements. A test asserts `capabilities().len()` equals the variant count, catching the case where a kind is matched but deliberately no-ops.

**Rationale**: FR-003 requires that missing implementations fail the build rather than degrade silently. Rust's exhaustiveness checking gives this for free — the current stubs pass CI precisely because they ignore the changeset instead of matching on it. Removing the catch-all is the entire fix.

**Alternatives considered**:
- *A `supported_kinds()` runtime check with a startup error.* Rejected: fails at runtime, which is exactly the silent-degradation mode FR-003 exists to prevent.
- *A build script that parses the enum and the backend.* Rejected: fragile, and duplicates what the type checker already does correctly.
- *A `#[non_exhaustive]`-style opt-out.* Rejected: it defeats the purpose.

**Consequence for FR-004**: with exhaustive matching there is no runtime gap for a component kind. FR-004's runtime diagnostic therefore applies only to components that a build deliberately excludes via `cfg` — for example a future minimal Linux build without `Msftedit`-equivalent optional libraries. The diagnostic is implemented as a `ComponentKind` → "unimplemented on this build" branch reachable only from such a `cfg` path.

---

## R5. Windowed tests without a display server

**Decision**: Linux windowed tests are gated on `GTK_A11Y`-independent display detection: the test harness checks for `WAYLAND_DISPLAY` or `DISPLAY` and, when both are absent, marks the test skipped with a printed reason rather than failing. CI provides `xvfb-run` for the X11 path so at least one windowed run happens on every PR; a Wayland run is a scheduled nightly job.

**Rationale**: SC-008 requires both Wayland and X11 to work, but an unprivileged container CI runner has neither. Silently passing would be dishonest; hard-failing would block every PR. Explicit skip plus a dedicated nightly matrix entry gives an honest signal on each.

**Alternatives considered**:
- *Require a compositor in CI for every run.* Rejected: heavy, slow, and flaky.
- *Skip silently.* Rejected: indistinguishable from a test that never ran.
- *Test only under Xvfb and never Wayland.* Rejected: leaves the Wayland-specific window-decoration and global-menu paths entirely unverified, and those are where the real differences are.

Windows tests run on the standard GitHub-hosted Windows runner, which has an interactive session available to the test process, so no equivalent gating is needed there.

---

## R6. DPI and scaling

**Decision**:
- **Windows**: the process declares per-monitor DPI awareness v2 via the application manifest. Layout is computed in logical units and the backend multiplies positions and sizes by the monitor's scale factor when applying frames. Scale-factor changes (`WM_DPICHANGED`) trigger a full re-apply of the last layout result at the new factor.
- **Linux**: GTK4 reports integer scale factor via `GdkSurface::scale_factor` and fractional scale via `GskRenderer`. Logical-unit layout is passed through unchanged; GTK applies scaling during rendering. Fractional scaling therefore needs no backend arithmetic — but font size and minimum sizes must be specified in logical units only, never device pixels.

**Rationale**: FR-015 and the high-DPI edge case in the spec. The two platforms differ fundamentally: Windows expects the application to scale, GTK expects the toolkit to scale. Trying to normalise that difference into one code path would be worse than handling it per platform, which is what Principle I asks for anyway.

**Alternatives considered**:
- *Pre-scale layout by passing a scaled container size into taffy.* Rejected: it changes the computed geometry, breaking SC-003's cross-platform agreement within two logical pixels. Layout must stay in logical units on every platform.
- *Declare system-DPI-aware on Windows and ignore per-monitor.* Rejected: produces blurry text when the window moves between monitors of different scale, which is the common laptop-plus-external-display case.

---

## Summary of decisions

| # | Decision | Rejected alternative that a reasonable engineer would try first |
|---|---|---|
| R1 | Win32 common controls, 7 kinds composed from native primitives | WinUI 3, custom painting |
| R2 | Stock GTK4, libadwaita only where necessary | libadwaita throughout, Qt |
| R3 | Dense `Vec<Option<Handle>>` indexed by node id | `HashMap`, slotmap, arena with generations |
| R4 | Exhaustive `match`, no catch-all | Runtime capability check, build script |
| R5 | Skip windowed tests when no display; Xvfb in CI, Wayland nightly | Require compositor always, skip silently |
| R6 | Windows scales in the backend; GTK scales in the toolkit; layout stays logical | Pre-scale layout input |

No NEEDS CLARIFICATION markers remained in the Technical Context after this research; no constitution gate failed; no Complexity Tracking entry is required.
