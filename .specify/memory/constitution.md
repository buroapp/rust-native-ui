<!--
SYNC IMPACT REPORT
==================
Version change: 2.0.0 → 2.1.0 (MINOR — new architectural principle added)
Reason for MINOR bump:
  - Added Principle VI: Declarative-Reconciler Architecture
    Defines the React Native model explicitly: single declarative component tree
    reconciled against native controls by a per-platform renderer backend.
    This is additive; existing principles unchanged.
Modified principles: none
Added sections:
  - Principle VI: Declarative-Reconciler Architecture (new)
Removed sections: none
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ no structural changes required
  - .specify/templates/spec-template.md ✅ no structural changes required
  - .specify/templates/tasks-template.md ✅ no structural changes required
Deferred TODOs: none
-->

# Hermes Studio Constitution

## Core Principles

### I. Native-First, Per Platform

The framework MUST drive platform-native OS widgets on each supported target:
- **macOS**: AppKit via `cacao` or direct `objc2` bindings
- **Windows**: Win32 controls or WinUI 3 via `windows-rs`
- **Linux**: GTK 4 / libadwaita via `gtk4-rs`

Custom-rendered widgets are PROHIBITED as a substitute for a native control that
exists on the target platform. Custom widgets are only permitted for interactions
that have no native equivalent on a given OS, and MUST be documented as exceptions
in the plan's Complexity Tracking table.

The framework's public API MUST be platform-agnostic (single trait surface). Platform
backend selection happens at compile time via Cargo features (`feature = "macos"`,
`feature = "windows"`, `feature = "linux"`). No runtime platform detection is used
for control dispatch.

**Rationale**: The framework's entire value proposition is native fidelity across all
three major desktop platforms. Any deviation from native controls undermines the
reason this project exists.

### II. Stable, Trait-Based Public API

All widget types and layout primitives MUST be expressed as Rust traits with a single
canonical implementation per platform backend. The public API surface is versioned
independently from internal backend code following semver. Breaking changes to public
traits require a MAJOR version bump of the framework crate.

Components MUST be composable and self-contained. Cross-component communication goes
through declared event channels or owned sub-trees only — no shared mutable state
across component boundaries.

**Rationale**: Framework consumers pin to a public API. Internal backend rewrites
(e.g., switching macOS backend from cacao to raw objc2) MUST NOT break downstream
crates if the trait contract is unchanged.

### III. Type-Driven Correctness

UI state MUST be modelled as algebraic data types (enums + structs). Invalid UI states
MUST be unrepresentable at the type level. `unwrap()` and `expect()` are PROHIBITED in
all library code; all fallibility surfaces as `Result` or `Option` at the call site.

Platform-specific types (e.g., `NSView`, `HWND`, `GtkWidget`) MUST be encapsulated
behind the trait boundary and MUST NOT leak into the public API.

**Rationale**: Framework consumers must not need to import platform crates or handle
platform types to use hermes-studio. Type safety and API hygiene are non-negotiable
for a library that others build on.

### IV. Render Budget

Each platform backend MUST complete layout and event dispatch within 8ms on the
minimum target hardware:
- macOS: Apple Silicon M1 baseline
- Windows: mid-range Intel 12th-gen (e.g., i5-12400)
- Linux: mid-range AMD Ryzen 5 (e.g., 5600G)

Allocations inside the render/layout path are PROHIBITED. All per-frame data
structures MUST be pre-allocated or arena-backed. Any feature that cannot meet this
budget MUST defer work to a background thread via message passing.

**Rationale**: 60fps requires ~16ms total. Reserving 8ms for the framework leaves
headroom for application logic, OS compositing, and background work.

### V. Simplicity and YAGNI

No abstraction layer is introduced until the same pattern appears in at least two
independent platform backends or two independent consumer use cases. Traits, generics,
and macro-based DSLs MUST be justified by demonstrated need, not anticipated reuse.

The working single-platform implementation ships first; the cross-platform abstraction
follows once the second backend is integrated and the seams are observable.

**Rationale**: Cross-platform abstraction in Rust accumulates lifetime and trait-bound
complexity quickly. Starting concrete and extracting later is always cheaper than
designing the abstraction in advance of knowing what it needs to cover.

### VI. Declarative-Reconciler Architecture

The framework MUST implement the React Native model adapted to Rust:

```
Consumer code (declarative)
        │
        ▼
  Virtual Widget Tree        ← platform-agnostic enums/structs
        │
        ▼
    Reconciler               ← diffs old tree vs new tree; emits a changeset
        │
        ▼
  Platform Renderer          ← per-platform backend; applies changeset to real controls
        │
        ▼
 Native OS controls          ← NSView / HWND / GtkWidget
```

**Rules:**
- Consumer code MUST express UI as a pure, immutable declarative tree — a Rust value,
  no side effects. State changes produce a new tree; the framework handles diffing.
- The reconciler MUST be platform-agnostic and independently testable without a
  display server or OS.
- Platform renderers MUST only receive and apply a minimal changeset (create / update /
  delete / reorder). They MUST NOT re-walk the full tree on every update.
- Consumer code MUST NOT hold references to native control handles. All interaction
  with native controls goes through the framework's event and command channels.

**Rationale**: This is the core architectural insight the framework is built on. The
declarative layer is what makes "write once" possible; the reconciler is what keeps it
performant; the per-platform renderer is what makes it genuinely native rather than
just rendered. Conflating any two of these three layers in implementation is the
canonical failure mode of cross-platform UI frameworks — keep them strictly separated.

## Platform Targets

| Platform | Minimum Version | Backend Crate       | Feature Flag          |
|----------|-----------------|---------------------|-----------------------|
| macOS    | 13.0 (Ventura)  | `cacao` / `objc2`   | `feature = "macos"`   |
| Windows  | Windows 10 22H2 | `windows-rs`        | `feature = "windows"` |
| Linux    | GTK 4.8+        | `gtk4-rs`           | `feature = "linux"`   |

All three platforms are first-class targets. No platform is treated as a port or
afterthought. CI MUST run on all three platforms for every PR that touches the
public API or any backend.

Mobile (iOS, Android) is explicitly out of scope for v1. Any PR that introduces
mobile-platform targeting MUST include a constitution amendment proposal.

## Development Workflow

- All features begin with a spec (`/speckit-specify`) before any code is written.
- Plans (`/speckit-plan`) MUST include a Constitution Check gate before Phase 0 research.
- New widgets MUST ship implementations for all three backends simultaneously, or be
  gated behind a `#[cfg(feature)]` with a documented timeline for the missing backends.
- Reconciler changes MUST include a test against the platform-agnostic reconciler unit
  suite (no OS required). Render-path changes MUST include before/after frame-time
  measurements on at least two platforms in the PR.
- Breaking changes to public traits MUST be flagged `[BREAKING]` in the PR title and
  include a migration guide.
- The `main` branch MUST build on macOS, Windows, and Linux at all times.

## Governance

This constitution supersedes all other project conventions. Amendments require:
1. A written rationale explaining what principle is changing and why.
2. An update to this document with an incremented version (semver rules apply).
3. A review of all open specs and plans for compliance impact.
4. A migration note for any in-flight features affected.

All implementation plans MUST pass the Constitution Check before entering Phase 1.
Complexity exceptions (e.g., custom-rendered widget, single-platform-only feature,
reconciler bypass) MUST be logged in the plan's Complexity Tracking table with a
simpler alternative considered and rejected.

**Version**: 2.1.0 | **Ratified**: 2026-11-02 | **Last Amended**: 2026-11-02
