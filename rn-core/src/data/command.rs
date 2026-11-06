use super::{CommandId, FlagId};
use crate::HandlerId;

/// Modifier keys for keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(pub u8);
impl Modifiers {
    pub const COMMAND:  Modifiers = Modifiers(0b0001);
    pub const SHIFT:    Modifiers = Modifiers(0b0010);
    pub const OPTION:   Modifiers = Modifiers(0b0100);
    pub const CONTROL:  Modifiers = Modifiers(0b1000);
    pub fn contains(self, other: Modifiers) -> bool { self.0 & other.0 == other.0 }
}
impl std::ops::BitOr for Modifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Modifiers(self.0 | rhs.0) }
}

/// A keyboard shortcut attached to a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub key: String,
    pub modifiers: Modifiers,
}

/// When a command is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvailabilityRule {
    Always,
    WhenFlag(FlagId),
    Never,
}

/// Where in the menu bar a command lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlacement {
    pub menu: String,
    pub order: u32,
}

impl Default for CommandPlacement {
    fn default() -> Self { CommandPlacement { menu: "View".to_owned(), order: 100 } }
}

/// One application command.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub id: CommandId,
    pub title: String,
    pub shortcut: Option<KeyBinding>,
    pub available: AvailabilityRule,
    pub handler: HandlerId,
    pub placement: CommandPlacement,
}

/// The set of commands declared on a window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommandSet(pub Vec<Command>);

impl CommandSet {
    pub fn new() -> Self { Self::default() }
    pub fn push(&mut self, cmd: Command) { self.0.push(cmd); }
    pub fn commands(&self) -> &[Command] { &self.0 }
}
