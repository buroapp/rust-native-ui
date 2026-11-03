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
    NSTextView, NSStepper, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask,
    NSBox, NSBoxType, NSImage, NSSearchField, NSSegmentedControl, NSSwitch, NSSplitView,
    NSTitlePosition, NSVisualEffectView, NSVisualEffectMaterial,
    NSVisualEffectState,
};
// Phase 2 extra imports for RichText attributed string rendering (FR-008)
use objc2_app_kit::{NSFont, NSFontManager, NSForegroundColorAttributeName, NSFontAttributeName};
use objc2_foundation::NSMutableAttributedString;
use objc2_foundation::{
    MainThreadMarker, NSDate,
    NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString, NSTimer,
};

use rn_core::{
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
    SplitViewWidget(Retained<NSSplitView>),
    ToolbarWidget(Retained<NSView>),
    RichTextWidget(Retained<NSTextView>),
    TouchableWidget(Retained<NSView>),
    SafeAreaWidget(Retained<NSView>),
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
            NativeHandle::GroupedSectionWidget(v) => Some(v),
            NativeHandle::SplitViewWidget(v) => Some(v),
            NativeHandle::ToolbarWidget(v) => Some(v),
            NativeHandle::RichTextWidget(v) => Some(v),
            NativeHandle::TouchableWidget(v) => Some(v),
            NativeHandle::SafeAreaWidget(v) => Some(v),
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

struct RendererState {
    mtm: MainThreadMarker,
    nodes: HashMap<NodeId, NativeHandle>,
    shadow: HashMap<NodeId, ShadowNode>,
    windows: Vec<NodeId>,
    pending_events: Vec<Event>,
    _tick_timer: Option<Retained<NSTimer>>,
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
        {
            let s = self.state.borrow();
            let roots: Vec<NodeId> = s.windows.clone();
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
            let style = NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Miniaturizable;
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
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnClose) {
                let delegate = WindowDelegate::new(mtm, *h, node_id);
                let proto: &ProtocolObject<dyn NSWindowDelegate> =
                    ProtocolObject::from_ref(&*delegate);
                window.setDelegate(Some(proto));
                stash_delegate(node_id, DelegateHandle::Window(delegate));
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
            let label: Retained<NSTextField> =
                unsafe { NSTextField::labelWithString(&ns_text, mtm) };
            unsafe {
                label.setEditable(false);
                label.setSelectable(false);
                label.setBordered(false);
                label.setDrawsBackground(false);
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
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(32.0, 32.0));
            let image_view: Retained<NSImageView> = unsafe {
                NSImageView::initWithFrame(mtm.alloc(), frame)
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
            attach_to_parent(state, parent_id, &image_view);
            state.nodes.insert(node_id, NativeHandle::Image(image_view));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
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
            let sv: Retained<NSScrollView> = unsafe { NSScrollView::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                sv.setHasVerticalScroller(true);
                sv.setHasHorizontalScroller(false);
                sv.setAutohidesScrollers(true);
                sv.setDrawsBackground(false);
                // Enable bounds-change notifications so on_scroll can be wired
                // via NotificationCenter in a future delegate implementation.
                let clip = sv.contentView();
                clip.setPostsBoundsChangedNotifications(true);
            }
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
            let sv: Retained<NSScrollView> = unsafe { NSScrollView::initWithFrame(mtm.alloc(), frame) };
            unsafe {
                sv.setHasVerticalScroller(true);
                sv.setAutohidesScrollers(true);
                sv.setDrawsBackground(false);
            }
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
            let vev: Retained<NSVisualEffectView> = unsafe {
                let v = NSVisualEffectView::initWithFrame(mtm.alloc(), frame);
                v.setState(NSVisualEffectState::Active);
                v.setMaterial(NSVisualEffectMaterial::Sidebar);
                v
            };
            attach_to_parent(state, parent_id, &vev);
            state.nodes.insert(node_id, NativeHandle::SidebarWidget(unsafe { Retained::cast(vev) }));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // SidebarItem — Clickable row with native selection highlight (FR-026)
        // ------------------------------------------------------------------
        ComponentKind::SidebarItem => {
            let label_text = if let Some(PropValue::String(s)) = props.get(&PropKey::Text) {
                s.clone()
            } else {
                String::new()
            };
            let selected = bool_prop(&props, PropKey::Selected, false);

            // Use NSBox as container — supports fillColor + cornerRadius natively
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(210.0, 32.0));
            let row: Retained<NSBox> = unsafe {
                let b = NSBox::initWithFrame(mtm.alloc(), frame);
                b.setBoxType(NSBoxType::NSBoxCustom);
                b.setCornerRadius(6.0);
                b.setBorderWidth(0.0);
                if selected {
                    b.setFillColor(&NSColor::controlAccentColor());
                } else {
                    b.setFillColor(&NSColor::clearColor());
                }
                b
            };

            // Label — inset inside box
            let tf_frame = NSRect::new(NSPoint::new(10.0, 7.0), NSSize::new(190.0, 18.0));
            let tf: Retained<NSTextField> = unsafe {
                let t = NSTextField::initWithFrame(mtm.alloc(), tf_frame);
                t.setEditable(false);
                t.setBordered(false);
                t.setDrawsBackground(false);
                let ns = NSString::from_str(&label_text);
                t.setStringValue(&ns);
                let size: f64 = 13.0;
                let font = if selected {
                    NSFont::boldSystemFontOfSize(size)
                } else {
                    NSFont::systemFontOfSize(size)
                };
                t.setFont(Some(&font));
                if selected {
                    t.setTextColor(Some(&NSColor::whiteColor()));
                } else {
                    t.setTextColor(Some(&NSColor::labelColor()));
                }
                t.setLineBreakMode(NSLineBreakMode(4)); // NSLineBreakByTruncatingTail
                t
            };
            unsafe {
                if let Some(cv) = row.contentView() {
                    cv.addSubview(&tf);
                } else {
                    row.addSubview(&tf);
                }
            }

            // Invisible NSButton overlay for click
            if let Some(PropValue::HandlerRef(h)) = props.get(&PropKey::OnSelect) {
                let target = ButtonTarget::new(mtm, *h, node_id);
                let btn_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(210.0, 32.0));
                unsafe {
                    let btn: Retained<NSButton> = NSButton::initWithFrame(mtm.alloc(), btn_frame);
                    btn.setBezelStyle(NSBezelStyle::Automatic);
                    btn.setBordered(false);
                    btn.setTransparent(true);
                    btn.setTitle(&NSString::from_str(""));
                    let obj: &AnyObject = &target;
                    btn.setTarget(Some(obj));
                    btn.setAction(Some(sel!(fire:)));
                    if let Some(cv) = row.contentView() {
                        cv.addSubview(&btn);
                    } else {
                        row.addSubview(&btn);
                    }
                }
                stash_delegate(node_id, DelegateHandle::Button(target));
            }

            let row_as_view: Retained<NSView> = unsafe { Retained::cast(row) };
            attach_to_parent(state, parent_id, &row_as_view);
            state.nodes.insert(node_id, NativeHandle::View(
                row_as_view,
                ContainerInfo { axis: Axis::Horizontal, padding: 8.0, gap: 0.0 }
            ));
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
                gb.setCornerRadius(8.0);
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
        // SplitView — NSSplitView with resizable panes (FR-028)
        // ------------------------------------------------------------------
        ComponentKind::SplitView => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(600.0, 400.0));
            let sv: Retained<NSSplitView> = unsafe {
                let s = NSSplitView::initWithFrame(mtm.alloc(), frame);
                s.setVertical(true);
                s
            };
            attach_to_parent(state, parent_id, &sv);
            state.nodes.insert(node_id, NativeHandle::SplitViewWidget(sv));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Toolbar — NSView hosting toolbar-style buttons (FR-029)
        // ------------------------------------------------------------------
        ComponentKind::Toolbar => {
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 36.0));
            let container: Retained<NSView> = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
            if let Some(PropValue::ToolbarActions(actions)) = props.get(&PropKey::ToolbarActions) {
                let mut x: f64 = 8.0;
                for action in actions {
                    if action.separator {
                        x += 8.0;
                        continue;
                    }
                    let btn_frame = NSRect::new(NSPoint::new(x, 4.0), NSSize::new(28.0, 28.0));
                    let btn: Retained<NSButton> = unsafe {
                        NSButton::initWithFrame(mtm.alloc(), btn_frame)
                    };
                    if let Some(icon) = &action.icon_name {
                        let ns_name = NSString::from_str(icon);
                        if let Some(img) = unsafe {
                            NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns_name, None)
                        } {
                            unsafe { btn.setImage(Some(&img)) };
                        }
                    }
                    unsafe {
                        btn.setBezelStyle(NSBezelStyle::Toolbar);
                        btn.setBordered(false);
                        container.addSubview(&btn);
                    }
                    x += 36.0;
                }
            }
            attach_to_parent(state, parent_id, &container);
            state.nodes.insert(node_id, NativeHandle::ToolbarWidget(container));
            state.shadow.insert(node_id, ShadowNode::new(kind, props, parent_id));
            register_child(state, parent_id, node_id);
        }

        // ------------------------------------------------------------------
        // Switch — NSSwitch (macOS 10.15+) (FR-013)
        // ------------------------------------------------------------------
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

fn update_node(
    state: &mut RendererState,
    node_id: NodeId,
    changed: Props,
) -> Result<(), RenderError> {
    let Some(handle) = state.nodes.get(&node_id) else {
        return Err(RenderError::InvalidNode(node_id));
    };
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
            (NativeHandle::SidebarWidget(_), PropKey::Selected) => {} // selection styling handled via SidebarItem
            // SidebarItem selection: update NSBox fill color
            // The row is stored as NativeHandle::View (NSBox cast to NSView).
            // We update via the shadow prop — the reconciler will re-render with
            // the new Selected value, rebuilding the row with correct colors.
            (NativeHandle::View(_, _), PropKey::Selected) => {
                // No direct NSBox cast available here — rely on full re-render.
                // The view will be rebuilt with the correct fillColor on the next
                // reconciler cycle (the shadow prop update below triggers it).
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
    match state.nodes.get(&pid) {
        Some(NativeHandle::Window(w)) => {
            if let Some(content) = w.contentView() {
                unsafe { content.addSubview(view) };
            }
        }
        Some(NativeHandle::View(parent, _)) => unsafe { parent.addSubview(view) },
        // NSSplitView requires addArrangedSubview to participate in split layout
        Some(NativeHandle::SplitViewWidget(sv)) => unsafe { sv.addArrangedSubview(view) },
        // Generic NSView-backed Phase-2 containers — plain addSubview
        Some(NativeHandle::SidebarWidget(parent)) => unsafe { parent.addSubview(view) },
        // NSBox containers: children go into contentView so they appear inside the box
        Some(NativeHandle::GroupedSectionWidget(parent)) => unsafe {
            if let Some(cv) = parent.contentView() { cv.addSubview(view) }
            else { parent.addSubview(view) }
        },
        Some(NativeHandle::CardWidget(parent)) => unsafe {
            if let Some(cv) = parent.contentView() { cv.addSubview(view) }
            else { parent.addSubview(view) }
        },
        Some(NativeHandle::SafeAreaWidget(parent)) => unsafe { parent.addSubview(view) },
        Some(NativeHandle::TouchableWidget(parent)) => unsafe { parent.addSubview(view) },
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn relayout_window(state: &RendererState, window_id: NodeId) {
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
    let result = compute_layout(&spec, content_rect);
    log::debug!("relayout_window: {} rects computed", result.rects.len());
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
        | ComponentKind::Popover => LayoutAxis::Vertical,
        _ => LayoutAxis::None,
    };
    let padding = f32_prop(&shadow.props, PropKey::Padding, 0.0);
    let gap = f32_prop(&shadow.props, PropKey::Gap, 0.0);
    let min_height = f32_opt(&shadow.props, PropKey::MinHeight);
    let (fixed_width, fixed_height) = match shadow.kind {
        ComponentKind::Label => (None, Some(20.0)),
        ComponentKind::Button => (Some(120.0), Some(32.0)),
        ComponentKind::TextInput => (Some(200.0), Some(24.0)),
        ComponentKind::Checkbox => (None, Some(22.0)),
        ComponentKind::Slider => (Some(200.0), Some(24.0)),
        ComponentKind::ProgressBar => (Some(200.0), Some(20.0)),
        ComponentKind::Separator => (None, Some(8.0)),
        ComponentKind::TextArea => (None, Some(100.0)),
        ComponentKind::Dropdown => (Some(180.0), Some(26.0)),
        ComponentKind::Image => (Some(32.0), Some(32.0)),
        ComponentKind::Stepper => (Some(120.0), Some(26.0)),
        ComponentKind::ColorWell => (Some(46.0), Some(26.0)),
        ComponentKind::DatePicker => (Some(180.0), Some(26.0)),
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
        ComponentKind::SearchBar => (None, Some(28.0)),
        ComponentKind::StatusBar => (None, Some(22.0)),
        ComponentKind::SidebarItem => (None, Some(28.0)),
        ComponentKind::RichText => (None, Some(80.0)),
        ComponentKind::FlatList => (None, Some(300.0)),
        ComponentKind::ScrollView => (None, Some(200.0)),
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
    // Labels with no explicit width use a minimum of 60px so they don't
    // collapse to zero in hstack layouts where other fixed-width siblings
    // compete for space.
    let min_width = match shadow.kind {
        ComponentKind::Label | ComponentKind::Checkbox => {
            Some(f32_opt(&shadow.props, PropKey::MinWidth).unwrap_or(60.0))
        }
        _ => f32_opt(&shadow.props, PropKey::MinWidth),
    };
    let children = shadow
        .children
        .iter()
        .filter_map(|c| build_spec(state, *c))
        .collect();

    // flex_grow: fill available space when no fixed width/height is set.
    // Used for containers that should expand to fill their parent (vstack
    // content areas, GroupedSection, SearchBar, etc.)
    let flex_grow = match shadow.kind {
        // Full-fill containers: always expand
        ComponentKind::LayoutContainer(_)
        | ComponentKind::SafeAreaView
        | ComponentKind::TouchableOpacity => 1.0,
        // Fill-width containers (no fixed_width): expand horizontally
        ComponentKind::GroupedSection
        | ComponentKind::Card
        | ComponentKind::SearchBar
        | ComponentKind::TextArea
        | ComponentKind::ScrollView
        | ComponentKind::FlatList
        | ComponentKind::RichText
        | ComponentKind::Toolbar
        | ComponentKind::Skeleton
        | ComponentKind::Separator
            if fixed_width.is_none() => 1.0,
        _ => 0.0,
    };
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
        children,
    })
}

fn apply_rects(state: &RendererState, content_view: &NSView, result: &rn_layout::LayoutResult) {
    let content_height = content_view.frame().size.height as f32;
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
                    (lx, ly, parent_rect.height)
                }
                None => {
                    // No parent rect (direct content-view child or window child).
                    (rect.x, rect.y, content_height)
                }
            };

        // Skip direct children of NSSplitView — NSSplitView manages pane frames internally
        // via addArrangedSubview. Applying our frames would collapse them to zero.
        let is_splitview_child = shadow_parent_id
            .and_then(|pid| state.nodes.get(&pid))
            .map(|h| matches!(h, NativeHandle::SplitViewWidget(_)))
            .unwrap_or(false);
        if is_splitview_child {
            continue;
        }

        // Flip y from taffy (top-left origin) to AppKit (bottom-left origin).
        let flipped_y = superview_height - local_y - rect.height;
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

        // After sizing a NSSplitView, distribute panes equally by setting divider positions.
        if let Some(NativeHandle::SplitViewWidget(sv)) = state.nodes.get(&node_id) {
            let pane_count = state.shadow.get(&node_id)
                .map(|s| s.children.len())
                .unwrap_or(0);
            if pane_count > 1 {
                let step = rect.width / pane_count as f32;
                for i in 0..(pane_count - 1) {
                    let pos = step * (i as f32 + 1.0);
                    unsafe { sv.setPosition_ofDividerAtIndex(pos as f64, i as isize) };
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Prop helpers
// ---------------------------------------------------------------------------

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
            let size: f64 = span.font_size.unwrap_or(13.0) as f64;
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
        Some(PropValue::F32(v)) => Some(*v),
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

#[derive(Debug)]
struct TargetIvars {
    handler: HandlerId,
    source: NodeId,
    kind: EventKind,
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
        let this = mtm.alloc().set_ivars(TargetIvars {
            handler,
            source,
            kind: EventKind::Press,
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
