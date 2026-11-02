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
    NSControlTextEditingDelegate, NSTextField, NSTextFieldDelegate, NSView, NSWindow,
    NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString, NSTimer,
};

use rn_core::{
    ApplyBridge, Axis, Changeset, ComponentKind, Event, EventKind, EventPayload, HandlerId, NodeId,
    Operation, PlatformRenderer, PropKey, PropValue, Props, RenderError,
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
}

impl NativeHandle {
    fn as_view(&self) -> Option<&NSView> {
        match self {
            NativeHandle::Window(_) => None,
            NativeHandle::View(v, _) => Some(v),
            NativeHandle::Label(v) => Some(v),
            NativeHandle::Button(v) => Some(v),
            NativeHandle::TextInput(v) => Some(v),
        }
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
        _ => LayoutAxis::None,
    };
    let padding = f32_prop(&shadow.props, PropKey::Padding, 0.0);
    let gap = f32_prop(&shadow.props, PropKey::Gap, 0.0);
    let min_height = f32_opt(&shadow.props, PropKey::MinHeight);
    let (fixed_width, fixed_height) = match shadow.kind {
        ComponentKind::Label => (None, Some(20.0)),
        ComponentKind::Button => (Some(120.0), Some(32.0)),
        ComponentKind::TextInput => (Some(200.0), Some(24.0)),
        _ => (
            f32_opt(&shadow.props, PropKey::Width),
            f32_opt(&shadow.props, PropKey::Height),
        ),
    };
    // Labels with no explicit width use a minimum of 60px so they don't
    // collapse to zero in hstack layouts where other fixed-width siblings
    // compete for space.
    let min_width = match shadow.kind {
        ComponentKind::Label => Some(f32_opt(&shadow.props, PropKey::MinWidth).unwrap_or(60.0)),
        _ => f32_opt(&shadow.props, PropKey::MinWidth),
    };
    let children = shadow
        .children
        .iter()
        .filter_map(|c| build_spec(state, *c))
        .collect();
    Some(LayoutSpec {
        id: LayoutNodeId(node_id.0),
        axis,
        padding,
        gap,
        min_width,
        min_height,
        fixed_width,
        fixed_height,
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

        // `rect` is absolute (relative to the layout root = content view).
        // AppKit NSView frames are relative to the superview's coordinate space.
        // For views whose superview IS the content view (LayoutContainers directly
        // inside the window), the rect is already in the right space.
        // For leaf views (Label/Button/TextInput) whose superview is a nested
        // LayoutContainer, we must subtract the parent's absolute origin.
        let parent_origin = state
            .shadow
            .get(&node_id)
            .and_then(|sn| sn.parent)
            .and_then(|pid| result.rects.get(&rn_layout::LayoutNodeId(pid.0)))
            .map(|pr| (pr.x, pr.y))
            .unwrap_or((0.0, 0.0));

        // Rect relative to immediate superview.
        let local_x = rect.x - parent_origin.0;
        let local_y = rect.y - parent_origin.1;

        // Flip y from taffy (top-left) to AppKit (bottom-left).
        // The superview's height is either the content view's or the parent rect's.
        let superview_height = if parent_origin.0 == 0.0 && parent_origin.1 == 0.0 {
            content_height
        } else {
            state
                .shadow
                .get(&node_id)
                .and_then(|sn| sn.parent)
                .and_then(|pid| result.rects.get(&rn_layout::LayoutNodeId(pid.0)))
                .map(|pr| pr.height)
                .unwrap_or(content_height)
        };

        let flipped_y = superview_height - local_y - rect.height;
        let ns = NSRect::new(
            NSPoint::new(local_x as f64, flipped_y as f64),
            NSSize::new(rect.width as f64, rect.height as f64),
        );
        log::trace!(
            "setFrame {:?} → x={:.0} y={:.0} w={:.0} h={:.0} (abs:{:.0},{:.0} parent:{:.0},{:.0})",
            node_id, local_x, flipped_y, rect.width, rect.height,
            rect.x, rect.y, parent_origin.0, parent_origin.1
        );
        unsafe { view.setFrame(ns) };
    }
}

// ---------------------------------------------------------------------------
// Prop helpers
// ---------------------------------------------------------------------------

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
            let event = Event {
                source: ivars.source,
                handler: ivars.handler,
                kind: ivars.kind,
                payload: EventPayload::None,
            };
            with_state(|s| s.pending_events.push(event.clone()));
            rn_core::dispatch_event(event);
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
