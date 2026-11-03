# Implementation Plan: Platform Parity — Windows and Linux Backends

**Branch**: `main` | **Date**: 2026-11-03 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-platform-parity/spec.md`

## Summary

The framework renders 57 component kinds natively on macOS and nothing on Windows or Linux — those two backend crates are 21-line stubs that return an error from the event loop. This plan delivers working Win32 and GTK4 renderers behind the existing `PlatformRenderer` trait, which needs no changes.

Two decisions shape the sequencing:

1. **Backends ship sequentially, not interleaved.** Windows completes before Linux starts. Principle V of the constitution makes the same argument in the abstract — the working implementation ships first and the seams become observable before the next one is built. Interleaving by component group would keep both backends perpetually half-done and neither demo runnable.

2. **Completeness is enforced by the compiler, not by a checklist.** Each backend matches exhaustively on `ComponentKind`, so adding a component kind without implementing it on every backend is a compile error. This costs nothing and makes the current failure mode — silent stub drift — structurally impossible.

No public API change. No reconciler change. No layout-engine change.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.75, pinned by the workspace

**Primary Dependencies**:
- Windows backend: `windows` crate (Win32 `UI`/`Graphics`/`Foundation` feature set)
- Linux backend: `gtk4` crate (GTK 4.8+), plus `glib`/`gio` re-exported by it
- Shared: `taffy` 0.5 for layout (unchanged), `log` 0.4 for diagnostics

**Storage**: N/A — no persistence in this feature

**Testing**: `cargo test` per crate; reconciler and layout unit tests remain OS-free and run on all three platforms; backend tests are `#[cfg(target_os)]`-gated and windowed tests skip when no display server is present

**Target Platform**: Windows 10 22H2+ (`windows-rs`, Win32 common controls); Linux GTK 4.8+ under both Wayland and X11

**Project Type**: Cargo workspace — library framework (desktop UI toolkit) with example applications

**Performance Goals**: 8 ms per backend for layout application plus event dispatch, measured on the constitution's baseline hardware (mid-range Intel 12th-gen for Windows, Ryzen 5 5600G for Linux)

**Constraints**:
- No allocation inside the render/apply path (constitution IV)
- No `unwrap`/`expect`/`panic` in library code (constitution III)
- No platform type (`HWND`, `GtkWidget`, …) in the public `rn` API (constitution III)
- Exactly one backend active per build (constitution I)

**Scale/Scope**: 57 component kinds × 2 new backends = 114 control mappings. Six example applications must run unmodified on all three platforms.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|---|---|---|
| **I. Native-First, Per Platform** | Every component maps to a native OS control; no custom-rendered substitutes; backend selected by Cargo feature, no runtime dispatch | **PASS** — this feature exists to satisfy it. Custom-rendering is prohibited by FR-013. |
| **II. Stable, Trait-Based Public API** | No breaking changes to public traits; components self-contained | **PASS** — `PlatformRenderer` is unchanged. FR-005 and FR-006 require zero application changes and zero new public types. |
| **III. Type-Driven Correctness** | ADT state, no `unwrap`/`expect` in library code, no platform types in public API | **PASS** — FR-021. Exhaustive `match` on `ComponentKind` is itself a type-level guarantee. Backend crates stay internal. |
| **IV. Render Budget** | 8 ms layout + event dispatch per backend, no allocation in render path | **PASS with obligation** — must be measured and reported, not assumed. Carried as explicit tasks per backend; pre-allocated control maps required. |
| **V. Simplicity and YAGNI** | No abstraction before a second backend exists | **PASS** — the second and third backends are this feature. Abstraction extracted *after* observing macOS remains the correct order; the plan adds no new trait beyond the existing one. |
| **VI. Declarative-Reconciler** | Renderers receive a minimal changeset only; never re-walk the tree | **PASS** — FR-008 and FR-010. Both new backends consume `Changeset` exactly as macOS does. |
| **Platform Targets** | All three first-class; CI on all three | **PASS** — FR-020 delivers the CI matrix. |

**No violations. Complexity Tracking table is not required for this feature.**

One item is worth naming explicitly even though it is not a violation: the constitution's Development Workflow requires that "new widgets MUST ship implementations for all three backends simultaneously, or be gated behind `#[cfg(feature)]` with a documented timeline". This feature closes that debt for the existing 57 kinds. Going forward, exhaustive matching is what enforces it.

## Project Structure

### Documentation (this feature)

```text
specs/003-platform-parity/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 — technology decisions
├── data-model.md        # Phase 1 — capability model and control mapping registry
├── quickstart.md        # Phase 1 — cross-platform validation guide
├── contracts/
│   └── backend-contract.md   # Phase 1 — what a conforming backend must do
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 output (/speckit-tasks — not created here)
```

### Source Code (repository root)

```text
rn-core/                    # shared types — unchanged by this feature
├── src/lib.rs              # ComponentKind, PropKey, PropValue, Changeset, PlatformRenderer
└── tests/                  # OS-free reconciler + type tests

rn-layout/                  # taffy-backed layout — unchanged by this feature
└── src/lib.rs

rn/                         # public API — unchanged public surface
├── Cargo.toml              # features: macos | windows | linux
└── src/
    ├── lib.rs              # builder functions
    ├── reconciler.rs
    └── flat_list.rs

rn-macos/                   # reference implementation — unchanged
└── src/renderer.rs

rn-windows/                 # NEW WORK
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── app.rs              # window creation, message pump, DPI handling
    ├── renderer.rs         # Changeset application + exhaustive ComponentKind match
    ├── capabilities.rs     # supported-kind table used by coverage tests
    └── controls/
        ├── mod.rs          # dispatch to per-group modules
        ├── basic.rs        # label, button, text input, checkbox, separator
        ├── ranged.rs       # slider, progress, stepper, level indicators
        ├── choice.rs       # dropdown, popup, segmented, radio, switch
        ├── containers.rs   # layout containers, card, grouped section, sidebar
        ├── lists.rs        # list tile, flat list virtualization, scroll
        ├── chrome.rs       # window, title bar, toolbar, split view
        └── dialogs.rs      # modal, alert, sheet, popover, tooltip

rn-linux/                   # NEW WORK
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── app.rs              # GtkApplication, Wayland/X11, dark-mode tracking
    ├── renderer.rs         # Changeset application + exhaustive ComponentKind match
    ├── capabilities.rs
    └── widgets/
        ├── mod.rs
        ├── basic.rs
        ├── ranged.rs
        ├── choice.rs
        ├── containers.rs
        ├── lists.rs
        ├── chrome.rs
        └── dialogs.rs

examples/                   # unchanged source — must build and run everywhere
├── hello-world/
├── counter/
├── gallery/
├── showcase/
├── settings-demo/
└── mail-demo/

.github/workflows/
└── ci.yml                  # NEW — three-platform build + test matrix
```

**Structure Decision**: The existing workspace layout is retained exactly. Both backend crates already exist as workspace members with the correct feature wiring in `rn/Cargo.toml`; this feature fills them in. Within each backend, components are grouped into seven modules by interaction family rather than one file per component, because 57 files per backend would obscure the repetitive shape of the mapping and make the exhaustive match hard to audit. The macOS backend stays a single file — restructuring it is not required by this feature and Principle V argues against doing it speculatively.

## Phase 0: Research

See [research.md](./research.md). Unknowns resolved there:

1. Which Win32 control class backs each of the 57 kinds, and which have no faithful equivalent (needed to size the Complexity Tracking entries if any arise).
2. Which GTK4 widget backs each kind, and how to express macOS-only chrome concepts idiomatically.
3. How to keep the render path allocation-free when both toolkits are handle-based and require per-node lookup.
4. How to make completeness a compile-time error rather than a runtime surprise.
5. How to run windowed tests in CI for Linux without a display server.
6. How Windows per-monitor DPI and GTK fractional scaling interact with logical-unit layout.

## Phase 1: Design

See the generated artifacts:

- [data-model.md](./data-model.md) — capability registry, control record, mapping tables
- [contracts/backend-contract.md](./contracts/backend-contract.md) — what a conforming backend must guarantee
- [quickstart.md](./quickstart.md) — cross-platform validation procedure

## Implementation Sequencing

Sequential, Windows first. Each stage is independently demonstrable.

| Stage | Content | Demonstrable outcome |
|---|---|---|
| A | Windows foundation: window, message pump, Changeset application, DPI | An empty window opens and closes cleanly on Windows |
| B | Windows basic + ranged + choice controls | Counter and gallery examples run on Windows |
| C | Windows containers, lists, chrome | settings-demo and mail-demo run on Windows |
| D | Windows dialogs + accessibility + render-budget measurement | Full Windows parity; SC-003 and SC-007 measured |
| E | Linux foundation: GtkApplication, Wayland/X11, dark-mode tracking | An empty window opens and closes cleanly on Linux |
| F | Linux basic + ranged + choice widgets | Counter and gallery examples run on Linux |
| G | Linux containers, lists, chrome | settings-demo and mail-demo run on Linux |
| H | Linux dialogs + accessibility + render-budget measurement | Full Linux parity |
| I | CI matrix, coverage enforcement, cross-platform geometry assertions | SC-004, SC-005, SC-006 verified continuously |

Stage I is scheduled last because the matrix is only meaningful once there is something on both sides of it to fail. The workflow file is written earlier if it becomes useful sooner, but its assertions activate at I.

## Complexity Tracking

*Constitution Principle I permits a custom widget only where no native control exists, and requires each such case to be logged here. Research (R1, R2) confirmed ten kinds have no single native control on at least one platform. All are compositions of native controls — not custom painting — which is the permitted form, but they are logged because the principle asks for the record.*

| Kind | Platform | Why no native control | Simpler alternative rejected because |
|---|---|---|---|
| `Switch` | Windows | Win32 has no toggle-switch control | Custom-painting an iOS-style switch: prohibited by FR-013 and would look foreign on Windows. `BS_AUTOCHECKBOX \| BS_PUSHLIKE` gives the platform's own binary-choice affordance with native keyboard and focus behaviour. |
| `SegmentedControl` | Windows | No segmented-control class | `SysTabControl32` is tabs, a different information scent. `BS_AUTORADIOBUTTON \| BS_PUSHLIKE` in a group is how Win32 itself expresses a horizontal exclusive choice. |
| `SplitView` | Windows | No splitter control | Third-party splitter or custom-drawn divider. Divider is a `STATIC` with mouse capture; the panes are unmodified child windows, so no content is custom-rendered. |
| `ColorWell` | Windows | No inline colour-well control | Custom colour swatch + popup. `BUTTON` opening the OS `ChooseColor` dialog is how Windows applications select colour and keeps the picker native. |
| `RatingIndicator` | Windows, Linux | No star-rating control on either | Custom-drawn stars. A row of native toggle buttons with star glyphs keeps input, focus, and theming native. |
| `GraphicalTimePicker` | Windows, Linux | Win32 has no clock face; GTK4 removed the one GTK3 had | Custom clock-face drawing. `DATETIMEPICK_CLASS` with `DTS_TIMEFORMAT` (Windows) and spin-button pair with calendar popover (Linux) are each platform's own time idiom. |
| `SegmentedControl` | Linux | No segmented control; `GtkStackSwitcher` requires a stack | `GtkStackSwitcher` would force an artificial stack. Linked `GtkToggleButton` group is the documented GTK idiom for the same control. |
| `DatePicker` | Linux | GTK4 has no date-entry widget | Custom calendar rendering. `GtkMenuButton` + `GtkPopover` + `GtkCalendar` uses GTK's own calendar. |
| `Sheet` | Windows, Linux | Neither platform has the macOS sheet concept | Emulating a macOS sheet would violate FR-016. A modal owned window is each platform's own idiom for the same "blocking child of this window" interaction. |
| `Card`, `Badge`, `Skeleton` | Windows, Linux | Presentation containers, not controls | Drawing them. They use native static/box widgets with the desktop's own theme classes, so appearance follows the OS theme rather than this framework. |

**Test applied to every row**: the widget must not paint its own content where an OS control could express it. Each composition above delegates drawing, focus, keyboard handling, and theming to native controls. None reimplements a control the platform already provides.

**Not violations**: `TitleBar`, `Toolbar`, `Sidebar`, `ContentArea`, `Scaffold`, `GroupedSection`, `TouchableOpacity`, `SafeAreaView`. These are layout and chrome concepts, expressed with each platform's native container or window chrome (`WS_CAPTION`, `GtkHeaderBar`), which is exactly what FR-016 asks for.

