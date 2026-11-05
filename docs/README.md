# rn documentation

`rn` is a Rust native UI framework with React Native-style components, rendered
with real OS controls. On macOS that means AppKit: `NSButton`, `NSTextField`,
`NSSwitch`, `NSOutlineView`, `NSToolbar`, `NSSplitViewController` — not
custom-drawn imitations.

## Contents

- **[Component catalog](components/README.md)** — every component with a live
  screenshot and the command to open it. Generated from the `storybook` example.
- **[Architecture](ARCHITECTURE.md)** — crate graph, layout model, the
  AppKit-owns-chrome / taffy-owns-interior split. Interactive version:
  [architecture.html](architecture.html).
- **[Backend status](backend-status.md)** — what works per platform.
- **[Benchmarks](benchmarks.md)** — FlatList virtualization and layout numbers.

## Crates

| Crate | Role |
|---|---|
| `rn` | Public builder API — the only crate an app depends on. |
| `rn-core` | Component kinds, prop keys, design tokens. No platform types. |
| `rn-layout` | Flexbox layout over taffy. |
| `rn-macos` | AppKit renderer. |
| `rn-windows`, `rn-linux` | Stubs; not yet implemented. |

## Running the examples

```sh
cargo run -p storybook     --features macos      # component catalog
cargo run -p mail-demo     --features macos      # three-pane mail app
cargo run -p settings-demo --features macos      # System Settings-style
cargo run -p finder-demo   --features macos      # Finder-style file grid
```

Storybook deep-links to a component: `cargo run -p storybook --features macos -- --select Switch`.

## Regenerating component docs

```sh
scripts/gen-docs.sh
```

Opens each component in `storybook`, captures its window natively
(`screencapture` by CG window id), and writes `docs/components/`. The catalog is
the single source of truth — `storybook --list` dumps it, and a drift test fails
if a component kind has no catalog entry.
