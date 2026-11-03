# Research: React Native Component Parity + Shadcn Design System

**Feature**: `002-react-native-parity` | **Phase**: 0

## Status: Complete — No Open Questions

The spec contained zero `[NEEDS CLARIFICATION]` markers. All technical decisions carry over from Phase 1 or are resolvable from prior knowledge.

---

## Decision Log

### 1. FlatList Virtualization Strategy

**Decision**: Row-by-row virtualization with a fixed-size component recycle pool. Items outside the visible viewport + buffer are unmounted; items entering the viewport are mounted from the pool (re-using native view objects).

**Rationale**: Simpler than the full React Native `VirtualizedList` (which supports variable heights, sections, sticky headers). Row-by-row covers 95% of the use case at far lower complexity, consistent with Principle V (YAGNI).

**Alternatives considered**:
- Windowed rendering with absolute Y positions — rejected: more complex, same performance on desktop where item heights are predictable
- Native `NSTableView` row virtualization — accepted as the macOS backend mechanism, but the framework layer still manages the recycle pool at the `rn` crate level for cross-platform consistency

---

### 2. Design System Token Approach

**Decision**: Explicit Rust enum token types (`ColorToken`, `SpacingToken`, etc.), not a dynamic CSS-like property map or a HashMap of strings.

**Rationale**: Type safety. The Rust compiler enforces valid token values at compile time. Consumers get autocomplete. No runtime parsing. Consistent with Principle III (Type-Driven Correctness).

**Alternatives considered**:
- CSS variable string injection (like Shadcn web) — rejected: non-type-safe, hard to map to native widget properties, requires a CSS parser dep
- A `HashMap<String, PropValue>` bag — rejected: loses type safety, adds runtime errors

---

### 3. Skeleton Shimmer Implementation

**Decision**: `CAGradientLayer` animation on macOS (hardware-accelerated). Fallback to simple opacity pulse on platforms without CA support.

**Rationale**: CA layers are the standard macOS approach for shimmer effects — used by Xcode, Mail, and Photos. Matches platform convention. No custom rendering required.

**Alternatives considered**:
- OpenGL/Metal custom shader — rejected: overkill, violates Principle V, requires unsafe
- Solid non-animated placeholder — rejected: fails SC-007 which requires shimmer

---

### 4. SplitView Implementation

**Decision**: Wrap platform-native splitter widgets. `NSSplitView` on macOS, `GtkPaned` on Linux, CSplitter on Win32. The `rn` crate only stores pane sizes and emits `SplitResize` events; the platform renderer drives the native widget.

**Rationale**: Native splitters have built-in drag handles, animation, and keyboard accessibility that would require significant custom work to replicate. Consistent with Principle I (Native-First).

---

### 5. SafeAreaView Inset Calculation

**Decision**: Insets are queried at render time from the platform (not hardcoded). macOS: `NSWindow.contentLayoutGuide.layoutFrame.origin.y`. Win32: `GetSystemMetrics(SM_CYCAPTION)`. Linux: query `GdkMonitor` workarea vs window bounds.

**Rationale**: Hardcoded values (e.g. "title bar is always 36px") break on platforms with custom title bars, accessibility settings, or window chrome overrides.
