# Implementation Plan: React Native Component Parity + Shadcn Design System

**Branch**: `002-react-native-parity` | **Date**: 2026-11-02 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-react-native-parity/spec.md`

## Summary

Extend the `rn` ("Rust Native") cross-platform native UI framework with React Native-level component parity and a Shadcn-style design system. The existing Phase 1 core framework (component tree, reconciler, 15 widgets, AppKit backend) is extended to include: virtualized `FlatList`, `ScrollView`, `TouchableOpacity`, `SafeAreaView`, `ActivityIndicator`, `Modal`, `Popover`, `Switch`, `StatusBar`, rich styled `Text`, a design system with semantic tokens (color, spacing, radius, font, focus ring), `Card`, `Badge`, `Skeleton` components, and structural layout primitives (`Sidebar`, `SidebarItem`, `SegmentedControl`, `SearchBar`, `GroupedSection`, `SplitView`, `Toolbar`). The design system enables building macOS Settings-style and Mail-style apps natively on all three platforms.

## Technical Context

**Language/Version**: Rust stable (already established in workspace)

**Primary Dependencies**:
- `taffy` — flexbox layout engine (existing)
- `objc2` + `objc2-app-kit` — macOS AppKit bindings (existing)
- `windows` crate — Win32 controls (Windows backend)
- `gtk4` — GTK4 bindings (Linux backend)
- `crossbeam-channel` — event dispatch (existing)
- `log` — framework observability (existing)

**Storage**: N/A — no persistent storage

**Testing**: `cargo test`; reconciler tests remain headless. Platform renderer tests require a display server and run in platform CI jobs.

**Target Platform**: macOS 13+, Windows 10 22H2+, Linux (GTK 4.8+)

**Project Type**: Cargo workspace of library crates with example apps

**Performance Goals**: `FlatList` with 10,000 items scrolls at 60fps on minimum target hardware. `FlatList` uses ≤5MB memory regardless of item count. Layout and event dispatch within 8ms on minimum target hardware per constitution.

**Constraints**: No platform types in public API (`rn` crate). No `unwrap()`/`expect()` in library code. Consumer event handlers wrapped in `catch_unwind`. No `tracing` dep in v1. All three platforms must ship simultaneously, or new widgets are gated behind feature flags with a documented timeline per constitution.

**Scale/Scope**: ~17 new components + design system tokens (5 token types + Style builder), implemented across 3 platform backends.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

|| Principle | Gate | Status |
||-----------|------|--------|
|| I. Native-First, Per Platform | All new components use platform-native widgets (NSScrollView, NSTableView virtualized, NSButton variants, NSSegmentedControl, NSSearchField, NSSidebarViewController, NSSplitView, etc.). No custom-rendered substitutes where native controls exist. | ✅ PASS |
|| II. Stable, Trait-Based Public API | New components added as `ComponentKind` variants in `rn` crate. `Style` builder type is part of the `rn` public API. Backend crates remain internal. | ✅ PASS |
|| III. Type-Driven Correctness | Token types (`ColorToken`, `SpacingToken`, etc.) are enums/structs. `Style` builder is type-safe with method chaining. Platform types behind trait boundary. | ✅ PASS |
|| IV. Render Budget | FlatList virtualization constrained to constant live components. Layout within `taffy` budget. No per-frame allocations. | ✅ PASS |
|| V. Simplicity / YAGNI | No macro DSL. No reactive signals. Builder API only. Design system uses explicit token types, not dynamic CSS-like engine. | ✅ PASS |
|| VI. Declarative-Reconciler Architecture | All new components express UI as pure declarative trees. Reconciler handles diffs. Platform renderers apply changesets. Consumer holds no native handles. | ✅ PASS |

**Post-design re-check**: All gates pass. No violations. Complexity Tracking table not required.

## Project Structure

### Documentation (this feature)

```text
specs/002-react-native-parity/
├── plan.md              ← this file
├── spec.md
├── research.md          ← Phase 0 output (no research needed)
├── data-model.md        ← Phase 1 output
├── quickstart.md        ← Phase 1 output
├── contracts/
│   └── public-api.md    ← updated with new component APIs + design system
└── tasks.md             ← Phase 2 output (/speckit-tasks command)
```

### Source Code (repository root)

```text
Cargo.toml                    # workspace root

rn/                           # public API crate (platform-agnostic)
├── Cargo.toml
└── src/
    ├── lib.rs                # updated: new builder fns, Style builder, token types
    ├── component.rs          # updated: new ComponentKind variants
    ├── reconciler.rs         # [unchanged — extends naturally]
    ├── event.rs              # updated: new EventPayload variants
    └── renderer.rs           # [unchanged — defines PlatformRenderer trait]

rn-layout/                    # taffy-backed layout engine (shared)
├── Cargo.toml
└── src/
    └── lib.rs                # [unchanged — new components fit within taffy]

rn-macos/                     # macOS AppKit backend
├── Cargo.toml
└── src/
    ├── lib.rs
    └── renderer.rs           # updated: new component implementations

rn-windows/                   # Win32 backend
├── Cargo.toml
└── src/
    ├── lib.rs
    └── renderer.rs           # updated: Win32 implementations (scaffold → full)

rn-linux/                     # GTK4 backend
├── Cargo.toml
└── src/
    ├── lib.rs
    └── renderer.rs           # updated: GTK4 implementations (scaffold → full)

examples/
├── showcase/                 # updated: Settings-style + Mail-style demo sections
└── settings-demo/            # NEW: macOS Settings-style app
    ├── Cargo.toml
    └── src/main.rs
└── mail-demo/                # NEW: macOS Mail-style app
    ├── Cargo.toml
    └── src/main.rs

tests/
└── reconciler/
    └── tests.rs              # updated: new component scenarios
```

**Structure Decision**: Extend existing workspace structure. New components live in `rn/` (public API) and platform backend crates. Two new example apps demonstrate the Settings and Mail layouts. No new crates needed — existing `rn`, `rn-macos`, `rn-windows`, `rn-linux`, and `rn-layout` cover all concerns.

## Complexity Tracking

> No constitution violations. Table not required.

## Phase 0: Research

### Research Tasks

No `[NEEDS CLARIFICATION]` markers in spec. All technical decisions already established by Phase 1. Research confirms:

1. **FlatList virtualization**: Row-by-row with viewport calculation + component recycling pool. Standard pattern, no novel research needed.
2. **Design system token mapping**: Each token type maps to platform-native property (NSAppearance/Win32/DWM color APIs, padding/margin, corner radius). Straightforward per-platform implementation.
3. **SplitView divider**: Platform-native splitter widgets (NSSplitView, Win32 CSPLITTER, GtkPaned). All support drag, resize, animations.
4. **Skeleton shimmer**: Platform-native animations (CA on macOS, DWM on Windows, GTK CSS on Linux). All available on minimum target platforms.

**Research Status**: Complete. No new research required — proceed to Phase 1.

## Phase 1: Design & Contracts

### Data Model

*See data-model.md*

### Interface Contracts

*See contracts/public-api.md (updated)*

### Quickstart Validation Guide

*See quickstart.md*

## Phase 2: Tasks

*Deferred to `/speckit-tasks` command*
