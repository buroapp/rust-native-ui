# Backend Status

v0.1 ships a working macOS AppKit backend and compile-only scaffolds for
Windows and Linux. The scaffolds satisfy the cross-platform build matrix in
`.github/workflows/ci.yml` so `cargo check` and `cargo clippy` stay green on
every CI target from day 1; they do not open windows.

| Widget             | macOS (`rn-macos`) | Windows (`rn-windows`) | Linux (`rn-linux`) |
|--------------------|--------------------|------------------------|--------------------|
| `Window`           | ✅ `NSWindow` | ❌ scaffold | ❌ scaffold |
| `Label`            | ✅ `NSTextField` (label mode) | ❌ scaffold | ❌ scaffold |
| `Button`           | ✅ `NSButton` + target/action | ❌ scaffold | ❌ scaffold |
| `TextInput`        | ✅ `NSTextField` + delegate | ❌ scaffold | ❌ scaffold |
| `LayoutContainer`  | ✅ `NSView` + taffy layout | ❌ scaffold | ❌ scaffold |

Events wired on macOS:

| Event | Source | Delivery |
|-------|--------|----------|
| `Press` | `NSButton` target/action | `rn::dispatch_handler` |
| `Change` | `NSTextFieldDelegate::controlTextDidChange:` | `rn::dispatch_handler` with `EventPayload::Text` |
| `Close` | `NSWindowDelegate::windowShouldClose:` | `rn::dispatch_handler` |

Backend `run_event_loop` behaviour:

| Platform | Behaviour |
|----------|-----------|
| macOS    | `NSApplication::run()` + 60Hz `NSTimer` that drains `rn::drain_updates()` and pending button events each tick |
| Windows  | Returns `RenderError::OsError("windows backend not implemented in v0.1")` |
| Linux    | Returns `RenderError::OsError("linux backend not implemented in v0.1")` |

## Target milestones for parity

- **v0.2**: Windows backend using `windows` crate (Win32 USER32 controls).
  Scope: Window, Label (`STATIC`), Button, TextInput (`EDIT`),
  LayoutContainer (child window + taffy).
- **v0.3**: Linux backend using `gtk4` (`ApplicationWindow`, `Label`,
  `Button`, `Entry`, `Box` for layout).

Each milestone must demonstrate the same hello-world and counter examples
running natively before being released.
