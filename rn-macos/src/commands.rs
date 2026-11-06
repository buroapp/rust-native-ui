// rn-macos/src/commands.rs — Phase A: command and keyboard layer

use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSMenu, NSMenuItem,
    NSCommandKeyMask, NSShiftKeyMask, NSAlternateKeyMask, NSControlKeyMask,
    NSEventModifierFlags,
};
use objc2_foundation::{MainThreadMarker, NSString};
use rn_core::data::{AvailabilityRule, Command, CommandSet, Modifiers};

/// Install the application main menu from a `CommandSet`.
///
/// Scans for duplicate `(key, modifiers)` pairs and prints a warning for each
/// conflict (FR-006). `targets` is a slice of `(target_obj, command)` pairs.
pub(crate) fn install_main_menu_with_targets(
    mtm: MainThreadMarker,
    _command_set: &CommandSet,
    targets: &[(&AnyObject, &Command)],
) {
    let app = NSApplication::sharedApplication(mtm);
    // FR-006: detect duplicate shortcuts before wiring.
    {
        let mut seen: std::collections::HashMap<(String, u8), &str> = std::collections::HashMap::new();
        for (_, cmd) in targets {
            if let Some(binding) = &cmd.shortcut {
                let key = (binding.key.clone(), binding.modifiers.0);
                if let Some(existing) = seen.get(&key) {
                    log::warn!(
                        "ShortcutConflict: '{}' and '{}' share the same binding ({}{:?})",
                        existing, cmd.title, binding.key, binding.modifiers
                    );
                    eprintln!(
                        "[rn] ShortcutConflict: '{}' and '{}' share binding ⌘{}",
                        existing, cmd.title, binding.key
                    );
                } else {
                    seen.insert(key, &cmd.title);
                }
            }
        }
    }
    let bar = make_menu(mtm, "");

    // App-name submenu (macOS requires it as the first item).
    add_bare_submenu(mtm, &bar, "App");

    // Group by menu name.
    let mut groups: std::collections::BTreeMap<&str, Vec<(&AnyObject, &Command)>> =
        std::collections::BTreeMap::new();
    for (obj, cmd) in targets {
        groups.entry(cmd.placement.menu.as_str()).or_default().push((*obj, cmd));
    }
    for cmds in groups.values_mut() {
        cmds.sort_by_key(|(_, c)| c.placement.order);
    }

    for (menu_name, entries) in &groups {
        let submenu = make_menu(mtm, menu_name);
        for (target_obj, cmd) in entries {
            let item = make_menu_item(mtm, cmd, target_obj);
            submenu.addItem(&item);
        }
        let parent = bare_item(mtm, menu_name);
        parent.setSubmenu(Some(&submenu));
        bar.addItem(&parent);
    }

    app.setMainMenu(Some(&bar));
}

fn make_menu(mtm: MainThreadMarker, title: &str) -> Retained<NSMenu> {
    unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(title)) }
}

fn add_bare_submenu(mtm: MainThreadMarker, bar: &NSMenu, title: &str) {
    let item = bare_item(mtm, title);
    item.setSubmenu(Some(&make_menu(mtm, title)));
    bar.addItem(&item);
}

fn bare_item(mtm: MainThreadMarker, title: &str) -> Retained<NSMenuItem> {
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            None,
            &NSString::from_str(""),
        )
    }
}

fn make_menu_item(
    mtm: MainThreadMarker,
    cmd: &Command,
    target: &AnyObject,
) -> Retained<NSMenuItem> {
    let key = cmd.shortcut.as_ref().map_or("", |k| k.key.as_str());
    let item: Retained<NSMenuItem> = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(&cmd.title),
            Some(sel!(fire:)),
            &NSString::from_str(key),
        )
    };
    if let Some(binding) = &cmd.shortcut {
        item.setKeyEquivalentModifierMask(modifiers_to_ns(binding.modifiers));
    }
    let enabled = cmd.available != AvailabilityRule::Never;
    unsafe {
        item.setEnabled(enabled);
        item.setTarget(Some(target));
        item.setAction(Some(sel!(fire:)));
    }
    item
}

pub(crate) fn modifiers_to_ns(m: Modifiers) -> NSEventModifierFlags {
    let mut f = NSEventModifierFlags(0);
    if m.contains(Modifiers::COMMAND)  { f.0 |= NSCommandKeyMask.0; }
    if m.contains(Modifiers::SHIFT)    { f.0 |= NSShiftKeyMask.0; }
    if m.contains(Modifiers::OPTION)   { f.0 |= NSAlternateKeyMask.0; }
    if m.contains(Modifiers::CONTROL)  { f.0 |= NSControlKeyMask.0; }
    f
}

// ---------------------------------------------------------------------------
// FR-003/FR-004: Dynamic WhenFlag availability
// ---------------------------------------------------------------------------

thread_local! {
    static FLAG_REGISTRY: std::cell::RefCell<std::collections::HashMap<rn_core::data::FlagId, bool>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Set a flag value so `WhenFlag(id)` availability rules update on the next menu open.
/// Call from application code before showing the window's menu.
pub fn set_command_flag(id: rn_core::data::FlagId, value: bool) {
    FLAG_REGISTRY.with(|m| { m.borrow_mut().insert(id, value); });
}

/// Query current flag value (true if not yet registered).
pub(crate) fn flag_is_enabled(id: rn_core::data::FlagId) -> bool {
    FLAG_REGISTRY.with(|m| *m.borrow().get(&id).unwrap_or(&true))
}
