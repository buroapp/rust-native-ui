# Command (Menu Bar & Keyboard Shortcuts)

Declarative application menu bar and keyboard shortcut system. Attach a `CommandSet` to any window to install the macOS menu bar and wire key equivalents.

## Usage

```rust
use rn::{command, window, CommandPlacement, CommandSet, KeyBinding, Modifiers};

let print_id = rn::HandlerId::new();
rn::on(print_id, |_| println!("Print triggered"));

let mut set = CommandSet::new();
let mut cmd = command(1, "Print…", print_id);
cmd.shortcut = Some(KeyBinding {
    key: "p".to_owned(),
    modifiers: Modifiers::COMMAND,
});
cmd.placement = CommandPlacement { menu: "File".to_owned(), order: 30 };
set.push(cmd);

let w = window().commands(set).build();
```

## Command fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | `CommandId` | Stable identity |
| `title` | `String` | Menu item label |
| `shortcut` | `Option<KeyBinding>` | Optional ⌘/⇧/⌥/⌃ key binding |
| `available` | `AvailabilityRule` | `Always`, `WhenFlag(id)`, or `Never` |
| `placement` | `CommandPlacement` | `menu` name + `order` within that menu |
| `handler` | `HandlerId` | Dispatched when the item is activated |

## Modifiers

```rust
Modifiers::COMMAND   // ⌘
Modifiers::SHIFT     // ⇧
Modifiers::OPTION    // ⌥
Modifiers::CONTROL   // ⌃
// combine: Modifiers::COMMAND | Modifiers::SHIFT
```

## AvailabilityRule

- `Always` — item enabled unconditionally.
- `Never` — item grayed out unconditionally (useful for unimplemented actions).
- `WhenFlag(FlagId)` — dynamic; evaluated by the application (Phase B).

## Platform notes

- macOS: installs `NSMenu` / `NSMenuItem` as the application main menu bar. Key equivalents and first-responder dispatch follow standard AppKit semantics.
- Windows/Linux: stub (Phase A scope is macOS only).
