// Phase A: Native App Foundation — pure data models, no platform types
// Shared identity newtypes live here; submodules import them via super::.

pub mod command;
pub mod table;
pub mod tree;

// Re-export everything consumers need from one place.
pub use command::{AvailabilityRule, Command, CommandPlacement, CommandSet, KeyBinding, Modifiers,
    ShortcutConflict};
pub use table::{Selection, SortDescriptor, SortDirection, TableCell, TableColumn, TableModel,
    TableRow, TextAlign};
pub use tree::{TreeModel, TreeNode};

// Shared identity newtypes ------------------------------------------------

/// Stable column identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(pub u64);

/// Stable row identity. Shared by table and tree so state restoration is uniform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowId(pub u64);

/// Stable form field identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub u64);

/// Application flag that drives command availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlagId(pub u64);

/// Stable command identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u64);
