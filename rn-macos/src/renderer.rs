//! macOS AppKit renderer (US1). Real native-control implementation using
//! `objc2` and `objc2-app-kit`.
//!
//! Design:
//! - `MacosRenderer::new` captures a `MainThreadMarker` and inits NSApplication.
//! - `apply(Changeset)` walks ops in order, routing to the per-op helpers.
//! - Each `Create` instantiates the right native class and stores a strong
//!   reference under its `NodeId` in `node_map`.
//! - Button clicks are routed through a `declare_class!` trampoline that
//!   reads an ivar carrying the `HandlerId` and calls `rn_core::dispatch_event`.
//! - TextInput `OnChange` uses an NSTextFieldDelegate trampoline (same shape).
//! - Windows wire `windowShouldClose:` to the OnClose handler via an
//!   NSWindowDelegate trampoline.
//! - A periodic `NSTimer` runs the tick bridge each frame to drain
//!   `update_sender` messages from background threads.
//! - All renderer state lives in `Rc<RefCell<RendererState>>` so an
//!   `ApplyBridge` installed in `rn-core` can push changesets back in from
//!   inside the running event loop.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{declare_class, msg_send_id, mutability, sel, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle, NSButton,
    NSColor, NSColorWell, NSControlTextEditingDelegate, NSControlStateValueOff, NSControlStateValueOn,
    NSDatePicker, NSDatePickerStyle, NSImageView, NSLineBreakMode, NSPopUpButton, NSProgressIndicator,
    NSScrollView, NSSlider, NSTextField, NSTextFieldDelegate,
    NSAutoresizingMaskOptions, NSAlert, NSTabView, NSTabViewItem, NSToolbar, NSToolbarItem,
    NSToolbarItemIdentifier, NSToolbarDelegate, NSToolbarDisplayMode, NSWindowToolbarStyle, NSTextView, NSStepper, NSView, NSWindow, NSWindowDelegate,
    NSWindowStyleMask, NSWindowTitleVisibility,
    NSBox, NSBoxType, NSDatePickerElementFlags, NSDatePickerMode, NSImage,
    NSLevelIndicator, NSLevelIndicatorStyle, NSSearchField,
    NSSegmentedControl, NSSwitch,
    NSTitlePosition,
    NSSplitViewController, NSSplitViewItem, NSViewController,
    NSOutlineView, NSOutlineViewDataSource, NSOutlineViewDelegate,
    NSTableColumn, NSTableView,
    NSTableViewStyle,
    NSTitlebarAccessoryViewController,
    NSLayoutAttribute,
};
// Phase 2 extra imports for RichText attributed string rendering (FR-008)
use objc2_app_kit::{NSFont, NSFontManager, NSForegroundColorAttributeName, NSFontAttributeName};
use objc2_foundation::NSMutableAttributedString;
use objc2_foundation::{
    MainThreadMarker, NSArray, NSDate, NSEdgeInsets, NSIndexSet, NSInteger, NSNumber, NSUInteger,
    NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString, NSTimer,
};

use rn_core::{
    RadiusToken,
    ApplyBridge, Axis, Changeset, ColorValue, ComponentKind, Event, EventKind, EventPayload,
    HandlerId, NodeId, Operation, PlatformRenderer, PropKey, PropValue, Props, RenderError,
    ScrollEvent,
};
use rn_layout::{compute_layout, LayoutAxis, LayoutNodeId, LayoutSpec, Rect};

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

enum NativeHandle {
    Window(Retained<NSWindow>),
    View(Retained<NSView>, ContainerInfo),
    Label(Retained<NSTextField>),
    Button(Retained<NSButton>),
    TextInput(Retained<NSTextField>),
    // New widget types
    Checkbox(Retained<NSButton>),
    Slider(Retained<NSSlider>),
    ProgressBar(Retained<NSProgressIndicator>),
    Separator(Retained<NSBox>),
    TextArea(Retained<NSScrollView>),  // NSScrollView wrapping NSTextView
    Dropdown(Retained<NSPopUpButton>),
    Image(Retained<NSImageView>),
    Stepper(Retained<NSView>),         // NSView wrapping NSStepper + NSTextField
    ColorWell(Retained<NSColorWell>),
    DatePicker(Retained<NSDatePicker>),
    RadioGroup(Retained<NSView>),      // NSView wrapping NSMatrix of radio buttons
    // Phase 2 — RN parity + design system
    ScrollViewWidget(Retained<NSScrollView>),
    FlatListWidget(Retained<NSScrollView>, Box<FlatListRendererState>),
    ActivityIndicatorWidget(Retained<NSProgressIndicator>),
    SwitchWidget(Retained<NSSwitch>),
    CardWidget(Retained<NSBox>),
    BadgeWidget(Retained<NSTextField>),
    SkeletonWidget(Retained<NSView>),
    SegmentedControlWidget(Retained<NSSegmentedControl>),
    SearchBarWidget(Retained<NSSearchField>),
    SidebarWidget(Retained<NSView>),
    GroupedSectionWidget(Retained<NSBox>),
    SplitViewWidget(Retained<NSView>),
    ToolbarWidget(Retained<NSView>),
    RichTextWidget(Retained<NSTextView>),
    TouchableWidget(Retained<NSView>),
    SafeAreaWidget(Retained<NSView>),
    // Batch 1 — macOS layout primitives
    ListTileWidget(Retained<NSView>),
    ScaffoldWidget(Retained<NSView>),
    TitleBarWidget(Retained<NSView>),
    // Batch 2 — button family
    IconButtonWidget(Retained<NSButton>),
    DisclosureButtonWidget(Retained<NSButton>),
    HelpButtonWidget(Retained<NSButton>),
    PopupButtonWidget(Retained<NSPopUpButton>),
    BackButtonWidget(Retained<NSButton>),
    // Batches 3-6
    CapacityIndicatorWidget(Retained<NSLevelIndicator>),
    RatingIndicatorWidget(Retained<NSLevelIndicator>),
    RelevanceIndicatorWidget(Retained<NSLevelIndicator>),
    TooltipWidget(Retained<NSTextField>),
    TimePickerWidget(Retained<NSDatePicker>),
    GraphicalTimePickerWidget(Retained<NSDatePicker>),
    ToolbarIconButtonWidget(Retained<NSButton>),
    ToolbarSpacerWidget(Retained<NSView>),
    ToolbarDividerWidget(Retained<NSBox>),
    ToolbarPopupWidget(Retained<NSPopUpButton>),
    AlertDialogWidget(Retained<NSView>),
    SheetWidget(Retained<NSView>, Retained<NSWindow>),
    TabViewWidget(Retained<NSTabView>),
    /// Row container, its selection pill, and its label.
    SidebarItemWidget(Retained<NSView>),
    /// Phase A: data-bound surfaces (NSScrollView wrapping NSTableView / NSOutlineView)
    TableWidget(Retained<NSScrollView>),
    TreeWidget(Retained<NSScrollView>),
}

impl NativeHandle {
    fn as_view(&self) -> Option<&NSView> {
        match self {
            NativeHandle::Window(_) => None,
            NativeHandle::View(v, _) => Some(v),
            NativeHandle::Label(v) => Some(v),
            NativeHandle::Button(v) => Some(v),
            NativeHandle::TextInput(v) => Some(v),
            NativeHandle::Checkbox(v) => Some(v),
            NativeHandle::Slider(v) => Some(v),
            NativeHandle::ProgressBar(v) => Some(v),
            NativeHandle::Separator(v) => Some(v),
            NativeHandle::TextArea(v) => Some(v),
            NativeHandle::Dropdown(v) => Some(v),
            NativeHandle::Image(v) => Some(v),
            NativeHandle::Stepper(v) => Some(v),
            NativeHandle::ColorWell(v) => Some(v),
            NativeHandle::DatePicker(v) => Some(v),
            NativeHandle::RadioGroup(v) => Some(v),
            // Phase 2
            NativeHandle::ScrollViewWidget(v) => Some(v),
            NativeHandle::FlatListWidget(v, _) => Some(v),
            NativeHandle::ActivityIndicatorWidget(v) => Some(v),
            NativeHandle::SwitchWidget(v) => Some(v),
            NativeHandle::CardWidget(v) => Some(v),
            NativeHandle::BadgeWidget(v) => Some(v),
            NativeHandle::SkeletonWidget(v) => Some(v),
            NativeHandle::SegmentedControlWidget(v) => Some(v),
            NativeHandle::SearchBarWidget(v) => Some(v),
            NativeHandle::SidebarWidget(v) => Some(v),
            NativeHandle::TableWidget(v) => Some(v),
            NativeHandle::TreeWidget(v) => Some(v),
            NativeHandle::GroupedSectionWidget(v) => Some(v),
            NativeHandle::SplitViewWidget(v) => Some(v),
            NativeHandle::ToolbarWidget(v) => Some(v),
            NativeHandle::RichTextWidget(v) => Some(v),
            NativeHandle::TouchableWidget(v) => Some(v),
            NativeHandle::SafeAreaWidget(v) => Some(v),
            NativeHandle::ListTileWidget(v) => Some(v),              
            NativeHandle::ScaffoldWidget(v) => Some(v),              
            NativeHandle::TitleBarWidget(v) => Some(v),              
            NativeHandle::IconButtonWidget(v) => Some(v),            
            NativeHandle::DisclosureButtonWidget(v) => Some(v),      
            NativeHandle::HelpButtonWidget(v) => Some(v),            
            NativeHandle::PopupButtonWidget(v) => Some(v),           
            NativeHandle::BackButtonWidget(v) => Some(v),            
            NativeHandle::CapacityIndicatorWidget(v) => Some(v),
            NativeHandle::RatingIndicatorWidget(v) => Some(v),
            NativeHandle::RelevanceIndicatorWidget(v) => Some(v),
            NativeHandle::TooltipWidget(v) => Some(v),
            NativeHandle::TimePickerWidget(v) => Some(v),
            NativeHandle::GraphicalTimePickerWidget(v) => Some(v),
            NativeHandle::ToolbarIconButtonWidget(v) => Some(v),
            NativeHandle::ToolbarSpacerWidget(v) => Some(v),
            NativeHandle::ToolbarDividerWidget(v) => Some(v),
            NativeHandle::ToolbarPopupWidget(v) => Some(v),
            NativeHandle::AlertDialogWidget(v) => Some(v),
            NativeHandle::SheetWidget(v, _) => Some(v),
            NativeHandle::TabViewWidget(v) => Some(v),
            NativeHandle::SidebarItemWidget(v) => Some(v),
        }
    }
}

/// Minimal per-FlatList virtualization state kept inside the macOS renderer.
/// Full logic lives in `rn::flat_list::FlatListState`; this mirrors the
/// minimum needed for the macOS backend tick loop (FR-002, FR-004).
struct FlatListRendererState {
    item_h: f32,
    viewport_h: f32,
    scroll_offset: f32,
    buffer_rows: usize,
    end_reach_threshold: f32,
    item_count: usize,
    mounted_start: usize,
    mounted_end: usize,
    end_reached_fired: bool,
}

impl FlatListRendererState {
    fn new(item_h: f32, viewport_h: f32, buffer_rows: usize, end_reach_threshold: f32) -> Self {
        Self {
            item_h,
            viewport_h,
            scroll_offset: 0.0,
            buffer_rows,
            end_reach_threshold,
            item_count: 0,
            mounted_start: 0,
            mounted_end: 0,
            end_reached_fired: false,
        }
    }

    /// Returns `(new_start, new_end)` for the desired visible+buffer range.
    fn desired_range(&self) -> (usize, usize) {
        if self.item_h <= 0.0 || self.item_count == 0 {
            return (0, 0);
        }
        let first = (self.scroll_offset / self.item_h) as usize;
        let rows_in_view = (self.viewport_h / self.item_h).ceil() as usize + 1;
        let start = first.saturating_sub(self.buffer_rows);
        let end = (first + rows_in_view + self.buffer_rows).min(self.item_count);
        (start, end)
    }

    /// Update scroll offset. Returns list of (data_index, is_mount) for rows
    /// that need mounting (`true`) or unmounting (`false`).
    fn on_scroll(&mut self, offset: f32) -> Vec<(usize, bool)> {
        self.scroll_offset = offset.max(0.0);
        let total_h = self.item_count as f32 * self.item_h;
        if !self.end_reached_fired
            && total_h - self.scroll_offset - self.viewport_h < self.end_reach_threshold
        {
            self.end_reached_fired = true;
        }
        let (new_start, new_end) = self.desired_range();
        let mut ops = Vec::new();
        // Unmount rows leaving the window
        for i in self.mounted_start..self.mounted_end {
            if i < new_start || i >= new_end {
                ops.push((i, false));
            }
        }
        // Mount new rows entering the window
        for i in new_start..new_end {
            if i < self.mounted_start || i >= self.mounted_end {
                ops.push((i, true));
            }
        }
        self.mounted_start = new_start;
        self.mounted_end = new_end;
        ops
    }
}

struct ContainerInfo {
    #[allow(dead_code)]
    axis: Axis,
    padding: f32,
    gap: f32,
}

struct ShadowNode {
    kind: ComponentKind,
    props: Props,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl ShadowNode {
    fn new(kind: ComponentKind, props: Props, parent: Option<NodeId>) -> Self {
        Self {
            kind,
            props,
            parent,
            children: Vec::new(),
        }
    }
}

/// Panes a split-hosted window owns: each pane's node id and its AppKit view.
type SplitPanes = Vec<(NodeId, Retained<NSView>)>;

struct RendererState {
    mtm: MainThreadMarker,
    nodes: HashMap<NodeId, NativeHandle>,
    shadow: HashMap<NodeId, ShadowNode>,
    windows: Vec<NodeId>,
    pending_events: Vec<Event>,
    _tick_timer: Option<Retained<NSTimer>>,
    /// Sheets already presented, so a relayout cannot present the same sheet twice.
    presented_sheets: std::cell::RefCell<std::collections::HashSet<NodeId>>,
    /// A window whose root is a SplitView with a leading Sidebar is hosted by an
    /// NSSplitViewController rather than laid out into one content view. This maps
    /// each such window to the pane views it owns, keyed by the pane's node id, so
    /// relayout can run taffy inside each pane independently.
    split_panes: HashMap<NodeId, SplitPanes>,
    /// Content inset each Sidebar's scroll view needs so its outline rows
    /// start below a pinned search/account/family header band. apply_rects
    /// re-asserts this on every relayout — without it, repeatedly setting
    /// the scroll view's own frame (every single rerender touches it, even
    /// when the rect hasn't changed) could make AppKit's safe-area/inset
    /// machinery silently recompute and drop our manual override, which
    /// only ever ran once at promotion time. Found via rapid-selection
    /// stress testing: the account/family band stayed visually pinned, but
    /// the outline's own rows started scrolling up underneath it — a
    /// one-shot setContentInsets that nothing ever re-applied.
    sidebar_top_insets: HashMap<NodeId, f64>,
}

impl RendererState {
    fn new(mtm: MainThreadMarker) -> Self {
        Self {
            mtm,
            nodes: HashMap::new(),
            shadow: HashMap::new(),
            windows: Vec::new(),
            pending_events: Vec::new(),
            _tick_timer: None,
            presented_sheets: std::cell::RefCell::new(std::collections::HashSet::new()),
            split_panes: HashMap::new(),
            sidebar_top_insets: HashMap::new(),
        }
    }
}

thread_local! {
    static STATE_SLOT: RefCell<Option<Rc<RefCell<RendererState>>>> = const { RefCell::new(None) };
}

/// Walk the shadow tree parent chain and fire `OnPress` handlers on any
/// `TouchableOpacity` ancestor until `stop_bubbling` is set or the root is
/// reached.  Implements FR-015 event bubbling semantics.
fn bubble_press_event(source: NodeId) {
    let ancestors = with_state(|s| {
        let mut result = Vec::new();
        let mut current = s.shadow.get(&source).and_then(|sn| sn.parent);
        while let Some(parent_id) = current {
            if let Some(sn) = s.shadow.get(&parent_id) {
                if sn.kind == ComponentKind::TouchableOpacity {
                    if let Some(PropValue::HandlerRef(h)) = sn.props.get(&PropKey::OnPress) {
                        result.push((parent_id, *h));
                    }
                }
                current = sn.parent;
            } else {
                break;
            }
        }
        result
    });
    for (node_id, handler) in ancestors.unwrap_or_default() {
        let bubble_event = Event {
            source: node_id,
            handler,
            kind: EventKind::Press,
            payload: EventPayload::None,
            stop_bubbling: false,
        };
        // Dispatch; consumer can set stop_bubbling = true to halt chain.
        rn_core::dispatch_event(bubble_event);
    }
}

/// Aliased to avoid clippy's "very complex type" lint.
type FlatOpsVec = Vec<(NodeId, HandlerId, Vec<(usize, bool)>)>;

fn with_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut RendererState) -> R,
{
    STATE_SLOT.with(|slot| {
        let state = slot.borrow().as_ref().cloned();
        state.map(|s| {
            let mut b = s.borrow_mut();
            f(&mut b)
        })
    })
}

// ---------------------------------------------------------------------------
// Public renderer
// ---------------------------------------------------------------------------

pub struct MacosRenderer {
    state: Rc<RefCell<RendererState>>,
}

struct GlobalApplyBridge;

// SAFETY: the raw pointer is only ever dereferenced on the main thread. We
// enforce that upstream via `MainThreadGuard::check` in `rn::rerender`. The
// Mutex only exists to satisfy `Send + Sync` for `OnceLock`.
unsafe impl Send for RawStatePtr {}
unsafe impl Sync for RawStatePtr {}

struct RawStatePtr(*mut RendererState);

static GLOBAL_STATE: OnceLock<Mutex<RawStatePtr>> = OnceLock::new();

impl ApplyBridge for GlobalApplyBridge {
    fn apply(&self, changeset: Changeset) -> Result<(), RenderError> {
        let Some(mutex) = GLOBAL_STATE.get() else {
            return Err(RenderError::OsError("renderer state not initialised".into()));
        };
        let Ok(guard) = mutex.lock() else {
            return Err(RenderError::OsError("renderer state mutex poisoned".into()));
        };
        let ptr = guard.0;
        if ptr.is_null() {
            return Err(RenderError::OsError("renderer state detached".into()));
        }
        // SAFETY: main-thread guarantee enforced upstream.
        let state = unsafe { &mut *ptr };
        apply_changeset(state, changeset)
    }
}

impl MacosRenderer {
    pub fn new() -> Result<Self, RenderError> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Err(RenderError::WrongThread);
        };

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        let state = Rc::new(RefCell::new(RendererState::new(mtm)));
        let raw: *mut RendererState = state.as_ptr();
        let mutex = GLOBAL_STATE.get_or_init(|| Mutex::new(RawStatePtr(std::ptr::null_mut())));
        if let Ok(mut g) = mutex.lock() {
            g.0 = raw;
        }
        rn_core::set_apply_bridge(Box::new(GlobalApplyBridge));

        STATE_SLOT.with(|slot| {
            *slot.borrow_mut() = Some(Rc::clone(&state));
        });

        log::debug!("MacosRenderer::new initialised NSApplication");
        Ok(Self { state })
    }
}

impl PlatformRenderer for MacosRenderer {
    fn apply(&mut self, changeset: Changeset) -> Result<(), RenderError> {
        let mut s = self.state.borrow_mut();
        apply_changeset(&mut s, changeset)
    }

    fn run_event_loop(&mut self) -> Result<(), RenderError> {
        let mtm = self.state.borrow().mtm;
        let app = NSApplication::sharedApplication(mtm);

        let target = TickTarget::new(mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                1.0 / 60.0,
                &target,
                sel!(fire:),
                None,
                true,
            )
        };
        // Keep both target and timer alive.
        stash_delegate(NodeId(u64::MAX - 1), DelegateHandle::Tick(target));
        self.state.borrow_mut()._tick_timer = Some(timer);

        // Force an immediate layout pass once the event loop is about to start.
        // `makeKeyAndOrderFront` was called during `render()` before the event
        // loop started, so `windowDidBecomeKey:` never fires as a transition;
        // the contentView frame is now valid so we relayout everything here.
        // Promote split windows, then lay out. Promotion calls
        // `setContentViewController`, which makes AppKit lay the window out
        // synchronously and re-enter our state; so it must run WITHOUT a borrow held.
        {
            let roots: Vec<NodeId> = self.state.borrow().windows.clone();
            for root in &roots {
                promote_split_window(&self.state, *root);
            }
            let s = self.state.borrow();
            for root in roots {
                relayout_window(&s, root);
            }
        }

        unsafe { app.activate() };
        unsafe { app.run() };
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Changeset application
// ---------------------------------------------------------------------------

fn apply_changeset(state: &mut RendererState, changeset: Changeset) -> Result<(), RenderError> {
    for op in changeset.operations {
        apply_op(state, op)?;
    }
    let roots: Vec<NodeId> = state.windows.clone();
    for root in roots {
        relayout_window(state, root);
    }
    Ok(())
}

fn apply_op(state: &mut RendererState, op: Operation) -> Result<(), RenderError> {
    match op {
        Operation::Create {
            parent_id,
            node_id,
            kind,
            props,
        } => create_node(state, parent_id, node_id, kind, props),
        Operation::Update {
            node_id,
            changed_props,
        } => update_node(state, node_id, changed_props),
        Operation::Delete { node_id } => delete_node(state, node_id),
        Operation::Reorder {
            parent_id,
            child_order,
        } => reorder_children(state, parent_id, child_order),
    }
}

fn create_node(
    state: &mut RendererState,
    parent_id: Option<NodeId>,
    node_id: NodeId,
    kind: ComponentKind,
    props: Props,
) -> Result<(), RenderError> {
    let mtm = state.mtm;
    match kind {
        ComponentKind::Window => {
            let (w, h) = size_from_props(&props, 400.0, 300.0);
            let rect = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(w as f64, h as f64));
            let modern = bool_prop(&props, PropKey::ModernLook, false);
            let mut style = NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Miniaturizable;
            if modern {
                style |= NSWindowStyleMask::FullSizeContentView;
            }
            let window: Retained<NSWindow> = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc(),
                    rect,
                    style,
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                )
            };
            if let Some(PropValue::String(title)) = props.get(&PropKey::Title) {
                let ns_title = NSString::from_str(title);
                window.setTitle(&ns_title);
            }
            if modern {
                // The modern macOS look: content runs under a transparent titlebar.
                window.setTitlebarAppearsTransparent(true);
                window.setTitleVisibility(NSWindowTitleVisibility::NSWindowTitleHidden);
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnClose) {
                let delegate = WindowDelegate::new(mtm, *h, node_id);
                let proto: &ProtocolObject<dyn NSWindowDelegate> =
                    ProtocolObject::from_ref(&*delegate);
                window.setDelegate(Some(proto));
                stash_delegate(node_id, DelegateHandle::Window(delegate));
            }
            // Phase A: command and keyboard layer — install the application main menu.
            if let Some(PropValue::CommandSet(command_set)) = props.get(&PropKey::Commands) {
                let mut targets: Vec<Retained<ButtonTarget>> = Vec::new();
                let pairs: Vec<(Retained<ButtonTarget>, _)> = command_set
                    .commands()
                    .iter()
                    .map(|cmd| (ButtonTarget::with_availability(
                        mtm, cmd.handler, node_id, cmd.available.clone()),
                        cmd))
                    .collect();
                let target_refs: Vec<(&AnyObject, &rn_core::data::Command)> = pairs
                    .iter()
                    .map(|(t, cmd)| {
                        let obj: &AnyObject = t;
                        (obj, *cmd)
                    })
                    .collect();
                let conflicts = crate::commands::install_main_menu_with_targets(mtm, command_set, &target_refs);
                for c in &conflicts {
                    log::warn!("ShortcutConflict: '{}' and '{}' share binding '{}{}'",
                        c.a, c.b, c.key, if c.modifiers.0 != 0 { " (with modifiers)" } else { "" });
                }
                for (t, _) in pairs { targets.push(t); }
                stash_delegate(node_id, DelegateHandle::Commands(targets));
            }
            window.makeKeyAndOrderFront(None);
            state.nodes.insert(node_id, NativeHandle::Window(window));
            state
                .shadow
                .insert(node_id, ShadowNode::new(kind, props, parent_id));
            state.windows.push(node_id);
        }
        ComponentKind::Label => {
            let text = string_prop(&props, PropKey::Text, "");
            let ns_text = NSString::from_str(&text);
            let max_lines = f32_prop(&props, PropKey::MaxLines, 1.0).max(1.0) as usize;
            let wants_wrap = bool_prop(&props, PropKey::TextWrap, false) || max_lines > 1;

            // AppKit ships a label already configured to wrap. Building an ordinary
            // label and poking its cell afterwards does not make it wrap — that is
            // why MaxLines produced a taller box with the same single line in it.
            let label: Retained<NSTextField> = unsafe {
                if wants_wrap {
                    NSTextField::wrappingLabelWithString(&ns_text, mtm)
                } else {
                    NSTextField::labelWithString(&ns_text, mtm)
                }
            };
            unsafe {
                label.setEditable(false);
                label.setSelectable(false);
                label.setBordered(false);
                label.setDrawsBackground(false);
                // Text alignment was a dead prop: the keys existed but nothing read
                // them, so centring a label was impossible.
                label.setAlignment(match f32_prop(&props, PropKey::TextAlign, 0.0) as i32 {
                    1 => objc2_app_kit::NSTextAlignment::Right,
                    2 => objc2_app_kit::NSTextAlignment::Center,
                    _ => objc2_app_kit::NSTextAlignment::Left,
                });
                if wants_wrap {
                    label.setLineBreakMode(NSLineBreakMode(0)); // word wrapping
                    label.setMaximumNumberOfLines(max_lines as isize);
                } else {
                    label.setLineBreakMode(NSLineBreakMode(4)); // truncate tail
                    label.setMaximumNumberOfLines(1);
                }
            }
            attach_to_parent(state, parent_id, &label);
            state.nodes.insert(node_id, NativeHandle::Label(label));
            state
                .shadow
                .insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }
        ComponentKind::Button => {
            let text = string_prop(&props, PropKey::Text, "Button");
            let ns_text = NSString::from_str(&text);
            let button: Retained<NSButton> =
                unsafe { NSButton::buttonWithTitle_target_action(&ns_text, None, None, mtm) };
            unsafe {
                button.setBezelStyle(NSBezelStyle::Push);
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    button.setTarget(Some(obj));
                    button.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &button);
            state.nodes.insert(node_id, NativeHandle::Button(button));
            state
                .shadow
                .insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }
        ComponentKind::TextInput => {
            let placeholder = string_prop(&props, PropKey::Placeholder, "");
            let ns_text = NSString::from_str(&placeholder);
            let field: Retained<NSTextField> =
                unsafe { NSTextField::textFieldWithString(&ns_text, mtm) };
            unsafe {
                field.setEditable(true);
                field.setBordered(true);
                if let Some(PropValue::String(initial)) = props.get(&PropKey::Text) {
                    let s = NSString::from_str(initial);
                    field.setStringValue(&s);
                }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let delegate = TextDelegate::new(mtm, *h, node_id);
                let proto: &ProtocolObject<dyn NSTextFieldDelegate> =
                    ProtocolObject::from_ref(&*delegate);
                unsafe { field.setDelegate(Some(proto)) };
                stash_delegate(node_id, DelegateHandle::Text(delegate));
            }
            attach_to_parent(state, parent_id, &field);
            state.nodes.insert(node_id, NativeHandle::TextInput(field));
            state
                .shadow
                .insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }
        ComponentKind::LayoutContainer(axis) => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 100.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            let info = ContainerInfo {
                axis,
                padding: f32_prop(&props, PropKey::Padding, 0.0),
                gap: f32_prop(&props, PropKey::Gap, 0.0),
            };
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::View(view, info));
            state
                .shadow
                .insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Checkbox (NSButton, switch style)
        // ------------------------------------------------------------------
        ComponentKind::Checkbox => {
            let text = string_prop(&props, PropKey::Text, "");
            let ns_text = NSString::from_str(&text);
            let btn: Retained<NSButton> = unsafe {
                NSButton::checkboxWithTitle_target_action(&ns_text, None, None, mtm)
            };
            let checked = bool_prop(&props, PropKey::Checked, false);
            unsafe { btn.setState(if checked { 1 } else { 0 }) };
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &btn);
            state.nodes.insert(node_id, NativeHandle::Checkbox(btn));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Slider (NSSlider)
        // ------------------------------------------------------------------
        ComponentKind::Slider => {
            let min = f32_prop(&props, PropKey::Min, 0.0);
            let max = f32_prop(&props, PropKey::Max, 100.0);
            let val = f32_prop(&props, PropKey::Value, min);
            let slider: Retained<NSSlider> = unsafe {
                NSSlider::sliderWithValue_minValue_maxValue_target_action(
                    val as f64,
                    min as f64,
                    max as f64,
                    None,
                    None,
                    mtm,
                )
            };
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    slider.setTarget(Some(obj));
                    slider.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &slider);
            state.nodes.insert(node_id, NativeHandle::Slider(slider));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ProgressBar (NSProgressIndicator, determinate)
        // ------------------------------------------------------------------
        ComponentKind::ProgressBar => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 20.0));
            let progress: Retained<NSProgressIndicator> = unsafe {
                NSProgressIndicator::initWithFrame(mtm.alloc(), frame)
            };
            let min = f32_prop(&props, PropKey::Min, 0.0);
            let max = f32_prop(&props, PropKey::Max, 100.0);
            let val = f32_prop(&props, PropKey::Value, 0.0);
            unsafe {
                progress.setIndeterminate(false);
                progress.setMinValue(min as f64);
                progress.setMaxValue(max as f64);
                progress.setDoubleValue(val as f64);
            }
            attach_to_parent(state, parent_id, &progress);
            state.nodes.insert(node_id, NativeHandle::ProgressBar(progress));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Separator (NSBox, separator style)
        // ------------------------------------------------------------------
        ComponentKind::Separator => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 8.0));
            let sep: Retained<NSBox> = unsafe { NSBox::initWithFrame(mtm.alloc(), frame) };
            unsafe { sep.setBoxType(NSBoxType::NSBoxSeparator) };
            attach_to_parent(state, parent_id, &sep);
            state.nodes.insert(node_id, NativeHandle::Separator(sep));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // TextArea (NSScrollView + NSTextView)
        // ------------------------------------------------------------------
        ComponentKind::TextArea => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 100.0));
            let scroll = unsafe { NSScrollView::initWithFrame(mtm.alloc(), frame) };
            let text_view = unsafe { NSTextView::initWithFrame(mtm.alloc(), frame) };
            let text = string_prop(&props, PropKey::Text, "");
            let ns_text = NSString::from_str(&text);
            unsafe {
                text_view.setString(&ns_text);
                scroll.setDocumentView(Some(&text_view));
                scroll.setHasVerticalScroller(true);
            }
            // Note: NSTextView uses NSTextViewDelegate, not NSTextFieldDelegate.
            // OnChange for TextArea is handled via a periodic poll in a future version.
            attach_to_parent(state, parent_id, &scroll);
            state.nodes.insert(node_id, NativeHandle::TextArea(scroll));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Dropdown (NSPopUpButton)
        // ------------------------------------------------------------------
        ComponentKind::Dropdown => {
            let popup: Retained<NSPopUpButton> = unsafe {
                NSPopUpButton::initWithFrame_pullsDown(
                    mtm.alloc(),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 26.0)),
                    false,
                )
            };
            if let Some(PropValue::StringList(items)) = props.get(&PropKey::Options) {
                for item in items {
                    let ns = NSString::from_str(item);
                    unsafe { popup.addItemWithTitle(&ns) };
                }
            }
            let idx = f32_prop(&props, PropKey::SelectedIndex, 0.0) as usize;
            unsafe { popup.selectItemAtIndex(idx as isize) };
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    popup.setTarget(Some(obj));
                    popup.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &popup);
            state.nodes.insert(node_id, NativeHandle::Dropdown(popup));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Image (NSImageView with SF Symbol)
        // ------------------------------------------------------------------
        ComponentKind::Image => {
            // IconColor turns the image into a System Settings header tile:
            // a colored rounded square with the glyph drawn white inside it.
            let icon_tint = match props.get(&PropKey::IconColor) {
                Some(PropValue::String(s)) => Some(s.clone()),
                _ => None,
            };
            let tile_size = 56.0;
            let (w, h) = if icon_tint.is_some() { (tile_size, tile_size) } else { (32.0, 32.0) };
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h));
            let image_view: Retained<NSImageView> = unsafe {
                let v = NSImageView::initWithFrame(mtm.alloc(), frame);
                // Without this the image draws at its natural size inside whatever
                // frame layout gives it, so a 16pt symbol in a 64pt cell stays 16pt.
                v.setImageScaling(objc2_app_kit::NSImageScaling::NSImageScaleProportionallyUpOrDown);
                v
            };
            if let Some(PropValue::String(name)) = props.get(&PropKey::ImageName) {
                let ns_name = NSString::from_str(name);
                // Try SF Symbol first (macOS 11+), fall back to named image.
                let img = unsafe {
                    NSImage::imageWithSystemSymbolName_accessibilityDescription(
                        &ns_name,
                        None,
                    )
                    .or_else(|| NSImage::imageNamed(&ns_name))
                };
                if let Some(img) = img {
                    unsafe { image_view.setImage(Some(&img)) };
                }
            }
            if let Some(tint_name) = icon_tint {
                let box_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(tile_size, tile_size));
                let tile = unsafe { NSBox::initWithFrame(mtm.alloc(), box_frame) };
                unsafe {
                    tile.setBoxType(NSBoxType::NSBoxCustom);
                    tile.setCornerRadius(13.0);
                    tile.setFillColor(&crate::renderer::icon_tint(&tint_name));
                    tile.setBorderWidth(0.0);
                    // NSBox insets its content view by default; zeroing the margins
                    // makes subview frames land exactly where we put them.
                    tile.setContentViewMargins(NSSize::new(0.0, 0.0));
                }
                // Build the glyph view at its final inner frame (20% margin).
                let inner = NSRect::new(
                    NSPoint::new(tile_size * 0.2, tile_size * 0.2),
                    NSSize::new(tile_size * 0.6, tile_size * 0.6),
                );
                let glyph: Retained<NSImageView> = unsafe {
                    let v = NSImageView::initWithFrame(mtm.alloc(), inner);
                    v.setImageScaling(objc2_app_kit::NSImageScaling::NSImageScaleProportionallyUpOrDown);
                    v
                };
                if let Some(PropValue::String(name)) = props.get(&PropKey::ImageName) {
                    let img = unsafe {
                        NSImage::imageWithSystemSymbolName_accessibilityDescription(
                            &NSString::from_str(name), None,
                        )
                    };
                    if let Some(img) = img {
                        unsafe { glyph.setImage(Some(&img)) };
                    }
                }
                unsafe {
                    glyph.setContentTintColor(Some(&NSColor::whiteColor()));
                    if let Some(cv) = tile.contentView() {
                        cv.addSubview(&glyph);
                    }
                }
                attach_to_parent(state, parent_id, &tile);
                state.nodes.insert(node_id, NativeHandle::View(
                    Retained::into_super(tile),
                    ContainerInfo { axis: Axis::Vertical, padding: 0.0, gap: 0.0 },
                ));
                state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
                register_child(state, parent_id, node_id);
            } else {
                attach_to_parent(state, parent_id, &image_view);
                state.nodes.insert(node_id, NativeHandle::Image(image_view));
                state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
                register_child(state, parent_id, node_id);
            }
        }

        // ------------------------------------------------------------------
        // Stepper (NSView holding NSStepper + NSTextField label)
        // ------------------------------------------------------------------
        ComponentKind::Stepper => {
            let container_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(120.0, 26.0));
            let container: Retained<NSView> = unsafe {
                NSView::initWithFrame(mtm.alloc(), container_frame)
            };
            let val = f32_prop(&props, PropKey::Value, 0.0);
            let min = f32_prop(&props, PropKey::Min, 0.0);
            let max = f32_prop(&props, PropKey::Max, 100.0);
            let step = f32_prop(&props, PropKey::Step, 1.0);
            // Value label (right-aligned)
            let label_frame = NSRect::new(NSPoint::new(0.0, 2.0), NSSize::new(70.0, 22.0));
            let label: Retained<NSTextField> = unsafe {
                NSTextField::labelWithString(&NSString::from_str(&format!("{val:.0}")), mtm)
            };
            unsafe { label.setFrame(label_frame) };
            // NSStepper
            let step_frame = NSRect::new(NSPoint::new(74.0, 0.0), NSSize::new(46.0, 26.0));
            let stepper: Retained<NSStepper> = unsafe {
                NSStepper::initWithFrame(mtm.alloc(), step_frame)
            };
            unsafe {
                stepper.setMinValue(min as f64);
                stepper.setMaxValue(max as f64);
                stepper.setDoubleValue(val as f64);
                stepper.setIncrement(step as f64);
                stepper.setValueWraps(false);
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    stepper.setTarget(Some(obj));
                    stepper.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            unsafe {
                container.addSubview(&label);
                container.addSubview(&stepper);
            }
            attach_to_parent(state, parent_id, &container);
            state.nodes.insert(node_id, NativeHandle::Stepper(container));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ColorWell (NSColorWell)
        // ------------------------------------------------------------------
        ComponentKind::ColorWell => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(46.0, 26.0));
            let well: Retained<NSColorWell> = unsafe {
                NSColorWell::initWithFrame(mtm.alloc(), frame)
            };
            if let Some(PropValue::Color(ColorValue { r, g, b, a })) = props.get(&PropKey::Color) {
                let color = unsafe {
                    NSColor::colorWithRed_green_blue_alpha(
                        *r as f64 / 255.0,
                        *g as f64 / 255.0,
                        *b as f64 / 255.0,
                        *a as f64 / 255.0,
                    )
                };
                unsafe { well.setColor(&color) };
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    well.setTarget(Some(obj));
                    well.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &well);
            state.nodes.insert(node_id, NativeHandle::ColorWell(well));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // DatePicker (NSDatePicker, compact style)
        // ------------------------------------------------------------------
        ComponentKind::DatePicker => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 26.0));
            let dp: Retained<NSDatePicker> = unsafe {
                NSDatePicker::initWithFrame(mtm.alloc(), frame)
            };
            unsafe {
                dp.setDatePickerStyle(NSDatePickerStyle::TextField);
                dp.setDateValue(&NSDate::now());
            }
            attach_to_parent(state, parent_id, &dp);
            state.nodes.insert(node_id, NativeHandle::DatePicker(dp));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // RadioGroup (NSView holding individual NSButton radio buttons)
        // ------------------------------------------------------------------
        ComponentKind::RadioGroup => {
            let options: Vec<String> = if let Some(PropValue::StringList(opts)) =
                props.get(&PropKey::Options)
            {
                opts.clone()
            } else {
                vec![]
            };
            let selected = f32_prop(&props, PropKey::SelectedIndex, 0.0) as i32;
            let button_h: f32 = 20.0;
            let gap: f32 = 4.0;
            let total_h = (options.len() as f32) * (button_h + gap);
            let frame = NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(180.0, total_h as f64),
            );
            let container: Retained<NSView> = unsafe {
                NSView::initWithFrame(mtm.alloc(), frame)
            };
            for (i, opt) in options.iter().enumerate() {
                let ns = NSString::from_str(opt);
                let btn: Retained<NSButton> = unsafe {
                    NSButton::radioButtonWithTitle_target_action(&ns, None, None, mtm)
                };
                let y = total_h - (i as f32 + 1.0) * (button_h + gap);
                let btn_frame = NSRect::new(
                    NSPoint::new(0.0, y as f64),
                    NSSize::new(180.0, button_h as f64),
                );
                unsafe {
                    btn.setFrame(btn_frame);
                    btn.setState(if i as i32 == selected { 1 } else { 0 });
                    container.addSubview(&btn);
                }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                let count = unsafe { container.subviews().count() };
                for i in 0..count {
                    let subview = unsafe { container.subviews().objectAtIndex(i) };
                    // All subviews added above are NSButton — raw cast is safe.
                    let btn_ref: &NSButton = unsafe { &*(subview.as_ref() as *const NSView as *const NSButton) };
                    unsafe {
                        let obj: &AnyObject = &target;
                        btn_ref.setTarget(Some(obj));
                        btn_ref.setAction(Some(sel!(fire:)));
                    }
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &container);
            state.nodes.insert(node_id, NativeHandle::RadioGroup(container));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ScrollView — NSScrollView with content NSView (FR-001, FR-016)
        // ------------------------------------------------------------------
        ComponentKind::ScrollView => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 200.0));
            let sv = new_scroll_container(mtm, frame, false);
            attach_to_parent(state, parent_id, &sv);
            state.nodes.insert(node_id, NativeHandle::ScrollViewWidget(sv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // FlatList — virtualized scrollable list (FR-002, FR-004)
        // Uses NSScrollView host; actual virtualization managed by FlatListState
        // ------------------------------------------------------------------
        ComponentKind::FlatList => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 300.0));
            let sv = new_scroll_container(mtm, frame, false);
            // Instantiate virtualization state (FR-002, FR-004).
            let item_h = f32_prop(&props, PropKey::ItemHeightFixed, 44.0);
            let flat_state = FlatListRendererState::new(
                item_h,
                300.0, // viewport height (updated on layout pass)
                f32_prop(&props, PropKey::BufferRows, 3.0) as usize,
                f32_prop(&props, PropKey::EndReachThreshold, 100.0),
            );
            attach_to_parent(state, parent_id, &sv);
            state.nodes.insert(node_id, NativeHandle::FlatListWidget(sv, Box::new(flat_state)));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // TouchableOpacity — transparent NSView overlay (FR-005, FR-006)
        // Full press/alpha implementation requires an NSView subclass;
        // stash as View for now, alpha applied on press via update_node.
        // ------------------------------------------------------------------
        ComponentKind::TouchableOpacity => {
            let press_opacity = f32_prop(&props, PropKey::PressOpacity, 0.4);
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 40.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            view.setWantsLayer(true);
            // Overlay a transparent NSButton that handles press feedback (FR-005, FR-006).
            // The button has no title/image; it sits on top of children and fires OnPress.
            let btn: Retained<NSButton> = unsafe { NSButton::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                btn.setTitle(&NSString::from_str(""));
                btn.setBordered(false);
                btn.setBezelStyle(NSBezelStyle::Automatic);
                btn.setTransparent(true);
                btn.setWantsLayer(true);
                view.addSubview(&btn);
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            let _ = press_opacity; // stored in shadow props for future layer-opacity animation
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::TouchableWidget(view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // SafeAreaView — insets content by title bar height (FR-007)
        // ------------------------------------------------------------------
        ComponentKind::SafeAreaView => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 400.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::SafeAreaWidget(view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ContentArea — main pane container with macOS content background
        // ------------------------------------------------------------------
        ComponentKind::ContentArea => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(600.0, 400.0));
            // Paints the window background and scrolls: a content area is a window's
            // main surface, so content taller than the pane must stay reachable.
            let sv = new_scroll_container(mtm, frame, true);
            attach_to_parent(state, parent_id, &sv);
            state.nodes.insert(node_id, NativeHandle::ScrollViewWidget(sv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ListTile — settings row: icon tile + title + subtitle + chevron
        // ------------------------------------------------------------------
        ComponentKind::ListTile => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 44.0));
            let row: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            populate_list_tile(mtm, &row, node_id, &props);
            attach_to_parent(state, parent_id, &row);
            state.nodes.insert(node_id, NativeHandle::ListTileWidget(row));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Scaffold — vertical stack container for title bar / toolbar / content
        // ------------------------------------------------------------------
        ComponentKind::Scaffold => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::ScaffoldWidget(view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // TitleBar — window chrome strip with centered title
        // ------------------------------------------------------------------
        ComponentKind::TitleBar => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 28.0));
            let bar: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            if let Some(PropValue::String(title)) = props.get(&PropKey::Text) {
                let tf: Retained<NSTextField> = unsafe {
                    let t = NSTextField::initWithFrame(
                        mtm.alloc(),
                        NSRect::new(NSPoint::new(80.0, 4.0), NSSize::new(640.0, 20.0)),
                    );
                    t.setEditable(false);
                    t.setBordered(false);
                    t.setDrawsBackground(false);
                    t.setAlignment(objc2_app_kit::NSTextAlignment::Center);
                    t.setStringValue(&NSString::from_str(title));
                    t.setFont(Some(&NSFont::boldSystemFontOfSize(NSFont::systemFontSize())));
                    t.setTextColor(Some(&NSColor::secondaryLabelColor()));
                    t
                };
                unsafe { bar.addSubview(&tf) };
            }
            attach_to_parent(state, parent_id, &bar);
            state.nodes.insert(node_id, NativeHandle::TitleBarWidget(bar));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // IconButton — borderless SF Symbol button
        // ------------------------------------------------------------------
        ComponentKind::IconButton => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(28.0, 28.0));
            let btn: Retained<NSButton> = unsafe { NSButton::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                btn.setBezelStyle(NSBezelStyle::Circular);
                btn.setBordered(false);
                btn.setTitle(&NSString::from_str(""));
            }
            if let Some(PropValue::String(sym)) = props.get(&PropKey::IconName) {
                let ns_name = NSString::from_str(sym);
                if let Some(img) =
                    unsafe { NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None) }
                {
                    unsafe { btn.setImage(Some(&img)) }
                }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &btn);
            state.nodes.insert(node_id, NativeHandle::IconButtonWidget(btn));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // DisclosureButton — triangle disclosure toggle
        // ------------------------------------------------------------------
        ComponentKind::DisclosureButton => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(160.0, 20.0));
            let btn: Retained<NSButton> = unsafe { NSButton::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                btn.setBezelStyle(NSBezelStyle::Disclosure);
                btn.setButtonType(objc2_app_kit::NSButtonType::PushOnPushOff);
            }
            if let Some(PropValue::String(label)) = props.get(&PropKey::Text) {
                unsafe { btn.setTitle(&NSString::from_str(label)) }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &btn);
            state.nodes.insert(node_id, NativeHandle::DisclosureButtonWidget(btn));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // HelpButton — round "?" button
        // ------------------------------------------------------------------
        ComponentKind::HelpButton => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(20.0, 20.0));
            let btn: Retained<NSButton> = unsafe { NSButton::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                btn.setBezelStyle(NSBezelStyle::HelpButton);
                btn.setTitle(&NSString::from_str(""));
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &btn);
            state.nodes.insert(node_id, NativeHandle::HelpButtonWidget(btn));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // PopupButton — bezeled pull-down for forms
        // ------------------------------------------------------------------
        ComponentKind::PopupButton => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(160.0, 26.0));
            let popup: Retained<NSPopUpButton> = unsafe {
                let p = NSPopUpButton::initWithFrame_pullsDown(mtm.alloc(), frame, false);
                p.setBezelStyle(NSBezelStyle::Toolbar);
                p
            };
            if let Some(PropValue::StringList(opts)) = props.get(&PropKey::Options) {
                unsafe {
                    popup.removeAllItems();
                    for o in opts {
                        popup.addItemWithTitle(&NSString::from_str(o));
                    }
                    let sel = f32_prop(&props, PropKey::SelectedIndex, 0.0) as isize;
                    popup.selectItemAtIndex(sel);
                }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    popup.setTarget(Some(obj));
                    popup.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &popup);
            state.nodes.insert(node_id, NativeHandle::PopupButtonWidget(popup));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // BackButton — chevron-left navigation button
        // ------------------------------------------------------------------
        ComponentKind::BackButton => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(32.0, 28.0));
            let btn: Retained<NSButton> = unsafe { NSButton::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                btn.setBezelStyle(NSBezelStyle::Toolbar);
                btn.setBordered(false);
                btn.setTitle(&NSString::from_str(""));
            }
            let ns_name = NSString::from_str("chevron.left");
            if let Some(img) =
                unsafe { NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None) }
            {
                unsafe { btn.setImage(Some(&img)) }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &btn);
            state.nodes.insert(node_id, NativeHandle::BackButtonWidget(btn));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // CapacityIndicator — discrete capacity bar (NSLevelIndicator)
        // ------------------------------------------------------------------
        ComponentKind::CapacityIndicator => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 16.0));
            let li: Retained<NSLevelIndicator> =
                unsafe { NSLevelIndicator::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                li.setLevelIndicatorStyle(NSLevelIndicatorStyle::DiscreteCapacity);
                li.setMinValue(0.0);
                li.setMaxValue(f32_prop(&props, PropKey::Max, 100.0) as f64);
                li.setDoubleValue(f32_prop(&props, PropKey::Value, 0.0) as f64);
            }
            attach_to_parent(state, parent_id, &li);
            state.nodes.insert(node_id, NativeHandle::CapacityIndicatorWidget(li));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // RatingIndicator — star rating (NSLevelIndicator Rating style)
        // ------------------------------------------------------------------
        ComponentKind::RatingIndicator => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(120.0, 20.0));
            let li: Retained<NSLevelIndicator> =
                unsafe { NSLevelIndicator::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                li.setLevelIndicatorStyle(NSLevelIndicatorStyle::Rating);
                li.setMinValue(0.0);
                li.setMaxValue(f32_prop(&props, PropKey::Max, 5.0) as f64);
                li.setDoubleValue(f32_prop(&props, PropKey::Value, 0.0) as f64);
            }
            attach_to_parent(state, parent_id, &li);
            state.nodes.insert(node_id, NativeHandle::RatingIndicatorWidget(li));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // RelevanceIndicator — search relevance bars (NSLevelIndicator)
        // ------------------------------------------------------------------
        ComponentKind::RelevanceIndicator => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(48.0, 16.0));
            let li: Retained<NSLevelIndicator> =
                unsafe { NSLevelIndicator::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                li.setLevelIndicatorStyle(NSLevelIndicatorStyle::Relevancy);
                li.setMinValue(0.0);
                li.setMaxValue(f32_prop(&props, PropKey::Max, 1.0) as f64);
                li.setDoubleValue(f32_prop(&props, PropKey::Value, 0.5) as f64);
            }
            attach_to_parent(state, parent_id, &li);
            state.nodes.insert(node_id, NativeHandle::RelevanceIndicatorWidget(li));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Tooltip — small secondary label
        // ------------------------------------------------------------------
        ComponentKind::Tooltip => {
            let text = match props.get(&PropKey::Text) {
                Some(PropValue::String(s)) => s.clone(),
                _ => String::new(),
            };
            let tf_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 18.0));
            let tf: Retained<NSTextField> = unsafe {
                let t = NSTextField::initWithFrame(mtm.alloc(), tf_frame);
                t.setEditable(false);
                t.setBordered(false);
                t.setDrawsBackground(false);
                t.setStringValue(&NSString::from_str(&text));
                t.setFont(Some(&NSFont::systemFontOfSize(NSFont::smallSystemFontSize())));
                t.setTextColor(Some(&NSColor::secondaryLabelColor()));
                t
            };
            attach_to_parent(state, parent_id, &tf);
            state.nodes.insert(node_id, NativeHandle::TooltipWidget(tf));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // TimePicker — hour/minute field with steppers
        // ------------------------------------------------------------------
        ComponentKind::TimePicker => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 26.0));
            let dp: Retained<NSDatePicker> =
                unsafe { NSDatePicker::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                dp.setDatePickerStyle(NSDatePickerStyle::TextFieldAndStepper);
                dp.setDatePickerMode(NSDatePickerMode::Single);
                dp.setDatePickerElements(NSDatePickerElementFlags(0x000c)); // HourMinute
            }
            attach_to_parent(state, parent_id, &dp);
            state.nodes.insert(node_id, NativeHandle::TimePickerWidget(dp));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // GraphicalTimePicker — clock face
        // ------------------------------------------------------------------
        ComponentKind::GraphicalTimePicker => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 160.0));
            let dp: Retained<NSDatePicker> =
                unsafe { NSDatePicker::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                dp.setDatePickerStyle(NSDatePickerStyle::ClockAndCalendar);
                dp.setDatePickerMode(NSDatePickerMode::Single);
                dp.setDatePickerElements(NSDatePickerElementFlags(0x000c)); // HourMinute
            }
            attach_to_parent(state, parent_id, &dp);
            state.nodes.insert(node_id, NativeHandle::GraphicalTimePickerWidget(dp));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ToolbarIconButton — SF Symbol button for toolbars
        // ------------------------------------------------------------------
        ComponentKind::ToolbarIconButton => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(32.0, 28.0));
            let btn: Retained<NSButton> = unsafe { NSButton::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                btn.setBezelStyle(NSBezelStyle::Toolbar);
                btn.setBordered(false);
                btn.setTitle(&NSString::from_str(""));
            }
            if let Some(PropValue::String(sym)) = props.get(&PropKey::IconName) {
                let ns_name = NSString::from_str(sym);
                if let Some(img) = unsafe {
                    NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None)
                } {
                    unsafe { btn.setImage(Some(&img)) }
                }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &btn);
            state.nodes.insert(node_id, NativeHandle::ToolbarIconButtonWidget(btn));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ToolbarSpacer — flexible blank area
        // ------------------------------------------------------------------
        ComponentKind::ToolbarSpacer => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(8.0, 28.0));
            let sp: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &sp);
            state.nodes.insert(node_id, NativeHandle::ToolbarSpacerWidget(sp));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ToolbarDivider — vertical rule between toolbar items
        // ------------------------------------------------------------------
        ComponentKind::ToolbarDivider => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 20.0));
            let dv: Retained<NSBox> = unsafe { NSBox::initWithFrame(mtm.alloc(), frame) };
            unsafe { dv.setBoxType(NSBoxType::NSBoxSeparator) };
            attach_to_parent(state, parent_id, &dv);
            state.nodes.insert(node_id, NativeHandle::ToolbarDividerWidget(dv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ToolbarPopup — pull-down menu for a toolbar
        // ------------------------------------------------------------------
        ComponentKind::ToolbarPopup => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(120.0, 26.0));
            let popup: Retained<NSPopUpButton> = unsafe {
                let p = NSPopUpButton::initWithFrame_pullsDown(mtm.alloc(), frame, true);
                p.setBezelStyle(NSBezelStyle::Toolbar);
                p
            };
            if let Some(PropValue::StringList(opts)) = props.get(&PropKey::Options) {
                unsafe {
                    popup.removeAllItems();
                    for o in opts {
                        popup.addItemWithTitle(&NSString::from_str(o));
                    }
                }
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    popup.setTarget(Some(obj));
                    popup.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &popup);
            state.nodes.insert(node_id, NativeHandle::ToolbarPopupWidget(popup));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // AlertDialog — modal alert panel with title + message
        // ------------------------------------------------------------------
        ComponentKind::AlertDialog => {
            // A real NSAlert, presented on press. AppKit alerts are modal system
            // objects, not views, so the component renders as the button that
            // raises one rather than a painted imitation of a dialog.
            let title = match props.get(&PropKey::Title) {
                Some(PropValue::String(s)) => s.clone(),
                _ => String::new(),
            };
            let message = match props.get(&PropKey::Text) {
                Some(PropValue::String(s)) => s.clone(),
                _ => String::new(),
            };
            let on_press = match props.get(&PropKey::OnPress) {
                Some(PropValue::HandlerRef(h)) => Some(*h),
                _ => None,
            };

            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 32.0));
            let btn: Retained<NSButton> = unsafe {
                let b = NSButton::initWithFrame(mtm.alloc(), frame);
                b.setBezelStyle(NSBezelStyle::Automatic);
                b.setTitle(&NSString::from_str(&title));
                b.setAction(Some(sel!(present:)));
                b
            };
            // A modal needs a window to attach to; without one there is nothing to
            // present on, so the button is simply disabled rather than dead.
            match enclosing_window(state, parent_id) {
                Some(window) => {
                    let target = AlertTarget::new(
                        mtm,
                        AlertIvars {
                            title: title.clone(),
                            message,
                            handler: on_press,
                            source: node_id,
                            window,
                        },
                    );
                    unsafe {
                        let obj: &AnyObject = &target;
                        btn.setTarget(Some(obj));
                    }
                    stash_delegate(node_id, DelegateHandle::Alert(target));
                }
                None => btn.setEnabled(false),
            }

            let btn_view: Retained<NSView> = unsafe { Retained::cast(btn) };
            attach_to_parent(state, parent_id, &btn_view);
            state.nodes.insert(node_id, NativeHandle::AlertDialogWidget(btn_view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Sheet — window-attached modal container
        // ------------------------------------------------------------------
        ComponentKind::Sheet => {
            // A real sheet: its own NSWindow, presented on the owner. Because the
            // sheet has its own window, its container is that window's content view
            // and children attach there instead of to the parent view.
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 280.0));
            let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
            let sheet_window: Retained<NSWindow> = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc(),
                    frame,
                    style,
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                )
            };
            unsafe {
                sheet_window.setReleasedWhenClosed(false);
                if let Some(PropValue::String(title)) = props.get(&PropKey::Title) {
                    sheet_window.setTitle(&NSString::from_str(title));
                }
            }

            let content = sheet_window.contentView().unwrap_or_else(|| {
                // A window always has a content view; synthesising one keeps the
                // component usable rather than panicking if that ever changes.
                let v: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
                sheet_window.setContentView(Some(&v));
                v
            });

            state.nodes.insert(node_id, NativeHandle::SheetWidget(content, sheet_window));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // Phase A: table and tree — renderer modules handle the actual construction,
        // delegated to rn-macos/src/table.rs and rn-macos/src/tree.rs.
        ComponentKind::Table => {
            // Extract TableModel from props.
            let model = match props.get(&PropKey::TableModelKey) {
                Some(PropValue::TableModelProp(m)) => m.clone(),
                _ => rn_core::data::TableModel {
                    columns: vec![],
                    rows: vec![],
                    sort: None,
                    selection: rn_core::data::Selection::default(),
                },
            };
            let handler_select = match props.get(&PropKey::OnSelect) {
                Some(PropValue::HandlerRef(h)) => Some(*h),
                _ => None,
            };
            let handler_activate = match props.get(&PropKey::OnActivate) {
                Some(PropValue::HandlerRef(h)) => Some(*h),
                _ => None,
            };
            // Build NSTableView inside a scroll view.
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
            let sv = new_scroll_container(mtm, frame, false);
            // FR-008/edge: enable horizontal scroll so columns hold their minimums when
            // the window is narrower than the combined minimum column widths.
            unsafe { sv.setHasHorizontalScroller(true); }
            let tv: Retained<NSTableView> = unsafe {
                NSTableView::initWithFrame(mtm.alloc(), frame)
            };
            // Configure NSTableView.
            unsafe {
                tv.setUsesAlternatingRowBackgroundColors(true);
                tv.setAllowsColumnResizing(true);
                tv.setAllowsColumnReordering(false);
                let multi = matches!(props.get(&PropKey::MultiSelect), Some(PropValue::Bool(true)));
                tv.setAllowsMultipleSelection(multi);
            }
            // Add a column per model column.
            for col in &model.columns {
                let ns_col: Retained<objc2_app_kit::NSTableColumn> = unsafe {
                    objc2_app_kit::NSTableColumn::initWithIdentifier(
                        mtm.alloc(),
                        &NSString::from_str(&col.title),
                    )
                };
                unsafe { ns_col.setWidth(col.width as f64); }
                unsafe { ns_col.setMinWidth(col.min_width as f64); }
                unsafe { ns_col.setTitle(&NSString::from_str(&col.title)); }
                // FR-008: allow user to resize columns by dragging the divider.
                unsafe {
                    use objc2_app_kit::NSTableColumnResizingOptions;
                    ns_col.setResizingMask(NSTableColumnResizingOptions::NSTableColumnUserResizingMask);
                }
                if col.sortable {
                    let desc = unsafe {
                        objc2_foundation::NSSortDescriptor::sortDescriptorWithKey_ascending(
                            Some(&NSString::from_str(&col.title)),
                            true,
                        )
                    };
                    unsafe { ns_col.setSortDescriptorPrototype(Some(&desc)); }
                }
                unsafe { tv.addTableColumn(&ns_col); }
            }
            // Create data source + delegate.
            let model_is_empty = model.rows.is_empty();
            let mut sources = crate::table::make_table_sources(
                mtm, model, node_id, handler_select, handler_activate,
            );
            unsafe {
                tv.setDataSource(Some(ProtocolObject::from_ref(&*sources.data)));
                tv.setDelegate(Some(ProtocolObject::from_ref(&*sources.delegate)));
                // FR-018: wire doubleAction to a ButtonTarget for OnActivate.
                if let Some(h) = handler_activate {
                    let activate_target = ButtonTarget::with_availability(
                        mtm, h, node_id, rn_core::data::AvailabilityRule::Always,
                    );
                    tv.setDoubleAction(Some(objc2::sel!(fire:)));
                    let obj: &AnyObject = &activate_target;
                    tv.setTarget(Some(obj));
                    // Keep alive via sources.
                    // ButtonTarget extends NSObject (ClassType::Super = NSObject).
                    let ns_obj: Retained<objc2::runtime::NSObject> = Retained::cast(activate_target);
                    sources.activate = Some(ns_obj);
                }
            }
            // Mount table inside scroll view.
            unsafe { sv.setDocumentView(Some(&*tv)); }
            stash_delegate(node_id, DelegateHandle::Table(sources));
            // FR-014: empty-state label shown when the model has no rows.
            if model_is_empty {
                let empty_text = match props.get(&PropKey::EmptyText) {
                    Some(PropValue::String(s)) => s.clone(),
                    _ => "No items".to_owned(),
                };
                let empty_label = unsafe {
                    NSTextField::labelWithString(&NSString::from_str(&empty_text), mtm)
                };
                unsafe {
                    empty_label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
                    empty_label.setTag(0xE0);
                    sv.addSubview(&empty_label);
                }
            }
            attach_to_parent(state, parent_id, &sv);
            state.nodes.insert(node_id, NativeHandle::TableWidget(sv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        ComponentKind::Tree => {
            // Extract TreeModel from props.
            let model = match props.get(&PropKey::TreeModelKey) {
                Some(PropValue::TreeModelProp(m)) => m.clone(),
                _ => rn_core::data::TreeModel::new(vec![]),
            };
            let handler_select = match props.get(&PropKey::OnSelect) {
                Some(PropValue::HandlerRef(h)) => Some(*h),
                _ => None,
            };
            let handler_activate = match props.get(&PropKey::OnActivate) {
                Some(PropValue::HandlerRef(h)) => Some(*h),
                _ => None,
            };
            // Build NSOutlineView inside a scroll view.
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
            let sv = new_scroll_container(mtm, frame, false);
            let ov: Retained<NSOutlineView> = unsafe {
                NSOutlineView::initWithFrame(mtm.alloc(), frame)
            };
            // Single name column.
            let col: Retained<NSTableColumn> = unsafe {
                NSTableColumn::initWithIdentifier(
                    mtm.alloc(),
                    &NSString::from_str("name"),
                )
            };
            unsafe {
                col.setWidth(300.0);
                ov.addTableColumn(&col);
                ov.setOutlineTableColumn(Some(&col));
                // FR-015/edge: cap indentation so deep nodes remain readable.
                // Compute max depth directly from the model (before make_tree_sources).
                let max_d = crate::tree::model_max_depth(&model.roots);
                let indent_per_level = if max_d > 6 {
                    // Cap at 96pt total: deepest level gets at least a few points.
                    (96.0_f64 / max_d as f64).max(4.0)
                } else {
                    16.0
                };
                ov.setIndentationPerLevel(indent_per_level);
                ov.setUsesAlternatingRowBackgroundColors(false);
            }
            // Create data source + delegate.
            let mut sources = crate::tree::make_tree_sources(
                mtm, model, node_id, handler_select, handler_activate,
            );
            unsafe {
                ov.setDataSource(Some(ProtocolObject::from_ref(&*sources.data)));
                ov.setDelegate(Some(ProtocolObject::from_ref(&*sources.delegate)));
                // FR-018: wire doubleAction to a ButtonTarget for OnActivate on tree.
                if let Some(h) = handler_activate {
                    let activate_target = ButtonTarget::with_availability(
                        mtm, h, node_id, rn_core::data::AvailabilityRule::Always,
                    );
                    ov.setDoubleAction(Some(objc2::sel!(fire:)));
                    let obj: &AnyObject = &activate_target;
                    ov.setTarget(Some(obj));
                    sources.activate = Some(Retained::cast(activate_target));
                }
            }
            // Expand all rows that were initially marked expanded.
            unsafe { ov.expandItem_expandChildren(None, true); }
            // Mount outline view inside scroll view.
            unsafe { sv.setDocumentView(Some(&*ov)); }
            stash_delegate(node_id, DelegateHandle::Tree(sources));
            attach_to_parent(state, parent_id, &sv);
            state.nodes.insert(node_id, NativeHandle::TreeWidget(sv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // SidebarSection — small caps group header inside a sidebar
        // ------------------------------------------------------------------
        ComponentKind::SidebarSection => {
            // Rendered by the sidebar's NSOutlineView as a group row (read at promote
            // time). The node is a 0x0 placeholder, never shown.
            let placeholder: Retained<NSView> = unsafe {
                NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)))
            };
            attach_to_parent(state, parent_id, &placeholder);
            state.nodes.insert(node_id, NativeHandle::SidebarItemWidget(placeholder));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // TabView — native NSTabView
        // ------------------------------------------------------------------
        ComponentKind::TabView => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
            let tab_view: Retained<NSTabView> =
                unsafe { NSTabView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &tab_view);
            state.nodes.insert(node_id, NativeHandle::TabViewWidget(tab_view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // TabItem — one NSTabViewItem whose page hosts the children
        // ------------------------------------------------------------------
        ComponentKind::TabItem => {
            let label = match props.get(&PropKey::Text) {
                Some(PropValue::String(s)) => s.clone(),
                _ => String::new(),
            };
            let page: Retained<NSView> = unsafe {
                NSView::initWithFrame(
                    mtm.alloc(),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 280.0)),
                )
            };
            // NSTabView sizes its pages itself; letting them resize with the tab
            // view is the native contract. Pages are deliberately not part of flex
            // layout, so nothing competes with that.
            unsafe {
                page.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable
                    | NSAutoresizingMaskOptions::NSViewHeightSizable);
            }

            let mut added = false;
            if let Some(NativeHandle::TabViewWidget(tab_view)) =
                parent_id.and_then(|pid| state.nodes.get(&pid))
            {
                let item: Retained<NSTabViewItem> = unsafe {
                    let it = NSTabViewItem::new();
                    it.setLabel(&NSString::from_str(&label));
                    it.setView(Some(&page));
                    tab_view.addTabViewItem(&it);
                    it
                };
                // The tab view owns its items; keep ours alive for the node's life.
                stash_delegate(node_id, DelegateHandle::TabItem(item));
                added = true;
            }
            if !added {
                // A TabItem outside a TabView has nowhere to go; render its page
                // inline rather than dropping the content silently.
                attach_to_parent(state, parent_id, &page);
            }

            state.nodes.insert(node_id, NativeHandle::View(
                page,
                ContainerInfo { axis: Axis::Vertical, padding: 0.0, gap: 0.0 },
            ));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // ActivityIndicator — NSProgressIndicator indeterminate (FR-010)
        // ------------------------------------------------------------------
        ComponentKind::ActivityIndicator => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(32.0, 32.0));
            let pi: Retained<NSProgressIndicator> = unsafe {
                NSProgressIndicator::initWithFrame(mtm.alloc(), frame)
            };
            unsafe {
                pi.setIndeterminate(true);
                pi.setStyle(objc2_app_kit::NSProgressIndicatorStyle::Spinning);
                if bool_prop(&props, PropKey::Visible, true) {
                    pi.startAnimation(None);
                }
            }
            attach_to_parent(state, parent_id, &pi);
            state.nodes.insert(node_id, NativeHandle::ActivityIndicatorWidget(pi));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Modal — NSPanel floating window (FR-011)
        // ------------------------------------------------------------------
        ComponentKind::Modal => {
            let visible = bool_prop(&props, PropKey::ModalVisible, false);
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            if !visible {
                view.setHidden(true);
            }
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::View(view, ContainerInfo { axis: Axis::Vertical, padding: 0.0, gap: 0.0 }));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Popover — NSView placeholder (FR-012)
        // ------------------------------------------------------------------
        ComponentKind::Popover => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            view.setHidden(true);
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::View(view, ContainerInfo { axis: Axis::Vertical, padding: 0.0, gap: 0.0 }));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // RichText — NSTextView with NSAttributedString (FR-008, FR-009)
        // ------------------------------------------------------------------
        ComponentKind::RichText => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 60.0));
            let tv: Retained<NSTextView> = unsafe {
                NSTextView::initWithFrame(mtm.alloc(), frame)
            };
            unsafe {
                tv.setEditable(false);
                tv.setSelectable(false);
                tv.setDrawsBackground(false);
            }
            // Build attributed string from spans (FR-008)
            if let Some(PropValue::Spans(spans)) = props.get(&PropKey::TextSpans) {
                if !spans.is_empty() {
                    let attr_str = build_attributed_string(spans, mtm);
                    if let Some(mut storage) = unsafe { tv.textStorage() } {
                        storage.setAttributedString(&attr_str);
                    } else {
                        // Fallback: plain text
                        let plain = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
                        let ns = NSString::from_str(&plain);
                        unsafe { tv.setString(&ns) };
                    }
                }
            }
            // Apply text wrap setting (FR-009)
            let wrap = bool_prop(&props, PropKey::TextWrap, true);
            if let Some(tc) = unsafe { tv.textContainer() } {
                unsafe {
                    tc.setWidthTracksTextView(wrap);
                    tc.setHeightTracksTextView(false);
                    if !wrap {
                        tc.setContainerSize(NSSize::new(f64::MAX, f64::MAX));
                    }
                }
            }
            attach_to_parent(state, parent_id, &tv);
            state.nodes.insert(node_id, NativeHandle::RichTextWidget(tv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Card — NSBox with rounded corners and background (FR-020)
        // ------------------------------------------------------------------
        ComponentKind::Card => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 100.0));
            let card: Retained<NSBox> = unsafe { NSBox::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                card.setBoxType(NSBoxType::NSBoxCustom);
                card.setContentViewMargins(NSSize::new(0.0, 0.0));
                card.setCornerRadius(RadiusToken::Xl.px() as f64);
                card.setBorderWidth(1.0);
                card.setBorderColor(&NSColor::separatorColor());
                card.setFillColor(&NSColor::controlBackgroundColor());
            }
            attach_to_parent(state, parent_id, &card);
            state.nodes.insert(node_id, NativeHandle::CardWidget(card));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Badge — NSTextField tinted pill (FR-021)
        // ------------------------------------------------------------------
        ComponentKind::Badge => {
            let label_text = if let Some(PropValue::String(s)) = props.get(&PropKey::BadgeLabel) {
                s.clone()
            } else {
                String::new()
            };
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(60.0, 20.0));
            let tf: Retained<NSTextField> = unsafe {
                NSTextField::initWithFrame(mtm.alloc(), frame)
            };
            unsafe {
                tf.setEditable(false);
                tf.setBordered(false);
                tf.setDrawsBackground(false);
                tf.setStringValue(&NSString::from_str(&label_text));
                tf.setAlignment(objc2_app_kit::NSTextAlignment::Center);
            }
            attach_to_parent(state, parent_id, &tf);
            state.nodes.insert(node_id, NativeHandle::BadgeWidget(tf));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Skeleton — NSView with shimmer placeholder (FR-022)
        // ------------------------------------------------------------------
        ComponentKind::Skeleton => {
            let w = f32_prop(&props, PropKey::Width, 100.0);
            let h = f32_prop(&props, PropKey::Height, 20.0);
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w as f64, h as f64));
            // Skeleton renders as a gray NSBox placeholder (shimmer via CALayer in future)
            let skel_box: Retained<NSBox> = unsafe { NSBox::initWithFrame(mtm.alloc(), frame) };
            unsafe { skel_box.setBoxType(NSBoxType::NSBoxCustom) };
            attach_to_parent(state, parent_id, &skel_box);
            state.nodes.insert(node_id, NativeHandle::SkeletonWidget(unsafe { Retained::cast(skel_box) }));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // SegmentedControl — NSSegmentedControl (FR-023)
        // ------------------------------------------------------------------
        ComponentKind::SegmentedControl => {
            let options: Vec<String> = if let Some(PropValue::StringList(opts)) = props.get(&PropKey::Options) {
                opts.clone()
            } else {
                vec![]
            };
            let selected = f32_prop(&props, PropKey::SelectedIndex, 0.0) as usize;
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 24.0));
            let sc: Retained<NSSegmentedControl> = unsafe {
                let s = NSSegmentedControl::initWithFrame(mtm.alloc(), frame);
                s.setSegmentCount(options.len() as isize);
                for (i, opt) in options.iter().enumerate() {
                    s.setLabel_forSegment(&NSString::from_str(opt), i as isize);
                }
                s.setSelectedSegment(selected as isize);
                s
            };
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    sc.setTarget(Some(obj));
                    sc.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &sc);
            state.nodes.insert(node_id, NativeHandle::SegmentedControlWidget(sc));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // BackForwardControl — the paired nav pill at the top-left of a
        // System Settings-style content pane. A real NSSegmentedControl,
        // momentary tracking, two chevron segments; wired to OnPress per
        // segment via a tiny target that reports which side fired.
        // ------------------------------------------------------------------
        ComponentKind::BackForwardControl => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(50.0, 24.0));
            let sc: Retained<NSSegmentedControl> = unsafe {
                let s = NSSegmentedControl::initWithFrame(mtm.alloc(), frame);
                s.setSegmentCount(2);
                s.setTrackingMode(objc2_app_kit::NSSegmentSwitchTracking::Momentary);
                for (i, name) in ["chevron.left", "chevron.right"].iter().enumerate() {
                    if let Some(img) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                        &NSString::from_str(name),
                        None,
                    ) {
                        s.setImage_forSegment(Some(&img), i as isize);
                    }
                    s.setWidth_forSegment(24.0, i as isize);
                }
                s
            };
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    sc.setTarget(Some(obj));
                    sc.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            // The real pill lives in the window's titlebar band, flush with
            // the traffic lights — not inline in the scrolling content. A
            // genuine NSTitlebarAccessoryViewController pinned to the left
            // gets that for free; the control plays no part in this
            // window's taffy tree (build_spec excludes BackForwardControl
            // everywhere), so it's never positioned a second time.
            if let Some(window) = enclosing_window(state, parent_id) {
                unsafe {
                    let wrapper = NSView::initWithFrame(
                        mtm.alloc(),
                        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(66.0, 28.0)),
                    );
                    sc.setFrame(NSRect::new(NSPoint::new(8.0, 2.0), NSSize::new(50.0, 24.0)));
                    wrapper.addSubview(&sc);
                    let accessory = NSTitlebarAccessoryViewController::new(mtm);
                    accessory.setView(&wrapper);
                    accessory.setLayoutAttribute(NSLayoutAttribute::Left);
                    window.addTitlebarAccessoryViewController(&accessory);
                }
            } else {
                attach_to_parent(state, parent_id, &sc);
                register_child(state, parent_id, node_id);
            }
            state.nodes.insert(node_id, NativeHandle::SegmentedControlWidget(sc));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
        }

        // ------------------------------------------------------------------
        // SearchBar — NSSearchField (FR-024)
        // ------------------------------------------------------------------
        ComponentKind::SearchBar => {
            let placeholder = if let Some(PropValue::String(s)) = props.get(&PropKey::Placeholder) {
                s.clone()
            } else {
                String::from("Search…")
            };
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 24.0));
            let sf: Retained<NSSearchField> = unsafe {
                let f = NSSearchField::initWithFrame(mtm.alloc(), frame);
                f.setPlaceholderString(Some(&NSString::from_str(&placeholder)));
                f
            };
            if let Some(PropValue::HandlerRef(_h)) = props.get(&PropKey::OnChange) {
                // NSSearchField's delegate must be NSSearchFieldDelegate.
                // For now wire change detection via target/action (set on the field itself).
                // Full delegate wiring deferred — the field still renders and accepts input.
            }
            attach_to_parent(state, parent_id, &sf);
            state.nodes.insert(node_id, NativeHandle::SearchBarWidget(sf));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Sidebar — NSView with vibrancy sidebar styling (FR-025)
        // ------------------------------------------------------------------
        ComponentKind::Sidebar => {
            let width = f32_prop(&props, PropKey::SidebarWidth, 220.0);
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width as f64, 400.0));
            // A source list is an NSOutlineView in a scroll view. The outline's
            // rows, cells, selection, keyboard nav and disclosure are AppKit's; we
            // only supply the data (populated at promote time, once the item/section
            // children have been created). The scroll view is the sidebar's view.
            let scroll = new_scroll_container(mtm, frame, false);
            let outline: Retained<NSOutlineView> = unsafe { NSOutlineView::new(mtm) };
            let column = unsafe {
                NSTableColumn::initWithIdentifier(mtm.alloc(), &NSString::from_str("main"))
            };
            unsafe {
                outline.setStyle(NSTableViewStyle::SourceList);
                outline.setHeaderView(None);
                outline.setFloatsGroupRows(false);
                outline.setIndentationPerLevel(0.0);
                // Default NSTableColumn width (~100pt) has no relation to the
                // sidebar's actual width — every row truncated regardless of
                // how wide the sidebar really is. apply_rects keeps this in
                // sync on every relayout; this is just the initial value so
                // the very first paint isn't wrong too.
                column.setWidth(width as f64 - 4.0);
                outline.addTableColumn(&column);
                outline.setOutlineTableColumn(Some(&column));
                scroll.setDocumentView(Some(&outline));
            }
            attach_to_parent(state, parent_id, &scroll);
            state.nodes.insert(node_id, NativeHandle::SidebarWidget(unsafe { Retained::cast(scroll) }));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        ComponentKind::SidebarItem => {
            // Rows are rendered by the sidebar's NSOutlineView, which reads this
            // node's props at promote time (build_sidebar_data). The node itself is
            // just a 0x0 placeholder so the shadow tree and child bookkeeping stay
            // consistent; it is never shown.
            let placeholder: Retained<NSView> = unsafe {
                NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)))
            };
            attach_to_parent(state, parent_id, &placeholder);
            state.nodes.insert(node_id, NativeHandle::SidebarItemWidget(placeholder));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // GroupedSection — NSBox grouped style (FR-027)
        // ------------------------------------------------------------------
        ComponentKind::GroupedSection => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(300.0, 100.0));
            let gb: Retained<NSBox> = unsafe { NSBox::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                gb.setBoxType(NSBoxType::NSBoxCustom);
                // Remove contentView margins so children sit flush with our layout rects
                gb.setContentViewMargins(NSSize::new(0.0, 0.0));
                // Subtle card background + border
                gb.setFillColor(&NSColor::controlBackgroundColor());
                gb.setBorderColor(&NSColor::separatorColor());
                gb.setBorderWidth(1.0);
                gb.setCornerRadius(RadiusToken::Xl.px() as f64);
            }
            if let Some(PropValue::String(title)) = props.get(&PropKey::SectionTitle) {
                unsafe { gb.setTitle(&NSString::from_str(title)) };
            } else {
                unsafe { gb.setTitlePosition(NSTitlePosition::NSNoTitle) };
            }
            attach_to_parent(state, parent_id, &gb);
            state.nodes.insert(node_id, NativeHandle::GroupedSectionWidget(gb));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // SplitView — panes laid out by taffy, not by an NSSplitView
        //
        // An NSSplitView arranges its own arranged subviews, which duplicates
        // layout authority: taffy computes pane rects and AppKit computes
        // different ones, and the children of each pane end up positioned
        // against a frame that never matches. Constitution VI makes the layout
        // result the sole source of geometry, so this is a plain container and
        // taffy positions every pane.
        //
        // Draggable dividers are a separate concern: they adjust the split
        // ratios in the application tree, which re-runs layout. That keeps one
        // authority instead of two.
        // ------------------------------------------------------------------
        ComponentKind::SplitView => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(600.0, 400.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::SplitViewWidget(view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Toolbar — NSView hosting toolbar-style buttons (FR-029)
        // ------------------------------------------------------------------
        ComponentKind::Toolbar => {
            // An NSToolbar belongs to the window and lives in its titlebar. It is not
            // a view, so the content tree keeps only a zero-height placeholder and the
            // window chrome owns the space and the geometry.
            if let Some(PropValue::ToolbarActions(actions)) = props.get(&PropKey::ToolbarActions) {
                if let Some(window) = enclosing_window(state, parent_id) {
                    let delegate = ToolbarDelegate::new(
                        mtm,
                        ToolbarIvars {
                            actions: actions.clone(),
                            source: node_id,
                            window,
                            keep_alive: Default::default(),
                        },
                    );
                    let obj: &AnyObject = &delegate;
                    let _ = unsafe {
                        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                            0.0,
                            obj,
                            sel!(install:),
                            None,
                            false,
                        )
                    };
                    stash_delegate(node_id, DelegateHandle::Toolbar(delegate));
                }
            }
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
            let container: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &container);
            state.nodes.insert(node_id, NativeHandle::ToolbarWidget(container));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }


        ComponentKind::Switch => {
            let value = bool_prop(&props, PropKey::SwitchValue, false);
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(50.0, 26.0));
            let sw: Retained<NSSwitch> = unsafe { NSSwitch::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                sw.setState(if value {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnChange) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                unsafe {
                    let obj: &AnyObject = &target;
                    sw.setTarget(Some(obj));
                    sw.setAction(Some(sel!(fire:)));
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }
            attach_to_parent(state, parent_id, &sw);
            state.nodes.insert(node_id, NativeHandle::SwitchWidget(sw));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // StatusBar — NSWindow title bar color (FR-014)
        // ------------------------------------------------------------------
        ComponentKind::StatusBar => {
            // StatusBar operates at the window level, not as a child view.
            // Find the nearest window and apply titlebarAppearsTransparent.
            // We render a zero-size placeholder view.
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
            let view: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            attach_to_parent(state, parent_id, &view);
            state.nodes.insert(node_id, NativeHandle::View(view, ContainerInfo { axis: Axis::Vertical, padding: 0.0, gap: 0.0 }));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }
    }
    Ok(())
}


/// Build (or rebuild) a ListTile's subviews — icon tile, title, subtitle,
/// trailing chevron, and the full-row click target — from `props`. Shared by
/// `create_node` and `update_node`: a ListTile's content is a composite of
/// several subviews with no per-field update path, so an update just clears
/// and rebuilds them, the same as creation. Without this, a prop change
/// (e.g. re-selecting a different sidebar item swaps a GroupedSection row's
/// Text/IconName at the same tree position) was silently dropped — the
/// reconciler diffed it as an Update, not a Create, and nothing handled it,
/// leaving stale text visible under a freshly-updated header next to it.
fn populate_list_tile(mtm: MainThreadMarker, row: &NSView, node_id: NodeId, props: &Props) {
    unsafe {
        for sv in row.subviews().to_vec() {
            sv.removeFromSuperview();
        }
    }

    let has_icon = props.contains_key(&PropKey::IconName);
    let mut text_x: f64 = 12.0;

    // Leading icon. With IconColor set, this is a Settings-style
    // colored tile (squircle, or a round avatar when IconShape is
    // "circle" — the Apple Account/Family rows); without it, a bare
    // template glyph, for rows that don't carry a brand color.
    if let Some(PropValue::String(icon_name)) = props.get(&PropKey::IconName) {
        let tint = match props.get(&PropKey::IconColor) {
            Some(PropValue::String(s)) => Some(s.clone()),
            _ => None,
        };
        let is_circle = matches!(
            props.get(&PropKey::IconShape),
            Some(PropValue::String(s)) if s == "circle"
        );
        let tile_size: f64 = if is_circle { 32.0 } else { 26.0 };
        let tile_origin = NSPoint::new(12.0, (44.0 - tile_size) / 2.0);

        if let Some(tint_name) = tint {
            let tile = unsafe {
                NSBox::initWithFrame(
                    mtm.alloc(),
                    NSRect::new(tile_origin, NSSize::new(tile_size, tile_size)),
                )
            };
            unsafe {
                tile.setBoxType(NSBoxType::NSBoxCustom);
                tile.setCornerRadius(if is_circle { tile_size / 2.0 } else { 6.0 });
                tile.setFillColor(&crate::renderer::icon_tint(&tint_name));
                tile.setBorderWidth(0.0);
                tile.setContentViewMargins(NSSize::new(0.0, 0.0));
            }
            let inner = NSRect::new(
                NSPoint::new(tile_size * 0.22, tile_size * 0.22),
                NSSize::new(tile_size * 0.56, tile_size * 0.56),
            );
            let glyph: Retained<NSImageView> = unsafe {
                let v = NSImageView::initWithFrame(mtm.alloc(), inner);
                v.setImageScaling(objc2_app_kit::NSImageScaling::NSImageScaleProportionallyUpOrDown);
                v
            };
            let ns_name = NSString::from_str(icon_name);
            if let Some(img) = unsafe {
                NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None)
            } {
                unsafe {
                    img.setTemplate(true);
                    glyph.setImage(Some(&img));
                    glyph.setContentTintColor(Some(&NSColor::whiteColor()));
                }
            }
            unsafe {
                if let Some(cv) = tile.contentView() {
                    cv.addSubview(&glyph);
                }
            }
            unsafe { row.addSubview(&tile) };
        } else {
            let iv: Retained<NSImageView> = unsafe {
                NSImageView::initWithFrame(
                    mtm.alloc(),
                    NSRect::new(tile_origin, NSSize::new(tile_size, tile_size)),
                )
            };
            let ns_name = NSString::from_str(icon_name);
            if let Some(img) = unsafe {
                NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None)
            } {
                unsafe { iv.setImage(Some(&img)) }
            }
            unsafe { row.addSubview(&iv) };
        }
        text_x = 12.0 + tile_size + 12.0;
    }

    let has_subtitle = props.contains_key(&PropKey::Subtitle);
    let title_text = match props.get(&PropKey::Text) {
        Some(PropValue::String(s)) => s.clone(),
        _ => String::new(),
    };

    // Title
    let title_h: f64 = 17.0;
    let title_y = if has_subtitle {
        24.0
    } else {
        (44.0 - title_h) / 2.0
    };
    let tf: Retained<NSTextField> = unsafe {
        let t = NSTextField::initWithFrame(
            mtm.alloc(),
            NSRect::new(
                NSPoint::new(text_x, title_y),
                NSSize::new(400.0 - text_x - 40.0, title_h),
            ),
        );
        t.setEditable(false);
        t.setBordered(false);
        t.setDrawsBackground(false);
        t.setStringValue(&NSString::from_str(&title_text));
        t.setFont(Some(&NSFont::systemFontOfSize(NSFont::systemFontSize())));
        t.setTextColor(Some(&NSColor::labelColor()));
        t.setLineBreakMode(NSLineBreakMode(4));
        t.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
        t
    };
    unsafe { row.addSubview(&tf) };

    // Subtitle
    if let Some(PropValue::String(sub)) = props.get(&PropKey::Subtitle) {
        let stf: Retained<NSTextField> = unsafe {
            let t = NSTextField::initWithFrame(
                mtm.alloc(),
                NSRect::new(
                    NSPoint::new(text_x, 8.0),
                    NSSize::new(400.0 - text_x - 40.0, 14.0),
                ),
            );
            t.setEditable(false);
            t.setBordered(false);
            t.setDrawsBackground(false);
            t.setStringValue(&NSString::from_str(sub));
            t.setFont(Some(&NSFont::systemFontOfSize(NSFont::smallSystemFontSize())));
            t.setTextColor(Some(&NSColor::secondaryLabelColor()));
            t.setLineBreakMode(NSLineBreakMode(4));
            t.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
            t
        };
        unsafe { row.addSubview(&stf) };
    }

    // Trailing disclosure chevron
    if bool_prop(props, PropKey::ShowChevron, true) {
        let chev_frame = NSRect::new(NSPoint::new(400.0 - 26.0, 14.0), NSSize::new(16.0, 16.0));
        let civ: Retained<NSImageView> = unsafe { NSImageView::initWithFrame(mtm.alloc(), chev_frame) };
        let ns_name = NSString::from_str("chevron.right");
        if let Some(img) = unsafe {
            NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None)
        } {
            unsafe { civ.setImage(Some(&img)) }
        }
        unsafe {
            civ.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewMinXMargin);
            row.addSubview(&civ);
        }
        let _ = has_icon;
    }

    // Click target
    if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnPress) {
        let target = ButtonTarget::new(mtm, *h, node_id);
        unsafe {
            let btn: Retained<NSButton> = NSButton::initWithFrame(
                mtm.alloc(),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 44.0)),
            );
            btn.setBezelStyle(NSBezelStyle::Automatic);
            btn.setBordered(false);
            btn.setTransparent(true);
            btn.setTitle(&NSString::from_str(""));
            btn.setAutoresizingMask(
                NSAutoresizingMaskOptions::NSViewWidthSizable
                    | NSAutoresizingMaskOptions::NSViewHeightSizable,
            );
            let obj: &AnyObject = &target;
            btn.setTarget(Some(obj));
            btn.setAction(Some(sel!(fire:)));
            row.addSubview(&btn);
        }
        stash_delegate(node_id, DelegateHandle::Button(target));
    }
}

fn update_node(
    state: &mut RendererState,
    node_id: NodeId,
    changed: Props,
) -> Result<(), RenderError> {
    let Some(handle) = state.nodes.get(&node_id) else {
        return Err(RenderError::InvalidNode(node_id));
    };

    // ListTile has no per-field update path — its content (icon tile,
    // title, subtitle, chevron, click target) is a handful of subviews
    // whose positions depend on each other, so any relevant prop change
    // just rebuilds all of them from the merged props, the same work
    // `create_node` does. See `populate_list_tile`.
    if let NativeHandle::ListTileWidget(row) = handle {
        let touches_content = changed.keys().any(|k| {
            matches!(
                k,
                PropKey::Text
                    | PropKey::Subtitle
                    | PropKey::IconName
                    | PropKey::IconColor
                    | PropKey::IconShape
                    | PropKey::ShowChevron
                    | PropKey::OnPress
            )
        });
        if touches_content {
            if let Some(mtm) = MainThreadMarker::new() {
                let mut merged = state
                    .shadow
                    .get(&node_id)
                    .map(|sn| sn.props.clone())
                    .unwrap_or_default();
                for (k, v) in &changed {
                    merged.insert(*k, v.clone());
                }
                let row = row.clone();
                populate_list_tile(mtm, &row, node_id, &merged);
            }
        }
    }

    for (key, value) in &changed {
        match (handle, key) {
            (NativeHandle::Window(w), PropKey::Title) => {
                if let PropValue::String(s) = value {
                    let ns = NSString::from_str(s);
                    w.setTitle(&ns);
                }
            }
            (NativeHandle::Label(f), PropKey::Text)
            | (NativeHandle::TextInput(f), PropKey::Text) => {
                if let PropValue::String(s) = value {
                    let ns = NSString::from_str(s);
                    unsafe { f.setStringValue(&ns) };
                }
            }
            (NativeHandle::Button(b), PropKey::Text) => {
                if let PropValue::String(s) = value {
                    let ns = NSString::from_str(s);
                    unsafe { b.setTitle(&ns) };
                }
            }
            (NativeHandle::Button(b), PropKey::Enabled) => {
                if let PropValue::Bool(enabled) = value {
                    b.setEnabled(*enabled);
                }
            }
            (NativeHandle::TextInput(f), PropKey::Enabled) => {
                if let PropValue::Bool(enabled) = value {
                    f.setEnabled(*enabled);
                }
            }
            (_, PropKey::Visible) => {
                if let (Some(view), PropValue::Bool(visible)) = (handle.as_view(), value) {
                    view.setHidden(!*visible);
                }
            }
            (NativeHandle::SwitchWidget(sw), PropKey::SwitchValue) => {
                if let PropValue::Bool(on) = value {
                    unsafe {
                        sw.setState(if *on { NSControlStateValueOn } else { NSControlStateValueOff });
                    }
                }
            }
            (NativeHandle::SegmentedControlWidget(sc), PropKey::SelectedIndex) => {
                if let PropValue::F32(f) = value {
                    unsafe { sc.setSelectedSegment(*f as isize) };
                }
            }
            (NativeHandle::SidebarWidget(_), PropKey::Selected) => {} // outline view owns selection
            // SidebarItem selection is owned by the NSOutlineView now; nothing to
            // repaint by hand. The pill and manual repaint this replaced are gone.
            (NativeHandle::SidebarItemWidget(_), PropKey::Selected) => {}
            // Phase A convergence: live-update table and tree on model change.
            (NativeHandle::TableWidget(sv), PropKey::TableModelKey) => {
                if let PropValue::TableModelProp(new_model) = value {
                    DELEGATES.with(|m| {
                        if let Some(DelegateHandle::Table(sources)) = m.borrow().get(&node_id) {
                            // Preserve selection: remove RowIds no longer in new model.
                            let valid: Vec<rn_core::data::RowId> =
                                new_model.rows.iter().map(|r| r.id).collect();
                            {
                                let mut model = sources.inner.model.borrow_mut();
                                model.selection.filter_to(&valid);
                            }
                            crate::table::update_table_model(sources, new_model.clone());
                            // FR-014: toggle empty-state label on live refresh.
                            let now_empty = new_model.rows.is_empty();
                            let subviews = unsafe { sv.subviews() };
                            // Remove any existing empty-state label (identified by tag 0xE0).
                            for sv_idx in 0..subviews.count() {
                                let subview = unsafe { subviews.objectAtIndex(sv_idx) };
                                let tag = unsafe { subview.tag() };
                                if tag == 0xE0 {
                                    unsafe { subview.removeFromSuperview() };
                                    break;
                                }
                            }
                            if now_empty {
                                if let Some(mtm) = MainThreadMarker::new() {
                                    let lbl = unsafe {
                                        NSTextField::labelWithString(&NSString::from_str("No items"), mtm)
                                    };
                                    unsafe {
                                        lbl.setAlignment(objc2_app_kit::NSTextAlignment::Center);
                                        lbl.setTag(0xE0);
                                        sv.addSubview(&lbl);
                                    }
                                }
                            }
                            // Reload the embedded NSTableView.
                            let doc = unsafe { sv.documentView() };
                            if let Some(doc) = doc {
                                let tv: &NSTableView =
                                    unsafe { &*(Retained::as_ptr(&doc) as *const NSTableView) };
                                // FR-012: remember first visible row before reload.
                                let visible_rect = unsafe { sv.documentVisibleRect() };
                                let first_visible_row = unsafe { tv.rowAtPoint(visible_rect.origin) };
                                unsafe { tv.reloadData() };
                                // Restore scroll: clamp to new row count.
                                let new_row_count = unsafe { tv.numberOfRows() };
                                if new_row_count > 0 && first_visible_row >= 0 {
                                    let target = first_visible_row.min(new_row_count - 1);
                                    unsafe { tv.scrollRowToVisible(target) };
                                }
                                // SC-006/FR-009: restore visual selection for surviving RowIds.
                                {
                                    let model = sources.inner.model.borrow();
                                    for sel_id in &model.selection.selected {
                                        if let Some(row_pos) = model.rows.iter().position(|r| r.id == *sel_id) {
                                            let idx_set = unsafe {
                                                objc2_foundation::NSIndexSet::indexSetWithIndex(row_pos as objc2_foundation::NSUInteger)
                                            };
                                            unsafe { tv.selectRowIndexes_byExtendingSelection(&idx_set, true) };
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
            (NativeHandle::TreeWidget(sv), PropKey::TreeModelKey) => {
                if let PropValue::TreeModelProp(new_model) = value {
                    DELEGATES.with(|m| {
                        if let Some(DelegateHandle::Tree(sources)) = m.borrow().get(&node_id) {
                            // Capture expanded ids: union of model's initial expanded and
                            // any the user opened interactively.
                            let model_expanded = sources.inner.model.borrow().find_expanded();
                            let interactive_expanded = sources.inner.expanded_ids.borrow().clone();
                            let mut expanded: Vec<rn_core::data::RowId> = model_expanded;
                            for id in interactive_expanded {
                                if !expanded.contains(&id) { expanded.push(id); }
                            }
                            crate::tree::update_tree_model(sources, new_model.clone());
                            // Reload and restore expansion.
                            let doc = unsafe { sv.documentView() };
                            if let Some(doc) = doc {
                                let ov: &NSOutlineView =
                                    unsafe { &*(Retained::as_ptr(&doc) as *const NSOutlineView) };
                                unsafe { ov.reloadData() };
                                crate::tree::restore_expansion(ov, sources, &expanded);
                                // SC-006/FR-016: restore visual selection for the surviving selected RowId.
                                let selected_id = sources.inner.model.borrow().selection;
                                if let Some(sel_id) = selected_id {
                                    let n_rows = unsafe { ov.numberOfRows() };
                                    for row in 0..n_rows {
                                        let item = unsafe { ov.itemAtRow(row) };
                                        if let Some(item) = item {
                                            let num: &objc2_foundation::NSNumber =
                                                unsafe { &*(Retained::as_ptr(&item) as *const objc2_foundation::NSNumber) };
                                            let idx = num.longLongValue();
                                            if idx >= 0 {
                                                let snap = sources.inner.snapshot.borrow();
                                                if snap.get(idx as usize).is_some_and(|n| n.id == sel_id) {
                                                    let idx_set = unsafe {
                                                        objc2_foundation::NSIndexSet::indexSetWithIndex(row as objc2_foundation::NSUInteger)
                                                    };
                                                    unsafe { ov.selectRowIndexes_byExtendingSelection(&idx_set, false) };
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
            _ => {}
        }
    }
    if let Some(sn) = state.shadow.get_mut(&node_id) {
        for (k, v) in changed {
            sn.props.insert(k, v);
        }
    }
    // After the shadow is updated, refresh ContainerInfo for a container.
    if let Some(sn) = state.shadow.get(&node_id) {
        let padding = f32_prop(&sn.props, PropKey::Padding, 0.0);
        let gap = f32_prop(&sn.props, PropKey::Gap, 0.0);
        if let Some(NativeHandle::View(_, info)) = state.nodes.get_mut(&node_id) {
            info.padding = padding;
            info.gap = gap;
        }
    }
    Ok(())
}

fn delete_node(state: &mut RendererState, node_id: NodeId) -> Result<(), RenderError> {
    if let Some(handle) = state.nodes.remove(&node_id) {
        match &handle {
            NativeHandle::Window(w) => w.close(),
            other => {
                if let Some(view) = other.as_view() {
                    unsafe { view.removeFromSuperview() };
                }
            }
        }
    }
    if let Some(sn) = state.shadow.remove(&node_id) {
        if let Some(parent) = sn.parent {
            if let Some(pn) = state.shadow.get_mut(&parent) {
                pn.children.retain(|c| *c != node_id);
            }
        }
    }
    state.windows.retain(|n| *n != node_id);
    clear_delegate(node_id);
    Ok(())
}

fn reorder_children(
    state: &mut RendererState,
    parent_id: NodeId,
    order: Vec<NodeId>,
) -> Result<(), RenderError> {
    let parent_view = match state.nodes.get(&parent_id) {
        Some(NativeHandle::View(v, _)) => v.clone(),
        _ => return Ok(()),
    };
    for id in &order {
        if let Some(handle) = state.nodes.get(id) {
            if let Some(view) = handle.as_view() {
                unsafe { view.removeFromSuperview() };
                unsafe { parent_view.addSubview(view) };
            }
        }
    }
    if let Some(pn) = state.shadow.get_mut(&parent_id) {
        pn.children = order;
    }
    Ok(())
}

fn register_child(state: &mut RendererState, parent_id: Option<NodeId>, child: NodeId) {
    if let Some(p) = parent_id {
        if let Some(pn) = state.shadow.get_mut(&p) {
            pn.children.push(child);
        }
    }
}

fn attach_to_parent(state: &RendererState, parent_id: Option<NodeId>, view: &NSView) {
    let Some(pid) = parent_id else {
        return;
    };
    let Some(handle) = state.nodes.get(&pid) else {
        return;
    };
    // Only three containers host their children somewhere other than themselves.
    // Everything else is a plain view, so there is no reason to name it.
    unsafe {
        match handle {
            NativeHandle::Window(w) => {
                if let Some(content) = w.contentView() {
                    content.addSubview(view);
                }
            }
            // A scroll view's children belong to its document, not to the clip view.
            NativeHandle::ScrollViewWidget(sv) | NativeHandle::FlatListWidget(sv, _) => {
                match sv.documentView() {
                    Some(doc) => doc.addSubview(view),
                    None => sv.addSubview(view),
                }
            }
            // NSBox hosts children in its content view so they sit inside the box.
            NativeHandle::GroupedSectionWidget(b) | NativeHandle::CardWidget(b) => {
                match b.contentView() {
                    Some(cv) => cv.addSubview(view),
                    None => b.addSubview(view),
                }
            }
            other => {
                if let Some(parent) = other.as_view() {
                    parent.addSubview(view);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Host a window's SplitView root in an `NSSplitViewController`.
///
/// This is the only way a sidebar reaches the top of the window. An `NSToolbar`
/// paints its background across the full window width, so a sidebar laid into a
/// plain content view is always covered at the top by the toolbar band. A
/// split-view *sidebar item* is drawn by AppKit above that band and gets the
/// vibrancy, the full height, and the draggable divider for free.
///
/// Detection is deliberately narrow: the window's single root child must be a
/// `SplitView` whose first child is a `Sidebar`. Any other shape is left to the
/// plain content-view path, so this cannot regress windows that do not opt in.
///
/// Each pane keeps its own `NSView`; taffy still lays out the pane's interior. The
/// controller owns only the geometry *between* panes (widths, the divider), which
/// is exactly the AppKit-owns-chrome / taffy-owns-interior split the constitution
/// calls for.
fn promote_split_window(state_cell: &Rc<RefCell<RendererState>>, window_id: NodeId) {
    // Resolve everything we need under a short borrow, then DROP it: the AppKit
    // calls below (`setContentViewController` especially) lay the window out
    // synchronously and re-enter our state through the window delegate. Holding a
    // borrow across them panics with "already borrowed".
    let (mtm, window, pane_views): (MainThreadMarker, Retained<NSWindow>, SplitPanes) = {
        let state = state_cell.borrow();
        let mtm = state.mtm;
        let Some(NativeHandle::Window(window)) = state.nodes.get(&window_id) else {
            return;
        };
        let Some(window_shadow) = state.shadow.get(&window_id) else {
            return;
        };
        let Some(&split_id) = window_shadow.children.first() else {
            return;
        };
        let Some(split_shadow) = state.shadow.get(&split_id) else {
            return;
        };
        // A split-hosted window is one whose root is a SplitView, or simply an
        // horizontal stack, whose first child is a Sidebar. Both express the same
        // intent — a sidebar beside content — so both get the real split controller.
        let root_is_splittable = matches!(
            split_shadow.kind,
            ComponentKind::SplitView | ComponentKind::LayoutContainer(Axis::Horizontal)
        );
        if !root_is_splittable {
            return;
        }
        let pane_ids = split_shadow.children.clone();
        let first_is_sidebar = pane_ids.first().is_some_and(|p| {
            state
                .shadow
                .get(p)
                .is_some_and(|s| s.kind == ComponentKind::Sidebar)
        });
        if !first_is_sidebar || pane_ids.len() < 2 {
            return;
        }
        let mut pane_views = Vec::new();
        for pane_id in &pane_ids {
            // A manual Separator between sidebar and content is redundant once the
            // split controller draws its own divider; skip it so it does not become a
            // 1pt resizable pane of its own.
            if state
                .shadow
                .get(pane_id)
                .is_some_and(|s| s.kind == ComponentKind::Separator)
            {
                continue;
            }
            if let Some(v) = state.nodes.get(pane_id).and_then(|h| h.as_view()).map(|v| v.retain()) {
                pane_views.push((*pane_id, v));
            }
        }
        (mtm, window.clone(), pane_views)
    };

    // No borrow held from here on.
    let split_controller = unsafe { NSSplitViewController::new(mtm) };
    let mut recorded: Vec<(NodeId, Retained<NSView>)> = Vec::new();

    for (idx, (pane_id, pane_view)) in pane_views.into_iter().enumerate() {
        unsafe { pane_view.removeFromSuperview() };
        let vc = unsafe { NSViewController::new(mtm) };
        unsafe { vc.setView(&pane_view) };

        let is_sidebar = idx == 0;
        let item = unsafe {
            if is_sidebar {
                NSSplitViewItem::sidebarWithViewController(&vc)
            } else {
                NSSplitViewItem::splitViewItemWithViewController(&vc)
            }
        };
        if is_sidebar {
            unsafe {
                item.setMinimumThickness(180.0);
                item.setMaximumThickness(320.0);
                item.setCanCollapse(true);
            }
        }
        unsafe { split_controller.addSplitViewItem(&item) };
        recorded.push((pane_id, pane_view));
    }

    unsafe { window.setContentViewController(Some(&split_controller)) };

    // Re-borrow only now, to record the panes.
    state_cell.borrow_mut().split_panes.insert(window_id, recorded.clone());

    // Populate each sidebar pane's outline view from its item/section children.
    // Done here, after the whole tree exists, because the data source is a pull
    // model: the children were created after the Sidebar node itself.
    for (pane_id, pane_view) in &recorded {
        populate_sidebar(state_cell, *pane_id, pane_view);
    }

    log::debug!(
        "promote_split_window: {:?} hosted in NSSplitViewController",
        window_id
    );
}

/// Build the outline view's data from a sidebar node's shadow children and attach
/// a SidebarModel as its data source and delegate. No-op if the pane is not a
/// Sidebar (content panes have no outline view).
fn populate_sidebar(state_cell: &Rc<RefCell<RendererState>>, pane_id: NodeId, pane_view: &NSView) {
    // Read the sidebar's children under a short borrow and build the data snapshot.
    let (mtm, data, is_sidebar) = {
        let state = state_cell.borrow();
        let is_sidebar = state
            .shadow
            .get(&pane_id)
            .is_some_and(|s| s.kind == ComponentKind::Sidebar);
        if !is_sidebar {
            return;
        }
        (state.mtm, build_sidebar_data(&state, pane_id), true)
    };
    let _ = is_sidebar;
    let (data, initial_selected_row) = data;

    // The pane view is the scroll view; its document is the outline view.
    let scroll: &NSScrollView = unsafe { &*(pane_view as *const NSView as *const NSScrollView) };
    let Some(doc) = (unsafe { scroll.documentView() }) else {
        return;
    };
    let outline: &NSOutlineView = unsafe { &*(Retained::as_ptr(&doc) as *const NSOutlineView) };

    // SearchBar/ListTile rows declared inside the sidebar are pinned above
    // the outline, like the real Settings search field + Apple Account +
    // Family cards — not among the scrollable items. Detach the tree-built
    // views, stack them top-down under the titlebar band, and inset the
    // document so no row scrolls under them.
    let pinned: Vec<(Retained<NSView>, f64)> = {
        let state = state_cell.borrow();
        let children = state.shadow.get(&pane_id).map(|s| s.children.clone()).unwrap_or_default();
        find_pinned_sidebar_views(&state, &children)
            .into_iter()
            .filter_map(|nid| {
                let handle = state.nodes.get(&nid)?;
                match handle {
                    NativeHandle::SearchBarWidget(sf) => {
                        Some((unsafe { Retained::cast::<NSView>(sf.clone()) }, 26.0))
                    }
                    NativeHandle::ListTileWidget(row) => {
                        Some((row.clone(), 44.0))
                    }
                    _ => None,
                }
            })
            .collect()
    };

    let model = SidebarModel::new(mtm, data, initial_selected_row);
    unsafe {
        outline.setDataSource(Some(ProtocolObject::from_ref(&*model)));
        outline.setDelegate(Some(ProtocolObject::from_ref(&*model)));
        outline.reloadData();
        // Apply the app's initial selection: the row whose Selected prop is true.
        if let Some(row) = model.ivars().initial_selected_row {
            let set = NSIndexSet::indexSetWithIndex(row as NSUInteger);
            outline.selectRowIndexes_byExtendingSelection(&set, false);
        }
    }
    if !pinned.is_empty() {
        let bounds = scroll.frame();
        let safe_top = unsafe { scroll.safeAreaInsets() }.top;
        let pad = 6.0_f64;
        // The sidebar's NSScrollView is a flipped view (y=0 at the top,
        // growing down) — the usual AppKit convention is bottom-left origin,
        // but ours sets isFlipped so row/document math reads top-down like
        // everything else in this renderer. Stack the pinned band the same
        // way: start just below the titlebar safe area and grow downward.
        let mut y = safe_top + pad;
        for (view, height) in &pinned {
            unsafe {
                view.removeFromSuperview();
                view.setFrame(NSRect::new(
                    NSPoint::new(pad, y),
                    NSSize::new(bounds.size.width - pad * 2.0, *height),
                ));
                view.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
                scroll.addSubview(view);
            }
            y += height;
        }
        // A hairline separator between the pinned header band and the item
        // list, the way the real Settings sidebar breaks after the account
        // cards.
        let sep_y = y + 4.0;
        let sep = unsafe {
            NSBox::initWithFrame(
                mtm.alloc(),
                NSRect::new(NSPoint::new(pad, sep_y), NSSize::new(bounds.size.width - pad * 2.0, 1.0)),
            )
        };
        unsafe {
            sep.setBoxType(NSBoxType::NSBoxCustom);
            sep.setFillColor(&NSColor::separatorColor());
            sep.setBorderWidth(0.0);
            sep.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
            scroll.addSubview(&sep);
        }
        unsafe {
            scroll.setAutomaticallyAdjustsContentInsets(false);
            scroll.setContentInsets(NSEdgeInsets {
                top: sep_y + 8.0,
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
            });
        }
        state_cell.borrow_mut().sidebar_top_insets.insert(pane_id, sep_y + 8.0);
    }
    stash_delegate(pane_id, DelegateHandle::Sidebar(model));
}

/// Flatten a sidebar node's children into outline entries, in tree order.
/// A SidebarSection becomes a group header row; a SidebarItem becomes a
/// selectable row. Items before any section simply appear at the top of the
/// list — no implicit untitled group (it rendered as a blank label row).
/// Returns the entries plus the outline row index the app marked Selected.
fn build_sidebar_data(state: &RendererState, sidebar_id: NodeId) -> (SidebarData, Option<usize>) {
    let mut entries: Vec<SidebarEntry> = Vec::new();
    let mut selected_row: Option<usize> = None;

    let children: Vec<NodeId> = state
        .shadow
        .get(&sidebar_id)
        .map(|s| s.children.clone())
        .unwrap_or_default();

    let rows = flatten_sidebar_children(state, &children);

    for (kind, node_id, props) in rows {
        match kind {
            ComponentKind::SidebarSection => {
                let title = string_prop(&props, PropKey::Text, "");
                // An empty title means "no visual group here" — skip the header.
                if !title.is_empty() {
                    entries.push(SidebarEntry::Header { title });
                }
            }
            ComponentKind::SidebarItem => {
                let label = string_prop(&props, PropKey::Text, "");
                let icon = match props.get(&PropKey::IconName) {
                    Some(PropValue::String(s)) => Some(s.clone()),
                    _ => None,
                };
                let icon_color = match props.get(&PropKey::IconColor) {
                    Some(PropValue::String(s)) => Some(s.clone()),
                    _ => None,
                };
                let handler = match props.get(&PropKey::OnSelect) {
                    Some(PropValue::HandlerRef(h)) => Some(*h),
                    _ => None,
                };
                let subtitle = match props.get(&PropKey::Subtitle) {
                    Some(PropValue::String(s)) => Some(s.clone()),
                    _ => None,
                };
                let avatar = matches!(
                    props.get(&PropKey::IconShape),
                    Some(PropValue::String(s)) if s == "circle"
                );
                if bool_prop(&props, PropKey::Selected, false) {
                    selected_row = Some(entries.len());
                }
                entries.push(SidebarEntry::Item { node_id, label, icon, icon_color, handler, subtitle, avatar });
            }
            _ => {}
        }
    }

    (SidebarData { entries }, selected_row)
}

/// Collect every SearchBar/ListTile descendant of a sidebar, in tree order.
/// These are "pinned" rows — the search field and Apple Account/Family
/// cards real Settings keeps fixed above the scrolling item list — and
/// `populate_sidebar` stacks their already-built native views into a
/// header band instead of letting them scroll with the outline.
fn find_pinned_sidebar_views(state: &RendererState, children: &[NodeId]) -> Vec<NodeId> {
    let mut found = Vec::new();
    for &child in children {
        let Some(shadow) = state.shadow.get(&child) else { continue };
        if matches!(shadow.kind, ComponentKind::SearchBar | ComponentKind::ListTile) {
            found.push(child);
            continue;
        }
        found.extend(find_pinned_sidebar_views(state, &shadow.children));
    }
    found
}


/// Collect (kind, node, props) for every SidebarItem/SidebarSection anywhere under
/// the sidebar, in tree order. Recurses through any wrapping containers (a
/// ScrollView+vstack is common) so the outline data does not depend on how deep
/// the app nests its rows.
fn flatten_sidebar_children(
    state: &RendererState,
    children: &[NodeId],
) -> Vec<(ComponentKind, NodeId, Props)> {
    let mut out = Vec::new();
    collect_sidebar_rows(state, children, &mut out);
    out
}

fn collect_sidebar_rows(
    state: &RendererState,
    children: &[NodeId],
    out: &mut Vec<(ComponentKind, NodeId, Props)>,
) {
    for &child in children {
        let Some(shadow) = state.shadow.get(&child) else { continue };
        match shadow.kind {
            ComponentKind::SidebarItem | ComponentKind::SidebarSection => {
                out.push((shadow.kind, child, shadow.props.clone()));
            }
            _ => collect_sidebar_rows(state, &shadow.children, out),
        }
    }
}

fn relayout_window(state: &RendererState, window_id: NodeId) {
    // A split-hosted window has no single content view to lay into; each pane is an
    // independent NSView owned by the split controller. Run taffy inside each pane.
    if let Some(panes) = state.split_panes.get(&window_id) {
        for (pane_id, pane_view) in panes {
            let frame = pane_view.frame();
            // A split-view pane (the sidebar especially) runs under the titlebar, so
            // its safe-area top inset is the band to keep rows clear of. AppKit
            // reports it per pane; the sidebar's includes the traffic lights.
            let top = unsafe { pane_view.safeAreaInsets() }.top as f32;
            let pane_rect = Rect {
                x: 0.0,
                y: top,
                width: frame.size.width as f32,
                height: (frame.size.height as f32 - top).max(0.0),
            };
            let Some(spec) = build_spec(state, *pane_id) else {
                continue;
            };
            let result = compute_layout(&spec, pane_rect);
            apply_rects(state, pane_view, &result);
        }
        ensure_selection_visible(state);
        return;
    }

    let (content_rect, content_view) = {
        let Some(NativeHandle::Window(window)) = state.nodes.get(&window_id) else {
            log::warn!("relayout_window: no Window handle for {:?}", window_id);
            return;
        };
        let Some(cv) = window.contentView() else {
            log::warn!("relayout_window: no contentView for {:?}", window_id);
            return;
        };
        let frame = cv.frame();
        log::debug!(
            "relayout_window: contentView frame = {}x{}",
            frame.size.width,
            frame.size.height
        );
        (
            Rect {
                x: 0.0,
                y: 0.0,
                width: frame.size.width as f32,
                height: frame.size.height as f32,
            },
            cv,
        )
    };
    let Some(window_shadow) = state.shadow.get(&window_id) else {
        log::warn!("relayout_window: no shadow for window {:?}", window_id);
        return;
    };
    log::debug!(
        "relayout_window: window shadow has {} children",
        window_shadow.children.len()
    );
    let Some(root_child) = window_shadow.children.first().copied() else {
        log::warn!("relayout_window: window shadow has no children — layout skipped");
        return;
    };
    let Some(spec) = build_spec(state, root_child) else {
        log::warn!("relayout_window: build_spec returned None for root_child {:?}", root_child);
        return;
    };
    log::debug!(
        "relayout_window: spec has {} children, computing layout in {}x{}",
        spec.children.len(),
        content_rect.width,
        content_rect.height
    );
    // A full-size content view runs under the titlebar, so it must start below the
    // band or the first row of every pane hides behind the traffic lights.
    let inset = titlebar_inset(&content_view);
    let content_rect = Rect {
        y: content_rect.y + inset,
        height: (content_rect.height - inset).max(0.0),
        ..content_rect
    };
    let result = compute_layout(&spec, content_rect);
    log::debug!("relayout_window: {} rects computed", result.rects.len());
    // Log first 6 rects by node id for layout debugging
    let mut sorted: Vec<_> = result.rects.iter().collect();
    sorted.sort_by_key(|(id, _)| id.0);
    for (id, r) in sorted.iter().take(6) {
        log::debug!("  rect {:?}: x={:.0} y={:.0} w={:.0} h={:.0}", id.0, r.x, r.y, r.width, r.height);
    }
    apply_rects(state, &content_view, &result);
}

fn build_spec(state: &RendererState, node_id: NodeId) -> Option<LayoutSpec> {
    let shadow = state.shadow.get(&node_id)?;
    let axis = match shadow.kind {
        ComponentKind::LayoutContainer(Axis::Horizontal) => LayoutAxis::Horizontal,
        ComponentKind::LayoutContainer(Axis::Vertical) => LayoutAxis::Vertical,
        // Phase-2 containers act as vertical stacks for their children
        ComponentKind::SplitView => LayoutAxis::Horizontal, // panes side-by-side
        ComponentKind::Sidebar
        | ComponentKind::SafeAreaView
        | ComponentKind::GroupedSection
        | ComponentKind::Card
        | ComponentKind::TouchableOpacity
        | ComponentKind::Modal
        | ComponentKind::Popover
        | ComponentKind::Scaffold
        | ComponentKind::ContentArea => LayoutAxis::Vertical,
        _ => LayoutAxis::None,
    };
    let padding = f32_prop(&shadow.props, PropKey::Padding, 0.0);
    let gap = f32_prop(&shadow.props, PropKey::Gap, 0.0);
    let min_height = f32_opt(&shadow.props, PropKey::MinHeight);
    // Scrollable containers get `min-height: 0`. Without it flex layout applies its
    // automatic minimum, which is the content height — so a scroll view refuses to
    // shrink into its parent and overflows it instead (a 2010pt list inside a 720pt
    // sidebar). Same reason CSS sets `min-height: 0` on scroll containers.
    let min_height = if matches!(
        shadow.kind,
        ComponentKind::ScrollView | ComponentKind::FlatList | ComponentKind::ContentArea
    ) {
        Some(min_height.unwrap_or(0.0))
    } else {
        min_height
    };
    let (fixed_width, fixed_height) = match shadow.kind {
        ComponentKind::Label => {
            // A wrapped label needs height for every line it may occupy.
            let lines = f32_prop(&shadow.props, PropKey::MaxLines, 1.0).max(1.0);
            let wrap = bool_prop(&shadow.props, PropKey::TextWrap, false);
            // `lines` counts on its own: asking for 2 lines is asking to wrap,
            // even when TextWrap was never set. Only counting `wrap` here is why
            // the second line was clipped — the text wrapped into 40pt of space
            // that the box never allocated.
            let rows = if wrap || lines > 1.0 { lines } else { 1.0 };
            (None, Some(20.0 * rows))
        }
        ComponentKind::Button => (Some(120.0), Some(32.0)),
        ComponentKind::TextInput => (Some(200.0), Some(24.0)),
        ComponentKind::Checkbox => (None, Some(22.0)),
        ComponentKind::Slider => (Some(200.0), Some(24.0)),
        ComponentKind::ProgressBar => (Some(200.0), Some(20.0)),
        ComponentKind::Separator => (
            None,
            Some(f32_opt(&shadow.props, PropKey::Height).unwrap_or(8.0)),
        ),
        ComponentKind::TextArea => (None, Some(100.0)),
        ComponentKind::Dropdown => (Some(180.0), Some(26.0)),
        ComponentKind::Image => (Some(32.0), Some(32.0)),
        ComponentKind::Stepper => (Some(120.0), Some(26.0)),
        ComponentKind::ColorWell => (Some(46.0), Some(26.0)),
        ComponentKind::DatePicker => (Some(180.0), Some(26.0)),
        ComponentKind::CapacityIndicator => (Some(200.0), Some(16.0)),
        ComponentKind::RatingIndicator => (Some(120.0), Some(20.0)),
        ComponentKind::RelevanceIndicator => (Some(48.0), Some(16.0)),
        ComponentKind::Tooltip => (None, Some(18.0)),
        ComponentKind::TimePicker => (Some(100.0), Some(26.0)),
        ComponentKind::GraphicalTimePicker => (Some(180.0), Some(160.0)),
        // The toolbar is the window's NSToolbar now; it occupies no space in the
        // content tree.
        ComponentKind::Toolbar => (None, Some(0.0)),
        // Phase A: data-bound surfaces; each occupies one layout node with a default
        // height that grows to fill its parent via flex_grow wired in build_spec.
        ComponentKind::Table => (None, Some(300.0)),
        ComponentKind::Tree => (None, Some(300.0)),
        ComponentKind::ToolbarIconButton => (Some(32.0), Some(28.0)),
        ComponentKind::ToolbarDivider => (Some(1.0), Some(20.0)),
        ComponentKind::ToolbarPopup => (Some(120.0), Some(26.0)),
        // AlertDialog renders a trigger button, not a dialog panel.
        ComponentKind::AlertDialog => (Some(300.0), Some(32.0)),
        // A sheet is a separate window; give it a usable default size when the
        // application does not set one.
        ComponentKind::Sheet => (Some(420.0), Some(280.0)),
        ComponentKind::RadioGroup => {
            let options_count = shadow
                .props
                .get(&PropKey::Options)
                .and_then(|v| if let PropValue::StringList(l) = v { Some(l.len()) } else { None })
                .unwrap_or(0);
            (Some(180.0), Some((options_count as f32) * 24.0))
        }
        // Phase-2 leaf widgets — fixed intrinsic sizes
        ComponentKind::Switch => (Some(51.0), Some(31.0)),
        ComponentKind::ActivityIndicator => (Some(32.0), Some(32.0)),
        ComponentKind::Badge => (Some(60.0), Some(20.0)),
        ComponentKind::Skeleton => (None, Some(20.0)),
        ComponentKind::SegmentedControl => (Some(220.0), Some(28.0)),
        ComponentKind::BackForwardControl => (Some(50.0), Some(24.0)),
        // A search field with no width collapses to its two-point content minimum
        // in a crowded toolbar — it was invisible in the Finder demo.
        ComponentKind::SearchBar => (Some(160.0), Some(28.0)),
        ComponentKind::StatusBar => (None, Some(22.0)),
        ComponentKind::SidebarItem => (None, Some(28.0)),
        ComponentKind::SidebarSection => (None, Some(18.0)),
        ComponentKind::ListTile => (None, Some(44.0)),               
        ComponentKind::TitleBar => (None, Some(28.0)),               
        ComponentKind::IconButton => (Some(28.0), Some(28.0)),
        ComponentKind::DisclosureButton => (None, Some(20.0)),
        ComponentKind::HelpButton => (Some(20.0), Some(20.0)),
        ComponentKind::PopupButton => (Some(160.0), Some(26.0)),
        ComponentKind::BackButton => (Some(32.0), Some(28.0)),
        ComponentKind::RichText => (None, Some(80.0)),
        ComponentKind::FlatList => (None, None),
        // Phase-2 containers — fill from props, fall back to flex/grow
        ComponentKind::Sidebar => {
            // Use SidebarWidth if present, else Width prop
            let w = shadow.props.get(&PropKey::SidebarWidth)
                .or_else(|| shadow.props.get(&PropKey::Width))
                .and_then(|v| if let PropValue::F32(f) = v { Some(*f) } else { None });
            (w, f32_opt(&shadow.props, PropKey::Height))
        },
        ComponentKind::GroupedSection => (None, None), // auto-sized from children
        ComponentKind::Card => (None, None), // auto-sized from children
        _ => (
            f32_opt(&shadow.props, PropKey::Width),
            f32_opt(&shadow.props, PropKey::Height),
        ),
    };
    // An explicit size prop always beats the intrinsic default. Without this the
    // table above silently won and a caller could not resize, say, a ScrollView.
    let fixed_width = f32_opt(&shadow.props, PropKey::Width).or(fixed_width);
    let fixed_height = f32_opt(&shadow.props, PropKey::Height).or(fixed_height);


    // Labels with no explicit width use a minimum of 60px so they don't
    // collapse to zero in hstack layouts where other fixed-width siblings
    // compete for space.
    let min_width = match shadow.kind {
        ComponentKind::Label | ComponentKind::Checkbox => {
            Some(f32_opt(&shadow.props, PropKey::MinWidth).unwrap_or(60.0))
        }
        _ => f32_opt(&shadow.props, PropKey::MinWidth),
    };
    // A declared width is a floor, not a wish. Without this flex shrink overrides
    // it whenever a sibling's content is wider — which is how a 200pt sidebar
    // collapsed to 71pt next to a wide wrapping grid. CSS behaves the same way:
    // flex items shrink below their width unless the minimum is pinned.
    let min_width = min_width.or(fixed_width);
    // A TabView's pages are sized by NSTabView, not by flex layout. Feeding them
    // to taffy would compute a second, competing rect for each page.
    let forces_no_children = matches!(shadow.kind, ComponentKind::TabView);
    let children = if forces_no_children {
        Vec::new()
    } else {
        shadow
            .children
            .iter()
            // A sidebar's SearchBar is lifted into a pinned slot above the
            // outline rows by populate_sidebar; laying it out here too would
            // give it a second, competing rect.
            .filter(|c| {
                !(shadow.kind == ComponentKind::Sidebar
                    && state.shadow.get(*c).is_some_and(|s| matches!(
                        s.kind,
                        ComponentKind::SearchBar | ComponentKind::ListTile
                    )))
            })
            .filter_map(|c| build_spec(state, *c))
            .collect()
    };

    // flex_grow comes directly from the FlexGrow prop — no per-kind guessing.
    // Callers set .flex(1.0) when they want fill-behavior.
    //
    // One structural exception: panes of a SplitView share the available space
    // equally unless they declare a fixed width. That is what a split view *is*,
    // not a layout guess — a split view whose panes all collapse to zero is not
    // a split view.
    let parent_is_splitview = shadow
        .parent
        .and_then(|p| state.shadow.get(&p))
        .is_some_and(|ps| matches!(ps.kind, ComponentKind::SplitView));
    let flex_grow = if parent_is_splitview && fixed_width.is_none() {
        1.0
    } else {
        f32_prop(&shadow.props, PropKey::FlexGrow, 0.0)
    };
    let justify = f32_prop(&shadow.props, PropKey::Justify, 0.0) as u8;
    let align = f32_prop(&shadow.props, PropKey::Align, 0.0) as u8;
    let wrap = f32_prop(&shadow.props, PropKey::Wrap, 0.0) as u8;
    Some(LayoutSpec {
        id: LayoutNodeId(node_id.0),
        axis,
        padding,
        gap,
        min_width,
        min_height,
        fixed_width,
        fixed_height,
        flex_grow,
        justify,
        align,
        wrap,
        children,
    })
}

/// How much of the window the titlebar covers. With a full-size content view this is
/// the traffic-light band, plus the toolbar when the window has one.
fn titlebar_inset(content_view: &NSView) -> f32 {
    let Some(window) = content_view.window() else {
        return 0.0;
    };
    if !window
        .styleMask()
        .contains(NSWindowStyleMask::FullSizeContentView)
    {
        return 0.0;
    }
    let full = content_view.frame().size.height;
    let usable = unsafe { window.contentLayoutRect() }.size.height;
    let inset = full - usable;
    if inset > 0.0 {
        inset as f32
    } else {
        0.0
    }
}

fn apply_rects(state: &RendererState, content_view: &NSView, result: &rn_layout::LayoutResult) {
    let content_height = content_view.frame().size.height as f32;

    // Pre-pass: a scroll view's children live in its document view, which must be
    // tall enough to hold them. Compute that from the children's own rects, then
    // size the document view. Without this, children positioned below the
    // viewport's own height land outside the document and vanish.
    let mut doc_heights: Vec<(NodeId, f32)> = Vec::new();
    for (lid, parent_rect) in &result.rects {
        let node_id = NodeId(lid.0);
        let is_scroll = matches!(
            state.nodes.get(&node_id),
            Some(NativeHandle::ScrollViewWidget(_)) | Some(NativeHandle::FlatListWidget(..))
        );
        if !is_scroll {
            continue;
        }
        let mut max_bottom: f32 = parent_rect.height;
        if let Some(sn) = state.shadow.get(&node_id) {
            for child in &sn.children {
                if let Some(cr) = result.rects.get(&rn_layout::LayoutNodeId(child.0)) {
                    let local_bottom = (cr.y - parent_rect.y) + cr.height;
                    if local_bottom > max_bottom {
                        max_bottom = local_bottom;
                    }
                }
            }
        }
        doc_heights.push((node_id, max_bottom));
    }

    for (node_id, h) in &doc_heights {
        let Some(handle) = state.nodes.get(node_id) else {
            continue;
        };
        let sv = match handle {
            NativeHandle::ScrollViewWidget(sv) => sv,
            NativeHandle::FlatListWidget(sv, _) => sv,
            _ => continue,
        };
        let width = result
            .rects
            .get(&rn_layout::LayoutNodeId(node_id.0))
            .map_or(0.0, |r| r.width);
        let viewport = result
            .rects
            .get(&rn_layout::LayoutNodeId(node_id.0))
            .map_or(0.0, |r| r.height);
        log::debug!(
            "scroll container {:?}: document {:.0}pt, viewport {:.0}pt{}",
            node_id,
            h,
            viewport,
            if *h > viewport { " (scrollable)" } else { " (fits)" }
        );
        if let Some(doc) = unsafe { sv.documentView() } {
            unsafe {
                doc.setFrame(NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(width as f64, *h as f64),
                ));
                // AppKit defers layout of the new document bounds. Without forcing it
                // here, that deferred pass runs after we set a scroll position and
                // overwrites it — which is why reveal appeared to do nothing.
                sv.layoutSubtreeIfNeeded();
            };
        }
    }

    for (lid, rect) in &result.rects {
        let node_id = NodeId(lid.0);
        let Some(handle) = state.nodes.get(&node_id) else {
            continue;
        };
        let Some(view) = handle.as_view() else {
            continue;
        };

        // `rect` is in absolute coords (origin = content view top-left).
        // Each view's frame must be expressed relative to its immediate
        // superview. Look up the shadow parent; if it has a taffy rect,
        // subtract it to get superview-relative coords.
        let shadow_parent_id = state.shadow.get(&node_id).and_then(|sn| sn.parent);

        let (local_x, local_y, superview_height) =
            match shadow_parent_id.and_then(|pid| result.rects.get(&rn_layout::LayoutNodeId(pid.0))) {
                Some(parent_rect) => {
                    // Parent has a taffy rect → coords are relative to parent.
                    let lx = rect.x - parent_rect.x;
                    let ly = rect.y - parent_rect.y;
                    // Inside a scroll view the superview is the document view, whose
                    // height is the content height, not the viewport height. Flipping
                    // against the viewport would place content off the document.
                    let h = shadow_parent_id
                        .and_then(|pid| {
                            doc_heights
                                .iter()
                                .find(|(nid, _)| *nid == pid)
                                .map(|(_, h)| *h)
                        })
                        .unwrap_or(parent_rect.height);
                    (lx, ly, h)
                }
                None => {
                    // No parent rect (direct content-view child or window child).
                    (rect.x, rect.y, content_height)
                }
            };

        // Flip y from taffy (top-left origin) to AppKit (bottom-left origin).
        //
        // Exception: content inside a scroll view lives in a flipped document view,
        // which already has a top-left origin, so no inversion is applied there.
        let parent_is_scroll = shadow_parent_id.is_some_and(|pid| {
            matches!(
                state.nodes.get(&pid),
                Some(NativeHandle::ScrollViewWidget(_))
                    | Some(NativeHandle::FlatListWidget(..))
                )
        });
        let flipped_y = if parent_is_scroll {
            local_y
        } else {
            superview_height - local_y - rect.height
        };
        let ns = NSRect::new(
            NSPoint::new(local_x as f64, flipped_y as f64),
            NSSize::new(rect.width as f64, rect.height as f64),
        );
        log::trace!(
            "setFrame {:?} → x={:.0} y={:.0} w={:.0} h={:.0} (abs:{:.0},{:.0} parent:{:?})",
            node_id, local_x, flipped_y, rect.width, rect.height,
            rect.x, rect.y, shadow_parent_id
        );
        unsafe { view.setFrame(ns) };

        // Re-assert the sidebar's reserved header-band inset every relayout.
        // setFrame touches the scroll view on every single rerender, even
        // when the rect is unchanged — found via rapid-selection stress
        // testing that AppKit can silently drop a manually-set
        // contentInsets.top somewhere in that path, which only ever ran
        // once at promotion time. Re-applying it here is cheap and makes
        // the pinned search/account/family band self-healing instead of a
        // one-shot setting nothing protects afterward.
        if let Some(top_inset) = state.sidebar_top_insets.get(&node_id) {
            if let NativeHandle::SidebarWidget(sv) = handle {
                let sv: &NSScrollView = unsafe { &*(Retained::as_ptr(sv) as *const NSScrollView) };
                unsafe {
                    sv.setAutomaticallyAdjustsContentInsets(false);
                    sv.setContentInsets(NSEdgeInsets {
                        top: *top_inset,
                        left: 0.0,
                        bottom: 0.0,
                        right: 0.0,
                    });
                    // The outline's single column has no automatic tie to the
                    // sidebar's actual width — left at NSTableColumn's ~100pt
                    // default, every row's text truncated regardless of the
                    // real column. Keep it in sync here too.
                    if let Some(doc) = sv.documentView() {
                        let ov: &NSOutlineView = &*(Retained::as_ptr(&doc) as *const NSOutlineView);
                        let cols = ov.tableColumns();
                        if let Some(col) = cols.firstObject() {
                            col.setWidth(sv.frame().size.width - 4.0);
                        }
                    }
                }
            }
        }
    }

    ensure_selection_visible(state);
    present_sheets(state, result);
}

/// Present any sheet that has been laid out but not yet shown.
///
/// Sheets are real windows, so they can only be presented once they have a size.
/// Presentation is recorded so a relayout triggered by an unrelated event does not
/// re-present a sheet the user already dismissed.
fn present_sheets(state: &RendererState, result: &rn_layout::LayoutResult) {
    for (id, handle) in &state.nodes {
        let NativeHandle::SheetWidget(_, sheet_window) = handle else {
            continue;
        };
        if state.presented_sheets.borrow().contains(id) {
            continue;
        }
        let Some(rect) = result.rects.get(&rn_layout::LayoutNodeId(id.0)) else {
            continue;
        };
        let Some(owner) = enclosing_window(state, state.shadow.get(id).and_then(|s| s.parent)) else {
            continue;
        };

        unsafe {
            sheet_window.setContentSize(NSSize::new(rect.width as f64, rect.height as f64));
            owner.beginSheet_completionHandler(sheet_window, None);
        }
        log::debug!(
            "presented sheet {:?}: {:.0}x{:.0}pt",
            id,
            rect.width,
            rect.height
        );
        state.presented_sheets.borrow_mut().insert(*id);
    }
}

/// Reveal each selected row inside whatever scroll view contains it.
///
/// Selection is usually set by the application, not by scrolling, so a deeply
/// selected row can sit far outside the visible part of a long list and the user
/// sees no highlight at all.
///
/// `scrollRectToVisible` is the AppKit primitive for exactly this: it scrolls the
/// nearest enclosing scroll view just far enough, and does nothing when the rect is
/// already visible. That makes this idempotent — it can run on every layout pass —
/// and it removes any coordinate maths of our own, which is where the flipping and
/// document-height assumptions used to go wrong.
fn ensure_selection_visible(state: &RendererState) {
    for (id, sn) in &state.shadow {
        if !matches!(
            sn.kind,
            ComponentKind::SidebarItem | ComponentKind::ListTile
        ) {
            continue;
        }
        if !matches!(sn.props.get(&PropKey::Selected), Some(PropValue::Bool(true))) {
            continue;
        }
        let Some(view) = state.nodes.get(id).and_then(NativeHandle::as_view) else {
            continue;
        };
        let revealed = unsafe { view.scrollRectToVisible(view.bounds()) };
        if revealed {
            log::debug!("revealed selection {:?}", id);
        }
    }
}

// ---------------------------------------------------------------------------
// Prop helpers
// ---------------------------------------------------------------------------

/// The window that owns a node, found by walking up the shadow tree.
///
/// Modal system objects (alerts, sheets) attach to a window rather than being a
/// view inside one, so they need the enclosing window rather than a parent view.
fn enclosing_window(state: &RendererState, start: Option<NodeId>) -> Option<Retained<NSWindow>> {
    let mut node = start?;
    loop {
        if let Some(NativeHandle::Window(w)) = state.nodes.get(&node) {
            return Some(w.clone());
        }
        node = state.shadow.get(&node).and_then(|s| s.parent)?;
    }
}

fn bool_prop(props: &Props, key: PropKey, default: bool) -> bool {
    match props.get(&key) {
        Some(PropValue::Bool(v)) => *v,
        _ => default,
    }
}

/// Build an `NSMutableAttributedString` from a list of `StyledSpan` values.
/// Uses plain text concatenation for now; bold is applied via NSFont.
/// (FR-008 — colored/styled spans via NSAttributedString)
fn build_attributed_string(spans: &[rn_core::StyledSpan], mtm: MainThreadMarker) -> Retained<NSMutableAttributedString> {
    let full_text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    let ns_text = NSString::from_str(&full_text);
    // Build a base attributed string with the full plain text.
    let result: Retained<NSMutableAttributedString> =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_text);
    // Apply per-span font and color attributes using addAttribute:value:range:.
    let mut offset: usize = 0;
    for span in spans {
        let span_len = span.text.chars().count();
        let range = objc2_foundation::NSRange::new(offset, span_len);
        // Font — handle bold, italic, bold+italic (FR-008)
        let font: Retained<NSFont> = unsafe {
            let size: f64 = match span.font_size {
                Some(s) => s as f64,
                None => NSFont::systemFontSize(),
            };
            let base: Retained<NSFont> = if span.bold {
                NSFont::boldSystemFontOfSize(size)
            } else {
                NSFont::systemFontOfSize(size)
            };
            if span.italic {
                // Use NSFontManager to add italic trait
                let fm = NSFontManager::sharedFontManager(mtm);
                let italic_mask = objc2_app_kit::NSFontTraitMask(
                    objc2_app_kit::NSFontTraitMask::NSItalicFontMask.0,
                );
                fm.convertFont_toHaveTrait(&base, italic_mask)
            } else {
                base
            }
        };
        // Use msg_send! directly to call addAttribute:value:range: since Retained<NSMutableAttributedString>
        // doesn't expose &mut methods in objc2 0.2 bindings.
        unsafe {
            let _: () = objc2::msg_send![
                &*result, addAttribute: NSFontAttributeName value: &*font range: range
            ];
        }
        // Color
        if let Some(token) = span.color {
            let ns_color: Retained<NSColor> = unsafe {
                match token {
                    rn_core::ColorToken::Destructive => NSColor::systemRedColor(),
                    rn_core::ColorToken::Muted => NSColor::secondaryLabelColor(),
                    rn_core::ColorToken::Success => NSColor::systemGreenColor(),
                    rn_core::ColorToken::Warning => NSColor::systemOrangeColor(),
                    _ => NSColor::controlTextColor(),
                }
            };
            unsafe {
                let _: () = objc2::msg_send![
                    &*result, addAttribute: NSForegroundColorAttributeName value: &*ns_color range: range
                ];
            }
        }
        offset += span_len;
    }
    result
}

fn f32_prop(props: &Props, key: PropKey, default: f32) -> f32 {
    match props.get(&key) {
        Some(PropValue::F32(v)) => *v,
        _ => default,
    }
}

fn f32_opt(props: &Props, key: PropKey) -> Option<f32> {
    match props.get(&key) {
        Some(PropValue::F32(v)) if *v > 0.0 => Some(*v),
        _ => None,
    }
}

fn string_prop(props: &Props, key: PropKey, default: &str) -> String {
    match props.get(&key) {
        Some(PropValue::String(s)) => s.clone(),
        _ => default.to_string(),
    }
}

fn size_from_props(props: &Props, dw: f32, dh: f32) -> (f32, f32) {
    (
        f32_prop(props, PropKey::Width, dw),
        f32_prop(props, PropKey::Height, dh),
    )
}

// ---------------------------------------------------------------------------
// declare_class! trampolines
// ---------------------------------------------------------------------------

// A document view whose coordinate origin is top-left.
//
// AppKit views are bottom-left origin by default, so a tall document view opens
// scrolled to its *bottom* — which is why a freshly shown list starts at the end.
// Flipping the document removes that and, as a side effect, removes the need to
// invert y for anything inside a scroll view.
declare_class!(
    struct FlippedView;

    unsafe impl ClassType for FlippedView {                          
        type Super = NSView;                                         
        type Mutability = mutability::MainThreadOnly;                
        const NAME: &'static str = "RnFlippedView";                  
    }                                                                

    impl DeclaredClass for FlippedView {
        type Ivars = ();
    }

    unsafe impl FlippedView {                                        
        #[method(isFlipped)]                                         
        fn is_flipped(&self) -> bool {                               
            true                                                     
        }                                                            
    }                                                                
);                                                                   

/// A scroll container with a flipped document view already installed.
///
/// Every scrolled surface needs the same three things: a vertical scroller, a
/// document view to hold the content, and a flipped document so it opens at the
/// top rather than the bottom.
fn new_scroll_container(
    mtm: MainThreadMarker,
    frame: NSRect,
    draws_background: bool,
) -> Retained<NSScrollView> {
    let sv: Retained<NSScrollView> = unsafe { NSScrollView::initWithFrame(mtm.alloc(), frame) };
    unsafe {
        sv.setHasVerticalScroller(true);
        sv.setHasHorizontalScroller(false);
        sv.setAutohidesScrollers(true);
        sv.setDrawsBackground(draws_background);
        if draws_background {
            sv.setBackgroundColor(&NSColor::windowBackgroundColor());
        }
        let doc = new_document_view(mtm, frame.size);
        sv.setDocumentView(Some(&doc));
    }
    sv
}

/// Create a flipped document view, the host for scrolled content.
fn new_document_view(mtm: MainThreadMarker, size: NSSize) -> Retained<NSView> {
    unsafe {
        let view: Retained<FlippedView> = msg_send_id![
            mtm.alloc::<FlippedView>(),
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), size)
        ];
        Retained::cast(view)
    }
}

/// One flattened sidebar entry. Headers and items are SIBLINGS at the top
/// level of the outline — the way real source lists (Finder, System Settings)
/// work — instead of nesting items under sections, which would indent them
/// and add disclosure triangles they never had.
#[derive(Clone)]
enum SidebarEntry {
    Header { title: String },
    Item {
        node_id: NodeId,
        label: String,
        icon: Option<String>,
        icon_color: Option<String>,
        handler: Option<HandlerId>,
        /// Account/Family-style rows: a subtitle line, a round avatar tile
        /// instead of a squircle, and a taller row — rendered as a genuine
        /// outline row (not a pinned subview) so AppKit's SourceList style
        /// gives it the exact same leading inset/alignment every other row
        /// gets automatically, instead of us trying to replicate it by hand.
        subtitle: Option<String>,
        avatar: bool,
    },
}

/// NSOutlineView is a pull model — it asks us for children and cells — so the
/// reconciler's push-built child views are replaced by this snapshot. Outline
/// items are `NSNumber` indices into `entries`.
struct SidebarData {
    entries: Vec<SidebarEntry>,
}

struct SidebarModelIvars {
    data: SidebarData,
    /// The fully-expanded outline row index the app marked Selected, if any.
    initial_selected_row: Option<usize>,
}

declare_class!(
    struct SidebarModel;

    unsafe impl ClassType for SidebarModel {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnSidebarModel";
    }

    impl DeclaredClass for SidebarModel {
        type Ivars = SidebarModelIvars;
    }

    unsafe impl NSObjectProtocol for SidebarModel {}

    // NSOutlineViewDelegate requires NSControlTextEditingDelegate as a supertrait;
    // we use none of its methods.
    unsafe impl NSControlTextEditingDelegate for SidebarModel {}

    unsafe impl NSOutlineViewDataSource for SidebarModel {
        // All entries are top-level siblings; nothing nests.
        #[method(outlineView:numberOfChildrenOfItem:)]
        unsafe fn number_of_children(&self, _ov: &NSOutlineView, item: Option<&AnyObject>) -> NSInteger {
            match item {
                Some(_) => 0,
                None => self.ivars().data.entries.len() as NSInteger,
            }
        }

        #[method_id(outlineView:child:ofItem:)]
        unsafe fn child_of_item(&self, _ov: &NSOutlineView, index: NSInteger, _item: Option<&AnyObject>) -> Retained<AnyObject> {
            Retained::cast::<AnyObject>(NSNumber::numberWithLongLong(index as i64))
        }

        #[method(outlineView:isItemExpandable:)]
        unsafe fn is_expandable(&self, _ov: &NSOutlineView, _item: &AnyObject) -> bool {
            false
        }
    }

    unsafe impl NSOutlineViewDelegate for SidebarModel {
        // Headers render as group rows: bold small caps, unselectable, taller
        // with breathing room above the group's items.
        #[method(outlineView:isGroupItem:)]
        unsafe fn is_group_item(&self, _ov: &NSOutlineView, item: &AnyObject) -> bool {
            matches!(self.entry(item), Some(SidebarEntry::Header { .. }))
        }

        // Group header rows get a taller slot so the label isn't flush against
        // the previous group's last item.
        #[method(outlineView:heightOfRowByItem:)]
        unsafe fn height_of_row(&self, _ov: &NSOutlineView, item: &AnyObject) -> f64 {
            match self.entry(item) {
                Some(SidebarEntry::Header { .. }) => SIDEBAR_HEADER_ROW_HEIGHT,
                Some(SidebarEntry::Item { subtitle, avatar, .. }) if subtitle.is_some() || *avatar => {
                    SIDEBAR_AVATAR_ROW_HEIGHT
                }
                _ => SIDEBAR_ITEM_ROW_HEIGHT,
            }
        }

        // Section headings are not selectable; item rows are.
        #[method(outlineView:shouldSelectItem:)]
        unsafe fn should_select(&self, _ov: &NSOutlineView, item: &AnyObject) -> bool {
            matches!(self.entry(item), Some(SidebarEntry::Item { .. }))
        }

        #[method_id(outlineView:viewForTableColumn:item:)]
        unsafe fn view_for_item(&self, _ov: &NSOutlineView, _col: Option<&NSTableColumn>, item: &AnyObject) -> Option<Retained<NSView>> {
            match MainThreadMarker::new() {
                Some(mtm) => match self.entry(item) {
                    Some(SidebarEntry::Header { title }) => {
                        Some(make_sidebar_label(mtm, title, None, None, true, None, false))
                    }
                    Some(SidebarEntry::Item { label, icon, icon_color, subtitle, avatar, .. }) => {
                        Some(make_sidebar_label(
                            mtm, label, icon.as_deref(), icon_color.as_deref(), false,
                            subtitle.as_deref(), *avatar,
                        ))
                    }
                    None => None,
                },
                None => None,
            }
        }

        #[method(outlineViewSelectionDidChange:)]
        unsafe fn selection_changed(&self, notification: &NSNotification) {
            let ov = match notification.object() {
                Some(o) => o,
                None => return,
            };
            let ov: &NSOutlineView = &*(Retained::as_ptr(&ov) as *const NSOutlineView);
            let row = ov.selectedRow();
            if row < 0 {
                return;
            }
            let item = match ov.itemAtRow(row) {
                Some(i) => i,
                None => return,
            };
            if let Some(SidebarEntry::Item { node_id, handler: Some(h), .. }) = self.entry(&item) {
                let (node_id, h) = (*node_id, *h);
                rn_core::dispatch_event(Event {
                    source: node_id,
                    handler: h,
                    kind: EventKind::Press,
                    payload: EventPayload::None,
                    stop_bubbling: false,
                });
            }
        }
    }
);

impl SidebarModel {
    /// Resolve an outline item (an NSNumber index we vended) to its entry.
    fn entry(&self, item: &AnyObject) -> Option<&SidebarEntry> {
        let num: &NSNumber = unsafe { &*(item as *const AnyObject as *const NSNumber) };
        let idx = num.longLongValue();
        if idx < 0 { return None; }
        self.ivars().data.entries.get(idx as usize)
    }
}

impl SidebarModel {
    fn new(mtm: MainThreadMarker, data: SidebarData, initial_selected_row: Option<usize>) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(SidebarModelIvars { data, initial_selected_row });
        unsafe { msg_send_id![super(this), init] }
    }
}

/// Named color tint → NSColor for the System Settings icon tile.
fn icon_tint(name: &str) -> Retained<NSColor> {
    unsafe {
        match name {
            "blue"   => NSColor::systemBlueColor(),
            "green"  => NSColor::systemGreenColor(),
            "red"    => NSColor::systemRedColor(),
            "orange" => NSColor::systemOrangeColor(),
            "purple" => NSColor::systemPurpleColor(),
            "teal"   => NSColor::systemTealColor(),
            "indigo" => NSColor::systemIndigoColor(),
            "yellow" => NSColor::systemYellowColor(),
            "pink"   => NSColor::systemPinkColor(),
            "brown"  => NSColor::systemBrownColor(),
            "gray"   => NSColor::systemGrayColor(),
            _        => NSColor::systemGrayColor(),
        }
    }
}

/// Shared sidebar row metrics — one leading inset and icon size for every
/// row kind, so a group header's text and an item's icon start at exactly
/// the same x, and icon/text sit vertically centered in whatever the row's
/// actual height is (28pt items, 30pt headers — see `height_of_row`). Before
/// this, headers used a hand-picked x=8 while items used x=2 for their
/// icon, and both icon/label used fixed y values sized for a 24pt frame
/// that the outline then stretched to 28/30 without recentering — the
/// "padding and margin is not the same" the user was pointing at.
const SIDEBAR_LEADING_INSET: f64 = 4.0;
// Measured against the real System Settings sidebar (pixel-scanned a
// screenshot of the live app, not guessed): 20pt square icon tiles, 32pt
// item rows. Every category icon — including gray ones like General — is
// a colored squircle tile with a white glyph, never a bare tinted glyph;
// that was the dominant visual gap, bigger than any inset tuning.
const SIDEBAR_ICON_SIZE: f64 = 20.0;
const SIDEBAR_ICON_TEXT_GAP: f64 = 8.0;
const SIDEBAR_ITEM_ROW_HEIGHT: f64 = 32.0;
const SIDEBAR_HEADER_ROW_HEIGHT: f64 = 34.0;
/// Account/Family-style rows: bigger avatar, two lines of text, taller row —
/// same leading inset as every other row, just more content.
const SIDEBAR_AVATAR_ROW_HEIGHT: f64 = 52.0;
const SIDEBAR_AVATAR_SIZE: f64 = 30.0;
/// Squircle corner radius for a sidebar icon tile, proportional to the real
/// app's ratio (measured ~0.2-0.25 of tile size, matching the content
/// header tile's 13/56 ratio).
const SIDEBAR_TILE_CORNER_RADIUS: f64 = 4.5;

#[allow(clippy::too_many_arguments)]
fn make_sidebar_label(
    mtm: MainThreadMarker,
    text: &str,
    icon: Option<&str>,
    icon_color: Option<&str>,
    group: bool,
    subtitle: Option<&str>,
    avatar: bool,
) -> Retained<NSView> {
    let row_height = if group {
        SIDEBAR_HEADER_ROW_HEIGHT
    } else if avatar || subtitle.is_some() {
        SIDEBAR_AVATAR_ROW_HEIGHT
    } else {
        SIDEBAR_ITEM_ROW_HEIGHT
    };
    let container = unsafe {
        NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(250.0, row_height)))
    };
    let icon_size = if avatar { SIDEBAR_AVATAR_SIZE } else { SIDEBAR_ICON_SIZE };
    let content_y = (row_height - icon_size) / 2.0;
    // Every row — header, plain item, or avatar row — starts its icon (or,
    // for headers, its text) at the exact same leading inset. This was the
    // whole point of this pass: Account/Family used to be separate pinned
    // NSViews outside the outline with their own hand-picked inset, which
    // never matched what AppKit's SourceList style gives every real row
    // automatically. Folding them into actual outline entries fixes that
    // at the root instead of trying to replicate AppKit's margin by hand.
    let mut label_x = SIDEBAR_LEADING_INSET;
    if let Some(name) = icon {
        if let Some(image) = unsafe {
            NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)
        } {
            let tint = match icon_color {
                Some(tint_name) => icon_tint(tint_name),
                None => icon_tint("gray"),
            };
            // Every sidebar icon — avatar circle or category squircle — is a
            // colored filled tile with a white glyph, never a bare tinted
            // glyph. Confirmed by pixel-measuring the real System Settings
            // app: even the "neutral" General icon sits on a light gray
            // tile, not a plain glyph with no background at all.
            let corner_radius = if avatar { icon_size / 2.0 } else { SIDEBAR_TILE_CORNER_RADIUS };
            let tile = unsafe {
                NSBox::initWithFrame(
                    mtm.alloc(),
                    NSRect::new(NSPoint::new(SIDEBAR_LEADING_INSET, content_y), NSSize::new(icon_size, icon_size)),
                )
            };
            unsafe {
                tile.setBoxType(NSBoxType::NSBoxCustom);
                tile.setCornerRadius(corner_radius);
                tile.setFillColor(&tint);
                tile.setBorderWidth(0.0);
                tile.setContentViewMargins(NSSize::new(0.0, 0.0));
            }
            let inner = NSRect::new(
                NSPoint::new(icon_size * 0.22, icon_size * 0.22),
                NSSize::new(icon_size * 0.56, icon_size * 0.56),
            );
            let glyph: Retained<NSImageView> = unsafe {
                let v = NSImageView::initWithFrame(mtm.alloc(), inner);
                v.setImageScaling(objc2_app_kit::NSImageScaling::NSImageScaleProportionallyUpOrDown);
                v
            };
            unsafe {
                image.setTemplate(true);
                glyph.setImage(Some(&image));
                glyph.setContentTintColor(Some(&NSColor::whiteColor()));
                if let Some(cv) = tile.contentView() {
                    cv.addSubview(&glyph);
                }
                container.addSubview(&tile);
            }
            label_x = SIDEBAR_LEADING_INSET + icon_size + SIDEBAR_ICON_TEXT_GAP;
        }
    }

    let trailing = if avatar || subtitle.is_some() { SIDEBAR_ICON_SIZE } else { SIDEBAR_LEADING_INSET };
    let label_width = 250.0 - label_x - trailing;
    let label = unsafe { NSTextField::labelWithString(&NSString::from_str(text), mtm) };
    let label_y = if subtitle.is_some() { content_y + 10.0 } else { content_y };
    unsafe {
        label.setFrame(NSRect::new(NSPoint::new(label_x, label_y), NSSize::new(label_width, SIDEBAR_ICON_SIZE)));
        label.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
        label.setBordered(false);
        label.setDrawsBackground(false);
        if group {
            label.setFont(Some(&NSFont::boldSystemFontOfSize(11.0)));
            label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        } else {
            label.setFont(Some(&NSFont::systemFontOfSize(NSFont::systemFontSize())));
        }
        container.addSubview(&label);
    }

    if let Some(sub) = subtitle {
        let stf = unsafe { NSTextField::labelWithString(&NSString::from_str(sub), mtm) };
        unsafe {
            stf.setFrame(NSRect::new(NSPoint::new(label_x, content_y - 8.0), NSSize::new(label_width, 14.0)));
            stf.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable);
            stf.setBordered(false);
            stf.setDrawsBackground(false);
            stf.setFont(Some(&NSFont::systemFontOfSize(NSFont::smallSystemFontSize())));
            stf.setTextColor(Some(&NSColor::secondaryLabelColor()));
            container.addSubview(&stf);
        }
    }

    // Avatar/subtitle rows always navigate somewhere (Account, Family) —
    // the trailing disclosure chevron the real app always shows for them.
    if avatar || subtitle.is_some() {
        let chev_frame = NSRect::new(
            NSPoint::new(250.0 - SIDEBAR_ICON_SIZE - 2.0, (row_height - 14.0) / 2.0),
            NSSize::new(14.0, 14.0),
        );
        let civ: Retained<NSImageView> = unsafe { NSImageView::initWithFrame(mtm.alloc(), chev_frame) };
        if let Some(img) = unsafe {
            NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str("chevron.right"), None)
        } {
            unsafe { civ.setImage(Some(&img)) }
        }
        unsafe {
            civ.setContentTintColor(Some(&NSColor::tertiaryLabelColor()));
            civ.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewMinXMargin);
            container.addSubview(&civ);
        }
    }

    container
}

/// Supplies `NSToolbarItem`s from the actions an application declared.
///
/// An `NSToolbar` is owned by the window and lives in its titlebar; it is not a view
/// in the content tree. That is the whole difference between a real toolbar and the
/// strip of buttons this used to render.
struct ToolbarIvars {
    actions: Vec<rn_core::ToolbarAction>,
    source: NodeId,
    window: Retained<NSWindow>,
    /// Nothing else retains the item targets, and they must outlive the toolbar.
    keep_alive: std::cell::RefCell<Vec<Retained<ButtonTarget>>>,
}

impl std::fmt::Debug for ToolbarIvars {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolbarIvars")
            .field("actions", &self.actions.len())
            .finish()
    }
}

impl ToolbarDelegate {
    fn new(mtm: MainThreadMarker, ivars: ToolbarIvars) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(ivars);
        unsafe { msg_send_id![super(this), init] }
    }
}

/// Separators and flexible space are supplied by AppKit itself, so they are not
/// given an identifier here.
fn toolbar_identifiers(actions: &[rn_core::ToolbarAction]) -> Vec<Retained<NSString>> {
    actions
        .iter()
        .filter(|a| !a.separator)
        .map(|a| NSString::from_str(&a.id))
        .collect()
}

declare_class!(
    struct ToolbarDelegate;

    unsafe impl ClassType for ToolbarDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnToolbarDelegate";
    }

    impl DeclaredClass for ToolbarDelegate {
        type Ivars = ToolbarIvars;
    }

    unsafe impl ToolbarDelegate {
        /// Installing a toolbar makes AppKit lay the window out immediately, which
        /// re-enters our render state and panics on an already-borrowed RefCell.
        /// So the install runs on a later runloop turn, outside the render pass.
        #[method(install:)]
        fn install(&self, _timer: Option<&NSTimer>) {
            let ivars = self.ivars();
            let mtm = match MainThreadMarker::new() {
                Some(m) => m,
                None => return,
            };
            let toolbar = unsafe {
                NSToolbar::initWithIdentifier(mtm.alloc(), &NSString::from_str("rn.toolbar"))
            };
            unsafe {
                toolbar.setDelegate(Some(ProtocolObject::from_ref(self)));
                toolbar.setDisplayMode(NSToolbarDisplayMode::IconAndLabel);
                toolbar.setAllowsUserCustomization(false);
                ivars.window.setToolbar(Some(&toolbar));
                ivars.window.setToolbarStyle(NSWindowToolbarStyle::Unified);
            }
            // The toolbar changed the height of the band the window draws over the
            // content, so every pane has to be inset again. Safe to borrow here: this
            // runs on a timer, outside the render pass.
            with_state(|s| {
                for window_id in s.windows.clone() {
                    relayout_window(s, window_id);
                }
            });
        }

        #[method_id(toolbarDefaultItemIdentifiers:)]
        fn default_identifiers(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSToolbarItemIdentifier>> {
            let ids = toolbar_identifiers(&self.ivars().actions);
            NSArray::from_id_slice(&ids)
        }

        #[method_id(toolbarAllowedItemIdentifiers:)]
        fn allowed_identifiers(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSToolbarItemIdentifier>> {
            let ids = toolbar_identifiers(&self.ivars().actions);
            NSArray::from_id_slice(&ids)
        }

        #[method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:)]
        fn item_for_identifier(
            &self,
            _toolbar: &NSToolbar,
            identifier: &NSToolbarItemIdentifier,
            _will_be_inserted: bool,
        ) -> Option<Retained<NSToolbarItem>> {
            let ivars = self.ivars();
            let want = identifier.to_string();
            let matched = ivars
                .actions
                .iter()
                .find(|a| !a.separator && a.id == want);

            // Written as one expression: the objc2 macro wraps the tail value, not an
            // early `return None`.
            match (MainThreadMarker::new(), matched) {
                (Some(mtm), Some(action)) => {
                    let item = unsafe { NSToolbarItem::initWithItemIdentifier(mtm.alloc(), identifier) };
                    let label = action.label.clone().unwrap_or_else(|| action.id.clone());
                    unsafe {
                        item.setPaletteLabel(&NSString::from_str(&label));
                        item.setLabel(&NSString::from_str(&label));
                        item.setBordered(true);
                        item.setEnabled(action.enabled);
                        if let Some(icon) = &action.icon_name {
                            if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                                &NSString::from_str(icon),
                                Some(&NSString::from_str(&label)),
                            ) {
                                item.setImage(Some(&image));
                            }
                        }
                        let target = ButtonTarget::new(mtm, action.handler, ivars.source);
                        item.setTarget(Some(&target));
                        item.setAction(Some(sel!(fire:)));
                        ivars.keep_alive.borrow_mut().push(target);
                    }
                    Some(item)
                }
                _ => None,
            }
        }
    }
    unsafe impl NSObjectProtocol for ToolbarDelegate {}
    unsafe impl NSToolbarDelegate for ToolbarDelegate {}
);

/// Presents a real `NSAlert` as a sheet on the window that owns the component.
///
/// An alert is a modal system object, not a view: it cannot be a child in the view
/// tree. The component therefore renders as a native button that presents one, which
/// is both honest about what AppKit offers and gives a real alert with real system
/// buttons rather than a painted imitation of one.
#[derive(Debug)]
struct AlertIvars {
    title: String,
    message: String,
    handler: Option<HandlerId>,
    source: NodeId,
    window: Retained<NSWindow>,
}

declare_class!(
    struct AlertTarget;

    unsafe impl ClassType for AlertTarget {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnAlertTarget";
    }

    impl DeclaredClass for AlertTarget {
        type Ivars = AlertIvars;
    }

    unsafe impl AlertTarget {
        #[method(present:)]
        fn present(&self, _sender: Option<&AnyObject>) {
            let ivars = self.ivars();
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let alert = unsafe { NSAlert::new(mtm) };
            unsafe {
                alert.setMessageText(&NSString::from_str(&ivars.title));
                alert.setInformativeText(&NSString::from_str(&ivars.message));
                alert.addButtonWithTitle(&NSString::from_str("OK"));
                alert.addButtonWithTitle(&NSString::from_str("Cancel"));
            }

            let source = ivars.source;
            let handler = ivars.handler;
            // The completion block is copied by AppKit, so a stack-scoped
            // reference-counted block is enough.
            let completion = block2::RcBlock::new(move |response: isize| {
                if let Some(h) = handler {
                    let event = Event {
                        source,
                        handler: h,
                        kind: EventKind::Press,
                        // 1000 = first button (OK), 1001 = second (Cancel).
                        payload: EventPayload::Text(match response {
                            1000 => "OK".to_string(),
                            _ => "Cancel".to_string(),
                        }),
                        stop_bubbling: false,
                    };
                    with_state(|s| s.pending_events.push(event.clone()));
                    rn_core::dispatch_event(event);
                }
            });

            unsafe {
                alert.beginSheetModalForWindow_completionHandler(&ivars.window, Some(&completion));
            }
        }
    }
);

impl AlertTarget {
    fn new(mtm: MainThreadMarker, ivars: AlertIvars) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(ivars);
        unsafe { msg_send_id![super(this), init] }
    }
}

#[derive(Debug)]
struct TargetIvars {
    handler: HandlerId,
    source: NodeId,
    kind: EventKind,
    /// Availability rule for command menu items (FR-003).
    available: rn_core::data::AvailabilityRule,
}

declare_class!(
    struct ButtonTarget;

    unsafe impl ClassType for ButtonTarget {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnButtonTarget";
    }

    impl DeclaredClass for ButtonTarget {
        type Ivars = TargetIvars;
    }

    unsafe impl ButtonTarget {
        #[method(fire:)]
        fn fire(&self, _sender: Option<&AnyObject>) {
            let ivars = self.ivars();
            // Respect the availability rule: Never commands must not fire.
            if ivars.available == rn_core::data::AvailabilityRule::Never { return; }
            // WhenFlag: check current flag state.
            if let rn_core::data::AvailabilityRule::WhenFlag(flag_id) = ivars.available {
                if !crate::commands::flag_is_enabled(flag_id) { return; }
            }
            log::debug!(
                "ButtonTarget::fire — handler={:?} source={:?}",
                ivars.handler,
                ivars.source
            );
            let event = Event {
                source: ivars.source,
                handler: ivars.handler,
                kind: ivars.kind,
                payload: EventPayload::None,
                stop_bubbling: false,
            };
            with_state(|s| s.pending_events.push(event.clone()));
            rn_core::dispatch_event(event);
            // Bubble press up through TouchableOpacity ancestors (FR-015).
            bubble_press_event(ivars.source);
        }
    }
);

impl ButtonTarget {
    fn new(mtm: MainThreadMarker, handler: HandlerId, source: NodeId) -> Retained<Self> {
        Self::with_availability(mtm, handler, source, rn_core::data::AvailabilityRule::Always)
    }
    fn with_availability(
        mtm: MainThreadMarker,
        handler: HandlerId,
        source: NodeId,
        available: rn_core::data::AvailabilityRule,
    ) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(TargetIvars {
            handler,
            source,
            kind: EventKind::Press,
            available,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

declare_class!(
    struct WindowDelegate;

    unsafe impl ClassType for WindowDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnWindowDelegate";
    }

    impl DeclaredClass for WindowDelegate {
        type Ivars = TargetIvars;
    }

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[method(windowShouldClose:)]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            let ivars = self.ivars();
            let event = Event {
                source: ivars.source,
                handler: ivars.handler,
                kind: EventKind::Close,
                payload: EventPayload::None,
                stop_bubbling: false,
            };
            rn_core::dispatch_event(event);
            true
        }

        /// Re-run taffy layout whenever the window is resized so subview
        /// frames stay correct. Called by AppKit on every live-resize tick.
        #[method(windowDidResize:)]
        fn window_did_resize(&self, _notif: &NSNotification) {
            let window_id = self.ivars().source;
            with_state(|s| relayout_window(s, window_id));
        }

        /// Trigger layout on first display: the contentView frame is 0×0
        /// when `render()` is called before the window is on screen; this
        /// fires once the window actually appears and gives us real geometry.
        #[method(windowDidBecomeKey:)]
        fn window_did_become_key(&self, _notif: &NSNotification) {
            let window_id = self.ivars().source;
            with_state(|s| relayout_window(s, window_id));
        }
    }
);

impl WindowDelegate {
    fn new(mtm: MainThreadMarker, handler: HandlerId, source: NodeId) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(TargetIvars {
            handler,
            source,
            kind: EventKind::Close,
            available: rn_core::data::AvailabilityRule::Always,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

declare_class!(
    struct TextDelegate;

    unsafe impl ClassType for TextDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnTextDelegate";
    }

    impl DeclaredClass for TextDelegate {
        type Ivars = TargetIvars;
    }

    unsafe impl NSObjectProtocol for TextDelegate {}

    unsafe impl NSControlTextEditingDelegate for TextDelegate {
        #[method(controlTextDidChange:)]
        fn control_text_did_change(&self, notif: &NSNotification) {
            let ivars = self.ivars();
            let text = unsafe {
                match notif.object() {
                    Some(obj) => {
                        let field_ptr: *const AnyObject = &*obj;
                        let field = &*(field_ptr as *const NSTextField);
                        field.stringValue().to_string()
                    }
                    None => String::new(),
                }
            };
            let event = Event {
                source: ivars.source,
                handler: ivars.handler,
                kind: EventKind::Change,
                payload: EventPayload::Text(text),
                stop_bubbling: false,
            };
            rn_core::dispatch_event(event);
        }
    }

    unsafe impl NSTextFieldDelegate for TextDelegate {}
);

impl TextDelegate {
    fn new(mtm: MainThreadMarker, handler: HandlerId, source: NodeId) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(TargetIvars {
            handler,
            source,
            kind: EventKind::Change,
            available: rn_core::data::AvailabilityRule::Always,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

declare_class!(
    struct TickTarget;

    unsafe impl ClassType for TickTarget {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "RnTickTarget";
    }

    impl DeclaredClass for TickTarget {
        type Ivars = ();
    }

    unsafe impl TickTarget {
        #[method(fire:)]
        fn fire(&self, _sender: Option<&AnyObject>) {
            let events: Vec<Event> =
                with_state(|s| std::mem::take(&mut s.pending_events)).unwrap_or_default();
            for e in events {
                rn_core::dispatch_event(e);
            }
            // Poll scroll positions for ScrollView nodes that have an OnScroll handler.
            // This fires EventPayload::Scroll when the contentView bounds change.
            let scroll_events = with_state(|s| {
                let mut evts = Vec::new();
                for (node_id, handle) in &s.nodes {
                    if let NativeHandle::ScrollViewWidget(sv) = handle {
                        if let Some(shadow) = s.shadow.get(node_id) {
                            if let Some(PropValue::HandlerRef(h)) = shadow.props.get(&PropKey::OnScroll) {
                                let clip = unsafe { sv.contentView() };
                                let bounds = clip.bounds();
                                let offset_y = bounds.origin.y as f32;
                                let offset_x = bounds.origin.x as f32;
                                let event = Event {
                                    source: *node_id,
                                    handler: *h,
                                    kind: EventKind::Change,
                                    payload: EventPayload::Scroll(ScrollEvent {
                                        offset_x,
                                        offset_y,
                                        velocity_x: 0.0,
                                        velocity_y: 0.0,
                                    }),
                                    stop_bubbling: false,
                                };
                                evts.push(event);
                            }
                        }
                    }
                }
                evts
            }).unwrap_or_default();
            for e in scroll_events {
                rn_core::dispatch_event(e);
            }

            // Poll FlatList virtualization — call on_scroll and dispatch render_item
            // for newly mounted rows (FR-002, FR-004).
            let flat_ops: FlatOpsVec =
                with_state(|s| {
                    let mut result = Vec::new();
                    for (node_id, handle) in s.nodes.iter_mut() {
                        if let NativeHandle::FlatListWidget(sv, fl_state) = handle {
                            if let Some(shadow) = s.shadow.get(node_id) {
                                if let Some(PropValue::HandlerRef(h)) =
                                    shadow.props.get(&PropKey::RenderItem)
                                {
                                    let clip = unsafe { sv.contentView() };
                                    let bounds = clip.bounds();
                                    let offset_y = bounds.origin.y as f32;
                                    let ops = fl_state.on_scroll(offset_y);
                                    if !ops.is_empty() {
                                        result.push((*node_id, *h, ops));
                                    }
                                }
                            }
                        }
                    }
                    result
                })
                .unwrap_or_default();
            for (node_id, handler, ops) in flat_ops {
                for (data_index, is_mount) in ops {
                    if is_mount {
                        // Fire render_item handler with the row index.
                        let evt = Event {
                            source: node_id,
                            handler,
                            kind: EventKind::Change,
                            payload: EventPayload::Index(data_index as i32),
                            stop_bubbling: false,
                        };
                        rn_core::dispatch_event(evt);
                    }
                }
            }
            rn_core::tick();
        }
    }
);

impl TickTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(());
        unsafe { msg_send_id![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// Delegate stash
// ---------------------------------------------------------------------------

#[allow(dead_code)] // Fields exist to keep delegates alive; never read directly.
enum DelegateHandle {
    Button(Retained<ButtonTarget>),
    Window(Retained<WindowDelegate>),
    Text(Retained<TextDelegate>),
    Tick(Retained<TickTarget>),
    Alert(Retained<AlertTarget>),
    TabItem(Retained<NSTabViewItem>),
    /// The window retains the toolbar; the delegate must not die with the render pass.
    Toolbar(Retained<ToolbarDelegate>),
    /// The outline view holds the sidebar model as a weak data-source/delegate ref,
    /// so we must keep it alive here.
    Sidebar(Retained<SidebarModel>),
    /// Phase A: command targets must outlive the menu items that reference them.
    Commands(Vec<Retained<ButtonTarget>>),
    /// Phase A: NSTableView data source + delegate pair.
    Table(crate::table::TableSources),
    /// Phase A: NSOutlineView tree data source + delegate pair.
    Tree(crate::tree::TreeSources),
}

thread_local! {
    static DELEGATES: RefCell<HashMap<NodeId, DelegateHandle>> =
        RefCell::new(HashMap::new());
}

fn stash_delegate(node_id: NodeId, handle: DelegateHandle) {
    DELEGATES.with(|m| {
        m.borrow_mut().insert(node_id, handle);
    });
}

fn clear_delegate(node_id: NodeId) {
    DELEGATES.with(|m| {
        m.borrow_mut().remove(&node_id);
    });
}
