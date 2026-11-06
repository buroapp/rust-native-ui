# Implementation Plan: Native App Foundation

**Branch**: `005-native-app-foundation` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-native-app-foundation/spec.md`

## Summary

Add the five capability areas an information-dense desktop application needs and the
framework currently lacks: a command and keyboard layer, data surfaces (table and
tree), anchored overlays, rich text editing, and application plumbing (forms, motion,
a headless test harness).

The technical theme that unifies them: **the framework gains data-bound components**.
Everything the framework renders today is built from a component subtree, one node per
visible thing. A 100,000-row table cannot be a component subtree, so rows must be data
the component owns, not nodes the reconciler owns. That single decision drives the
design of table, tree, forms and the test harness, and it is the one place this plan
knowingly diverges from uniform tree ownership (recorded in Complexity Tracking).

All five areas map onto native platform facilities rather than custom drawing, which is
what keeps the feature inside the constitution's native-first rule.

## Technical Context

**Language/Version**: Rust 2021, MSRV 1.75 (workspace `rust-version`)

**Primary Dependencies**: `objc2` 0.5 + `objc2-app-kit` 0.2 (already pinned; several new
feature flags required), `objc2-foundation` 0.2, `block2` 0.5, `taffy` 0.5 via `rn-layout`

**Storage**: N/A. All row, tree, form and document data is supplied in memory by the
application and owned for the lifetime of the component

**Testing**: `cargo test` across the workspace; the existing 60 unit tests in
`rn-layout`, `rn-core`, `rn` and `rn-macos` stay green. New work follows the spec's US8:
layout and state assertions through the reconciler, plus native accessibility assertions
against an offscreen window

**Target Platform**: macOS only becomes functional. Windows and Linux keep exhaustive
stub matches so the `completeness-guard` CI job continues to fail on any unhandled
component kind

**Project Type**: Rust library workspace (seven crates plus eight example apps)

**Performance Goals**: Every layout and event dispatch inside the constitution's 8ms
budget; 100,000-row table scrolls without a frame over budget; memory proportional to
visible rows, not total rows

**Constraints**: No allocation in the per-frame path; no `unwrap()`/`expect()` in library
code; no platform types in the public API; no hand-picked chrome metrics; every new
`ComponentKind` must satisfy the catalogue drift test and the stub completeness guard

**Scale/Scope**: 8 new component kinds, ~12 new pure-data model types, 3 delivery phases,
37 functional requirements, 15 success criteria

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Article | Gate | Result |
|---|---|---|
| I. Native-First | New surfaces must drive platform-native controls; no custom-drawn substitute where a native control exists | **PASS**. Table and tree drive the platform's table and outline controls, overlays use the platform popover and tooltip, the editor uses the platform text view, reduced motion reads the platform accessibility preference. No pixel drawing is introduced |
| II. Stable, Trait-Based API | Public API stays platform-agnostic and additive | **PASS**. New component kinds, builder functions and pure data models in `rn-core`. No platform type appears in any public signature; the native data source and delegate stay inside the macOS crate |
| III. Type-Driven Correctness | UI state as enums and structs, invalid states unrepresentable, no `unwrap`/`expect` in library code | **PASS**. Row identity, selection, sort descriptor, form validity and document state are modelled as types. Validation rules are a closed enum, so an unknown rule cannot be constructed |
| IV. Render Budget | 8ms layout and dispatch; no per-frame allocation | **PASS with a required design constraint**. Rows never become shadow nodes, so a 100,000-row table allocates one model and nothing per frame. Violating this would put 100,000 nodes through taffy and fail the budget outright |
| V. Simplicity and YAGNI | No abstraction until two independent backends or two use cases | **PASS**. Data binding is shared by table and tree (two consumers). Validation is shared by forms and the editor (two consumers). No generic "data source trait" is introduced beyond what those two consume |
| VI. Declarative-Reconciler Architecture | Component tree is the unit of ownership | **PASS WITH DOCUMENTED DEVIATION**. Data-bound components own their own row rendering outside the reconciler. Recorded in Complexity Tracking with justification and the rejected alternative |

No gate blocks Phase 0. One deviation is recorded rather than waived silently.

## Project Structure

### Documentation (this feature)

```text
specs/005-native-app-foundation/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── public-api.md    # Phase 1 output: additive public surface
├── checklists/
│   └── requirements.md  # from /speckit-specify
└── tasks.md             # /speckit-tasks output
```

### Source Code (repository root)

```text
rn-core/src/
├── lib.rs               # ComponentKind, PropKey, PropValue, EventKind
├── data/                # NEW: pure data models, no platform types
│   ├── mod.rs
│   ├── table.rs         # TableColumn, TableRow, TableModel, SortDescriptor
│   ├── tree.rs          # TreeNode, TreeModel
│   ├── command.rs       # Command, KeyBinding, CommandId
│   ├── form.rs          # FormField, ValidationRule, FieldValue, FormResult
│   └── overlay.rs       # MenuItem, OverlayPlacement
└── capabilities.rs      # per-backend completeness declaration

rn/src/
├── lib.rs               # builders: table, tree, editor, form, command set
├── component.rs         # builder methods for the new kinds
└── testing/             # NEW: headless session (US8)
    ├── mod.rs           # TestSession, queries, assertions
    └── input.rs         # synthetic pointer and keyboard input

rn-layout/src/
└── lib.rs               # unchanged; data-bound kinds declare fixed intrinsic height

rn-macos/src/
├── renderer.rs          # new arms: Table, Tree, Editor, Form, FormField
├── table.rs             # NEW: table data source and delegate
├── tree.rs              # NEW: tree data source and delegate
├── commands.rs          # NEW: command registry to main menu and key equivalents
├── overlays.rs          # NEW: anchored popover, tooltip, menu
└── motion.rs            # NEW: transition helper honouring reduced motion

rn-windows/src/renderer.rs  # extend assert_all_kinds_handled
rn-linux/src/renderer.rs    # extend assert_all_kinds_handled

examples/
└── table-demo/          # NEW: 100,000-row table, sortable columns, keyboard nav
```

**Structure Decision**: Keep the existing seven-crate split. New pure models live in
`rn-core/src/data/` so every backend can consume them without a platform dependency.
Native implementations go in new modules inside `rn-macos` rather than growing
`renderer.rs`, because that file is already ~3,400 lines and three of the five areas add
a data source or delegate that does not belong in a match arm. `rn-layout` stays
untouched: a data surface is one layout node with an intrinsic height, exactly as
`FlatList` is today.

## Complexity Tracking

> One deliberate deviation from Article VI. Everything else conforms.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Data-bound components own their row rendering outside the component tree | A 100,000-row table as component nodes means 100,000 shadow entries, 100,000 taffy nodes and 100,000 AppKit views. That fails the 8ms render budget by orders of magnitude before any row is drawn | Making rows component nodes and virtualizing inside the reconciler was considered. It requires the reconciler to support partial materialization, which is a far larger change to Article VI's core loop than one documented carve-out, and it would still allocate per row on every data change |
| New `rn-core` data module instead of new fields on `PropValue` | Commands, table models, tree models and form rules are structured data with invariants; flattening them into prop primitives loses the invariants and makes invalid states constructible | Reusing `PropValue::StringList` and similar was rejected because it cannot express a nullable sort direction, a validation rule, or a stable row identity without encoding them as strings |

## Phases

Delivery is phased so each phase is independently shippable and independently
verifiable. Phases map to spec priority, not to a preference.

| Phase | Contents | Spec stories | Exit condition |
|---|---|---|---|
| **A** | Commands, table, tree | US1, US2, US3 | Menu bar and shortcuts work; 100,000-row table scrolls in budget; tree preserves expansion across refresh |
| **B** | Anchored overlays, editor | US4, US5 | Popover/tooltip/menu anchor and dismiss correctly; editor passes the standard editing operations |
| **C** | Forms, motion, test harness | US6, US7, US8 | Form validation gates submission; reduced motion honoured; a screen renders and is driven in a test with no window |

Phase A is the minimum shippable slice. If work stops after A, the framework is
materially more capable than it is today and nothing is half-wired.

## Cross-Cutting Integration Obligations

These are not optional cleanups. Each one is a CI failure if missed when a new
`ComponentKind` is added.

1. **`ComponentKind::ALL`** must list every new kind, because `capabilities.rs` and the
   drift tests assert against it.
2. **`ComponentKind::COUNT`** must be updated to match, or the guard test fails.
3. **`rn-windows` and `rn-linux` `assert_all_kinds_handled`** must gain an arm per new
   kind, or the `completeness-guard` CI job fails by design (verified: it injects a
   throwaway variant and expects a non-exhaustive match).
4. **Storybook catalogue** must gain an entry per new kind, or
   `catalog_covers_every_component_kind` fails.
5. **`PropValue::reset_value`** in the reconciler must handle any new value variant.
6. **`docs/components/`** regenerates via `scripts/gen-docs.sh` once new kinds render.

## Risks

- **Native table view delegates.** `NSTableView` and `NSOutlineView` are pull-model data
  sources; the renderer is push-model. The sidebar's `NSOutlineView` work already proved
  this pattern works, including selection dispatch, which is the main de-risking here.
- **Main menu replacement.** Installing an `NSMenu` as the main menu is application-wide
  state. Two windows must not fight over it. Decision needed at implementation: the first
  window installs, later windows merge, or commands are declared once per application.
- **Editor undo stack.** Native undo arrives via the responder chain; standing up a
  working undo manager without per-frame allocation needs care.
- **Scope.** 37 requirements in three phases. Phase A alone is a substantial change to a
  crate that is currently one 3,400-line file.
