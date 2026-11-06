// rn-macos/src/commands.rs — Phase A: command and keyboard layer

use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::sel;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSMenu, NSMenuItem,
    NSCommandKeyMask, NSShiftKeyMask, NSAlternateKeyMask, NSControlKeyMask,
    NSEventModifierFlags,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};
use rn_core::data::{AvailabilityRule, Command, CommandSet, Modifiers};

/// Install the application main menu from a `CommandSet`.
///
/// Scans for duplicate `(key, modifiers)` pairs and returns any
/// conflicts (FR-006/SC-001). `targets` is a slice of `(target_obj, command)` pairs.
pub(crate) fn install_main_menu_with_targets(
    mtm: MainThreadMarker,
    _command_set: &CommandSet,
    targets: &[(&AnyObject, &Command)],
) -> Vec<rn_core::data::ShortcutConflict> {
    let app = NSApplication::sharedApplication(mtm);
    // FR-006: detect duplicate shortcuts and return conflicts to the caller.
    let mut conflicts: Vec<rn_core::data::ShortcutConflict> = Vec::new();
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
                    conflicts.push(rn_core::data::ShortcutConflict {
                        a: existing.to_string(),
                        b: cmd.title.clone(),
                        key: binding.key.clone(),
                        modifiers: binding.modifiers,
                    });
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
        // Collect WhenFlag items for the menu delegate.
        let mut flag_items: Vec<(Retained<NSMenuItem>, rn_core::data::FlagId)> = Vec::new();
        for (target_obj, cmd) in entries {
            let item = make_menu_item(mtm, cmd, target_obj);
            if let rn_core::data::AvailabilityRule::WhenFlag(id) = cmd.available {
                flag_items.push((Retained::clone(&item), id));
            }
            submenu.addItem(&item);
        }
        // FR-003/FR-004: attach MenuOpenDelegate so enabled state updates before display.
        if !flag_items.is_empty() {
            let delegate = MenuOpenDelegate::new(mtm, flag_items);
            unsafe {
                submenu.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            }
            // Keep the delegate alive by stashing on the submenu (NSMenu retains its delegate weakly).
            // We'll leak it deliberately — the app menu lives for the lifetime of the app.
            std::mem::forget(delegate);
        }
        let parent = bare_item(mtm, menu_name);
        parent.setSubmenu(Some(&submenu));
        bar.addItem(&parent);
    }

    app.setMainMenu(Some(&bar));
    conflicts
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
// FR-003/FR-004: NSMenuDelegate to update WhenFlag item enabled state on open
// ---------------------------------------------------------------------------

use objc2_app_kit::NSMenuDelegate;

/// A simple menu delegate that re-evaluates `WhenFlag` enabled state before
/// the menu appears, so visual greying reflects the current flag state.
#[allow(dead_code)]
pub(crate) struct MenuDelegateIvars {
    /// Pairs of (NSMenuItem, FlagId) for items that have `WhenFlag` availability.
    items: Vec<(Retained<NSMenuItem>, rn_core::data::FlagId)>,
}

declare_class!(
    pub(crate) struct MenuOpenDelegate;

    unsafe impl ClassType for MenuOpenDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnMenuOpenDelegate";
    }

    impl DeclaredClass for MenuOpenDelegate {
        type Ivars = std::cell::UnsafeCell<Vec<(Retained<NSMenuItem>, rn_core::data::FlagId)>>;
    }

    unsafe impl NSObjectProtocol for MenuOpenDelegate {}

    unsafe impl NSMenuDelegate for MenuOpenDelegate {
        #[method(menuWillOpen:)]
        unsafe fn menu_will_open(&self, _menu: &NSMenu) {
            let items = &*self.ivars().get();
            for (item, flag_id) in items {
                let enabled = flag_is_enabled(*flag_id);
                item.setEnabled(enabled);
            }
        }
    }
);

impl MenuOpenDelegate {
    fn new(
        mtm: MainThreadMarker,
        flag_items: Vec<(Retained<NSMenuItem>, rn_core::data::FlagId)>,
    ) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(std::cell::UnsafeCell::new(flag_items));
        unsafe { msg_send_id![super(this), init] }
    }
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
