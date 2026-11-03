//! Rust Native (`rn`) — declarative cross-platform native UI framework.
//!
//! Consumers describe UI as a pure `ComponentTree`. The framework reconciles
//! consecutive trees into a minimal `Changeset` and dispatches to a per-platform
//! renderer that drives native OS controls.
//!
//! See `specs/001-core-framework/` in the repository for the full specification,
//! plan, and public-API contract.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![deny(unreachable_pub)]

pub mod component;
pub mod error;
pub mod event;
pub mod flat_list;
pub mod reconciler;
pub mod renderer;
pub mod thread;

use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub use component::{Axis, ColorValue, Component, ComponentBuilder, ComponentKind, ComponentTree, PropKey, PropValue, Props};
pub use error::{ReconcileError, RenderError, RnError};
pub use event::{Event, EventKind, EventPayload, HandlerId, KeyCode};
pub use reconciler::{Changeset, Operation, Reconciler};
pub use renderer::PlatformRenderer;
pub use rn_core::{
    ColorToken, FontToken, Insets, Point, RadiusToken, ScrollEvent,
    SpacingToken, StyledSpan, ToolbarAction, NodeId,
};

/// Platform backend bindings. Enabled by the `macos`, `windows`, or `linux`
/// feature flag. Only the renderer type is re-exported; no `objc2`/`windows`/
/// `gtk4` symbols leak through this module (US3 / FR-007).
///
/// This module is internal and subject to change without a semver bump.
/// Consumers should use `rn::render` and `rn::run` directly and let the
/// feature flag select the backend; only import from this module when you
/// need to name the renderer type explicitly (e.g. to call `::new()`).
#[doc(hidden)]
pub mod backend {
    #[cfg(feature = "macos")]
    pub use rn_macos::MacosRenderer;
    #[cfg(feature = "windows")]
    pub use rn_windows::WindowsRenderer;
    #[cfg(feature = "linux")]
    pub use rn_linux::LinuxRenderer;
}

// ---------------------------------------------------------------------------
// Builder entry points (contracts/public-api.md)
// ---------------------------------------------------------------------------

/// Start building a `Window` component.
pub fn window() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Window)
}

/// Build a `Label` component with the given text.
pub fn label(text: impl Into<String>) -> Component {
    ComponentBuilder::new(ComponentKind::Label)
        .prop(PropKey::Text, PropValue::String(text.into()))
        .build()
}

/// Build a `Label` with an explicit minimum width (useful for aligned form rows).
pub fn label_w(text: impl Into<String>, min_width: f32) -> Component {
    ComponentBuilder::new(ComponentKind::Label)
        .prop(PropKey::Text, PropValue::String(text.into()))
        .prop(PropKey::MinWidth, PropValue::F32(min_width))
        .build()
}

/// Start building a `Button` component with the given label text.
pub fn button(label: impl Into<String>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Button).prop(PropKey::Text, PropValue::String(label.into()))
}

/// Start building a `TextInput` component.
pub fn text_input() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::TextInput)
}

/// Start building a vertical layout container.
pub fn vstack() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::LayoutContainer(Axis::Vertical))
}

/// Start building a horizontal layout container.
pub fn hstack() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::LayoutContainer(Axis::Horizontal))
}

/// Wrap a component with an explicit reconciler key (for dynamic lists).
pub fn keyed(key: impl Into<String>, mut component: Component) -> Component {
    component.key = Some(key.into());
    component
}

/// Build a `Checkbox` component.
pub fn checkbox(label: impl Into<String>, checked: bool) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Checkbox)
        .prop(PropKey::Text, PropValue::String(label.into()))
        .prop(PropKey::Checked, PropValue::Bool(checked))
}

/// Build a `Slider` component.
pub fn slider(value: f32, min: f32, max: f32) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Slider)
        .prop(PropKey::Value, PropValue::F32(value))
        .prop(PropKey::Min, PropValue::F32(min))
        .prop(PropKey::Max, PropValue::F32(max))
}

/// Build a `ProgressBar` component (determinate, 0.0–100.0).
pub fn progress_bar(value: f32) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::ProgressBar)
        .prop(PropKey::Value, PropValue::F32(value.clamp(0.0, 100.0)))
        .prop(PropKey::Min, PropValue::F32(0.0))
        .prop(PropKey::Max, PropValue::F32(100.0))
}

/// Build a horizontal `Separator` line.
pub fn separator() -> Component {
    ComponentBuilder::new(ComponentKind::Separator).build()
}

/// Build a `TextArea` (multi-line text input).
pub fn text_area(text: impl Into<String>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::TextArea)
        .prop(PropKey::Text, PropValue::String(text.into()))
}

/// Build a `Dropdown` (pop-up button).
pub fn dropdown(options: Vec<String>, selected: i32) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Dropdown)
        .prop(PropKey::Options, PropValue::StringList(options))
        .prop(PropKey::SelectedIndex, PropValue::F32(selected as f32))
}

/// Build an `Image` from an SF Symbol name (e.g. `"star.fill"`).
pub fn image(name: impl Into<String>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Image)
        .prop(PropKey::ImageName, PropValue::String(name.into()))
}

/// Build a `Stepper` with a linked value label.
pub fn stepper(value: f32, min: f32, max: f32, step: f32) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Stepper)
        .prop(PropKey::Value, PropValue::F32(value))
        .prop(PropKey::Min, PropValue::F32(min))
        .prop(PropKey::Max, PropValue::F32(max))
        .prop(PropKey::Step, PropValue::F32(step))
}

/// Build a `ColorWell`.
pub fn color_well(color: ColorValue) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::ColorWell)
        .prop(PropKey::Color, PropValue::Color(color))
}

/// Build a `DatePicker` (ISO 8601 date string).
pub fn date_picker(date: impl Into<String>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::DatePicker)
        .prop(PropKey::DateValue, PropValue::String(date.into()))
}

/// Build a `RadioGroup`.
pub fn radio_group(options: Vec<String>, selected: i32) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::RadioGroup)
        .prop(PropKey::Options, PropValue::StringList(options))
        .prop(PropKey::SelectedIndex, PropValue::F32(selected as f32))
}

// ---------------------------------------------------------------------------
// Handler registry (FR-013 contract)
// ---------------------------------------------------------------------------

type HandlerFn = Box<dyn Fn(Event) + Send + Sync + 'static>;

fn handlers() -> &'static Mutex<HashMap<HandlerId, HandlerFn>> {
    static HANDLERS: OnceLock<Mutex<HashMap<HandlerId, HandlerFn>>> = OnceLock::new();
    HANDLERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a callback for a `HandlerId`. Replaces any previously registered
/// handler for the same id. Thread-safe.
pub fn on(id: HandlerId, handler: impl Fn(Event) + Send + Sync + 'static) {
    let Ok(mut map) = handlers().lock() else {
        log::error!("rn::on: handler registry mutex poisoned; dropping registration");
        return;
    };
    map.insert(id, Box::new(handler));
}

/// Dispatch an event to its registered handler, panic-safe (FR-013).
///
/// A panic inside the consumer handler is caught, logged at `error` level, and
/// swallowed — the event loop continues. Framework state is never mutated as
/// a result of a handler panic.
///
/// Returns `true` if a handler was registered and ran (whether or not it
/// panicked); `false` if no handler was registered for the event's `handler`
/// id.
pub fn dispatch_handler(event: Event) -> bool {
    let Ok(map) = handlers().lock() else {
        log::error!("rn::dispatch_handler: handler registry mutex poisoned");
        return false;
    };
    let Some(handler) = map.get(&event.handler) else {
        log::trace!(
            "rn::dispatch_handler: no handler for {:?} on {:?}",
            event.kind,
            event.handler
        );
        return false;
    };
    let result = catch_unwind(AssertUnwindSafe(|| handler(event.clone())));
    if let Err(payload) = result {
        log::error!("handler {:?} panicked: {}", event.handler, panic_payload_str(&payload));
    }
    true
}

fn panic_payload_str(payload: &Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

// ---------------------------------------------------------------------------
// Render entry points (FR-011, FR-012, FR-014)
// ---------------------------------------------------------------------------

fn reconciler_cell() -> &'static Mutex<Reconciler> {
    static RECONCILER: OnceLock<Mutex<Reconciler>> = OnceLock::new();
    RECONCILER.get_or_init(|| Mutex::new(Reconciler::new()))
}

/// Reconcile `tree` against the previously rendered tree and apply the resulting
/// `Changeset` to `renderer`.
///
/// Must be called from the platform main thread (FR-011). Non-main callers get
/// `Err(RnError::WrongThread)` — no panic (FR-012).
pub fn render<R: PlatformRenderer>(renderer: &mut R, tree: ComponentTree) -> Result<(), RnError> {
    thread::MainThreadGuard::check()?;

    let start = Instant::now();
    let changeset = {
        let Ok(mut rec) = reconciler_cell().lock() else {
            return Err(RnError::Render(RenderError::OsError(
                "reconciler mutex poisoned".to_string(),
            )));
        };
        rec.reconcile(tree).map_err(RnError::Reconcile)?
    };
    let reconcile_time = start.elapsed();

    log::trace!(
        "rn::render: reconcile took {:?}, {} ops",
        reconcile_time,
        changeset.operations.len()
    );
    log::debug!("rn::render: {}", changeset.summary());

    renderer.apply(changeset).map_err(RnError::Render)?;
    Ok(())
}

/// Re-reconcile `tree` and apply via the installed backend bridge. Used from
/// inside a running event loop (e.g. the `set_tick` closure in the counter
/// example) where the host no longer has `&mut renderer` in scope. The bridge
/// is installed by `rn::run` when the backend starts.
pub fn rerender(tree: ComponentTree) -> Result<(), RnError> {
    thread::MainThreadGuard::check()?;
    let start = Instant::now();
    let changeset = {
        let Ok(mut rec) = reconciler_cell().lock() else {
            return Err(RnError::Render(RenderError::OsError(
                "reconciler mutex poisoned".to_string(),
            )));
        };
        rec.reconcile(tree).map_err(RnError::Reconcile)?
    };
    log::trace!(
        "rn::rerender: reconcile took {:?}, {} ops",
        start.elapsed(),
        changeset.operations.len()
    );
    log::debug!("rn::rerender: {}", changeset.summary());
    rn_core::apply_via_bridge(changeset).map_err(RnError::Render)?;
    Ok(())
}

/// Start the platform event loop. Must be called from the main thread.
pub fn run<R: PlatformRenderer>(renderer: &mut R) -> Result<(), RnError> {
    thread::MainThreadGuard::check()?;
    // Hand the framework's event dispatcher to backends via rn-core so they
    // can route native events without depending on `rn` (FR-013 bridge).
    rn_core::set_dispatch_bridge(dispatch_handler_trampoline);
    renderer.run_event_loop().map_err(RnError::Render)?;
    Ok(())
}

fn dispatch_handler_trampoline(event: Event) {
    let _ = dispatch_handler(event);
}

// ---------------------------------------------------------------------------
// Tick hook — host apps install a closure here; the backend calls
// `rn_core::tick()` from its event-loop timer, which routes to this closure.
// Used by examples to drain `update_sender` messages each tick (FR-011,
// coalescing per analyze G1).
// ---------------------------------------------------------------------------

type TickClosure = Box<dyn Fn() + Send + Sync + 'static>;

fn tick_closure() -> &'static Mutex<Option<TickClosure>> {
    static TICK: OnceLock<Mutex<Option<TickClosure>>> = OnceLock::new();
    TICK.get_or_init(|| Mutex::new(None))
}

/// Install a per-tick closure invoked once per event-loop tick on the main
/// thread. Used by examples to drain pending `update_sender` messages and
/// trigger a re-render.
pub fn set_tick(f: impl Fn() + Send + Sync + 'static) {
    if let Ok(mut slot) = tick_closure().lock() {
        *slot = Some(Box::new(f));
    }
    rn_core::set_tick_bridge(tick_trampoline);
}

fn tick_trampoline() {
    let Ok(slot) = tick_closure().lock() else {
        return;
    };
    if let Some(f) = slot.as_ref() {
        let f = f.as_ref();
        let _ = catch_unwind(AssertUnwindSafe(f));
    }
}

// ---------------------------------------------------------------------------
// Cross-thread update channel (FR-011)
// ---------------------------------------------------------------------------

/// A type-erased update message. Consumers wrap their state-update values via
/// `UpdateMessage::new(value)` and recover them via `downcast` on the receiving
/// side.
pub struct UpdateMessage(Box<dyn Any + Send>);

impl UpdateMessage {
    /// Wrap a value for cross-thread posting.
    pub fn new<T: Send + 'static>(value: T) -> Self {
        UpdateMessage(Box::new(value))
    }

    /// Attempt to recover the inner value.
    pub fn downcast<T: Send + 'static>(self) -> Result<T, Self> {
        match self.0.downcast::<T>() {
            Ok(boxed) => Ok(*boxed),
            Err(any) => Err(UpdateMessage(any)),
        }
    }
}

type UpdateChannel = (
    crossbeam_channel::Sender<UpdateMessage>,
    crossbeam_channel::Receiver<UpdateMessage>,
);

fn update_channel() -> &'static UpdateChannel {
    static CHANNEL: OnceLock<UpdateChannel> = OnceLock::new();
    CHANNEL.get_or_init(crossbeam_channel::unbounded)
}

/// Returns a sender keyed to the main-thread message queue. Background threads
/// clone this and post `UpdateMessage::new(value)` to request a re-render.
pub fn update_sender() -> crossbeam_channel::Sender<UpdateMessage> {
    update_channel().0.clone()
}

/// Framework-internal: drain the update channel. Called by backends from the
/// main event loop each tick to collect pending state updates and coalesce
/// multiple posts into a single render (FR-011, analyze G1).
#[doc(hidden)]
pub fn drain_updates() -> Vec<UpdateMessage> {
    let (_, rx) = update_channel();
    rx.try_iter().collect()
}

// ---------------------------------------------------------------------------
// Phase 2 builder functions (002-react-native-parity)
// ---------------------------------------------------------------------------

/// Start building a `ScrollView` component.
pub fn scroll_view() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::ScrollView)
}

/// Start building a `FlatList` component.
///
/// - `render_item`: a `HandlerId` whose handler will be called with
///   `EventPayload::Index(i)` for each visible row; it must call
///   `rn::rerender(tree)` to update that row's component.
pub fn flat_list(render_item: HandlerId) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::FlatList)
        .prop(PropKey::RenderItem, PropValue::HandlerRef(render_item))
}

/// Start building a `TouchableOpacity` component.
pub fn touchable_opacity(on_press: HandlerId) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::TouchableOpacity)
        .prop(PropKey::OnPress, PropValue::HandlerRef(on_press))
        .prop(PropKey::PressOpacity, PropValue::F32(0.4))
}

/// Start building a `SafeAreaView` component.
pub fn safe_area_view() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::SafeAreaView)
}

/// Start building an `ActivityIndicator` component.
pub fn activity_indicator() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::ActivityIndicator)
        .prop(PropKey::Visible, PropValue::Bool(true))
}

/// Start building a `Modal` component.
pub fn modal(visible: bool) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Modal)
        .prop(PropKey::ModalVisible, PropValue::Bool(visible))
}

/// Start building a `Popover` component anchored to `anchor_id`.
pub fn popover(anchor_id: NodeId) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Popover)
        .prop(PropKey::AnchorId, PropValue::NodeRef(anchor_id))
}

/// Start building a `RichText` component from styled spans.
pub fn rich_text(spans: Vec<StyledSpan>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::RichText)
        .prop(PropKey::TextSpans, PropValue::Spans(spans))
        .prop(PropKey::TextWrap, PropValue::Bool(true))
}

/// Build a simple single-span `RichText` component.
pub fn text_plain(s: impl Into<String>) -> ComponentBuilder {
    rich_text(vec![StyledSpan::new(s)])
}

/// Start building a `Switch` component.
pub fn switch(value: bool, on_change: HandlerId) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Switch)
        .prop(PropKey::SwitchValue, PropValue::Bool(value))
        .prop(PropKey::OnChange, PropValue::HandlerRef(on_change))
}

/// Start building a `StatusBar` component.
pub fn status_bar() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::StatusBar)
        .prop(PropKey::Visible, PropValue::Bool(true))
}

/// Start building a `Card` component.
pub fn card() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Card)
}

/// Start building a `Badge` component.
pub fn badge(label: impl Into<String>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Badge)
        .prop(PropKey::BadgeLabel, PropValue::String(label.into()))
}

/// Start building a `Skeleton` placeholder.
pub fn skeleton(width: f32, height: f32) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Skeleton)
        .prop(PropKey::Width, PropValue::F32(width))
        .prop(PropKey::Height, PropValue::F32(height))
}

/// Start building a `SegmentedControl` component.
pub fn segmented_control(
    options: Vec<String>,
    selected_index: i32,
    on_change: HandlerId,
) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::SegmentedControl)
        .prop(PropKey::Options, PropValue::StringList(options))
        .prop(PropKey::SelectedIndex, PropValue::F32(selected_index as f32))
        .prop(PropKey::OnChange, PropValue::HandlerRef(on_change))
}

/// Start building a `SearchBar` component.
pub fn search_bar(placeholder: impl Into<String>, on_change: HandlerId) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::SearchBar)
        .prop(PropKey::Placeholder, PropValue::String(placeholder.into()))
        .prop(PropKey::OnChange, PropValue::HandlerRef(on_change))
}

/// Start building a `Sidebar` component.
pub fn sidebar() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Sidebar)
        .prop(PropKey::SidebarWidth, PropValue::F32(220.0))
        .prop(PropKey::SidebarMinWidth, PropValue::F32(160.0))
        .prop(PropKey::SidebarMaxWidth, PropValue::F32(320.0))
}

/// Start building a `SidebarItem` component.
pub fn sidebar_item(label: impl Into<String>, on_select: HandlerId) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::SidebarItem)
        .prop(PropKey::Text, PropValue::String(label.into()))
        .prop(PropKey::OnSelect, PropValue::HandlerRef(on_select))
        .prop(PropKey::Selected, PropValue::Bool(false))
}

/// Start building a `GroupedSection` component.
pub fn grouped_section() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::GroupedSection)
}

/// Start building a `SplitView` component.
pub fn split_view() -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::SplitView)
        .prop(PropKey::DividerWidth, PropValue::F32(1.0))
}

/// Start building a `Toolbar` component.
pub fn toolbar(actions: Vec<ToolbarAction>) -> ComponentBuilder {
    ComponentBuilder::new(ComponentKind::Toolbar)
        .prop(PropKey::ToolbarActions, PropValue::ToolbarActions(actions))
}
