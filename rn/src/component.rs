//! Component tree types — the declarative description language.
//!
//! Leaf types (`NodeId`, `ComponentKind`, `PropKey`, `PropValue`, `Props`,
//! `HandlerId`, `Axis`) are re-exported from `rn-core` so platform backends
//! can share them without depending on `rn`. The `Component` tree and
//! builder live here in `rn` (backends never see the virtual tree — only
//! the `Changeset` the reconciler produces).

pub use rn_core::{
    Axis, ColorValue, ComponentKind, HandlerId, NodeId, PropKey, PropValue, Props,
    ColorToken, FontToken, Insets, Point, RadiusToken, ScrollEvent,
    SpacingToken, StyledSpan, ToolbarAction,
};

use std::collections::HashMap;

/// A declarative description of one UI element and its subtree.
///
/// Constructing a `Component` performs no OS calls and allocates only the
/// `Vec<Component>` and `Props` map it owns.
#[derive(Debug, Clone)]
pub struct Component {
    pub kind: ComponentKind,
    pub props: Props,
    pub key: Option<String>,
    pub children: Vec<Component>,
}

impl Component {
    /// Set a prop on an already-built component.
    #[must_use]
    pub fn prop(mut self, key: PropKey, value: PropValue) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Centre this component's text.
    #[must_use]
    pub fn text_center(mut self) -> Self {
        self.props.insert(PropKey::TextAlign, PropValue::F32(2.0));
        self
    }

    /// Wrap this component's text onto at most `lines` lines.
    #[must_use]
    pub fn lines(mut self, lines: u8) -> Self {
        self.props
            .insert(PropKey::MaxLines, PropValue::F32(f32::from(lines)));
        self
    }

    pub fn new(kind: ComponentKind) -> Self {
        Self {
            kind,
            props: HashMap::new(),
            key: None,
            children: Vec::new(),
        }
    }
}

/// Fluent builder for `Component` values.
#[derive(Debug, Clone)]
pub struct ComponentBuilder {
    inner: Component,
}

impl ComponentBuilder {
    pub(crate) fn new(kind: ComponentKind) -> Self {
        Self {
            inner: Component::new(kind),
        }
    }

    pub fn prop(mut self, key: PropKey, value: PropValue) -> Self {
        self.inner.props.insert(key, value);
        self
    }

    pub fn title(self, text: impl Into<String>) -> Self {
        self.prop(PropKey::Title, PropValue::String(text.into()))
    }

    pub fn size(self, width: f32, height: f32) -> Self {
        self.prop(PropKey::Width, PropValue::F32(width))
            .prop(PropKey::Height, PropValue::F32(height))
    }

    /// Use the modern window look: content extends under a transparent titlebar
    /// (`FullSizeContentView`), which is what current macOS windows do.
    pub fn modern(self, modern: bool) -> Self {
        self.prop(PropKey::ModernLook, PropValue::Bool(modern))
    }

    /// Attach an SF Symbol name to this component (row icons, toolbar glyphs).
    pub fn icon(self, name: impl Into<String>) -> Self {
        self.prop(PropKey::IconName, PropValue::String(name.into()))
    }

    pub fn enabled(self, enabled: bool) -> Self {
        self.prop(PropKey::Enabled, PropValue::Bool(enabled))
    }

    pub fn placeholder(self, text: impl Into<String>) -> Self {
        self.prop(PropKey::Placeholder, PropValue::String(text.into()))
    }

    pub fn value(self, text: impl Into<String>) -> Self {
        self.prop(PropKey::Text, PropValue::String(text.into()))
    }

    pub fn gap(self, gap: f32) -> Self {
        self.prop(PropKey::Gap, PropValue::F32(gap))
    }

    pub fn padding(self, padding: f32) -> Self {
        self.prop(PropKey::Padding, PropValue::F32(padding))
    }

    /// Set `flex_grow` — how much this node expands to fill available space
    /// in its parent container. `1.0` = fill all remaining space.
    /// `0.0` (default) = shrink to content.
    pub fn flex(self, grow: f32) -> Self {
        self.prop(PropKey::FlexGrow, PropValue::F32(grow))
    }

    /// Push children apart along the main axis (React Native `justifyContent: 'space-between'`).
    pub fn justify_between(self) -> Self {
        self.prop(PropKey::Justify, PropValue::F32(3.0))
    }

    /// Center children along the main axis.
    pub fn justify_center(self) -> Self {
        self.prop(PropKey::Justify, PropValue::F32(2.0))
    }

    /// Align children to the end of the main axis.
    /// Let children wrap onto new lines when they do not fit.
    pub fn wrap(self) -> Self {
        self.prop(PropKey::Wrap, PropValue::F32(1.0))
    }

    /// Centre the text inside a label.
    pub fn text_center(self) -> Self {
        self.prop(PropKey::TextAlign, PropValue::F32(2.0))
    }

    /// Wrap text onto at most `lines` lines before truncating.
    pub fn lines(self, lines: u8) -> Self {
        self.prop(PropKey::MaxLines, PropValue::F32(f32::from(lines)))
    }

    /// Wrap text instead of truncating it on one line.
    pub fn text_wrap(self) -> Self {
        self.prop(PropKey::TextWrap, PropValue::Bool(true))
    }

    /// Centre children on the cross axis.
    pub fn align_center(self) -> Self {
        self.prop(PropKey::Align, PropValue::F32(2.0))
    }

    /// Align children to the start of the cross axis.
    pub fn align_start(self) -> Self {
        self.prop(PropKey::Align, PropValue::F32(1.0))
    }

    pub fn justify_end(self) -> Self {
        self.prop(PropKey::Justify, PropValue::F32(1.0))
    }

    pub fn on_press(self, id: HandlerId) -> Self {
        self.prop(PropKey::OnPress, PropValue::HandlerRef(id))
    }

    pub fn on_change(self, id: HandlerId) -> Self {
        self.prop(PropKey::OnChange, PropValue::HandlerRef(id))
    }

    pub fn on_close(self, id: HandlerId) -> Self {
        self.prop(PropKey::OnClose, PropValue::HandlerRef(id))
    }

    pub fn checked(self, checked: bool) -> Self {
        self.prop(PropKey::Checked, PropValue::Bool(checked))
    }

    pub fn min(self, v: f32) -> Self {
        self.prop(PropKey::Min, PropValue::F32(v))
    }

    pub fn max(self, v: f32) -> Self {
        self.prop(PropKey::Max, PropValue::F32(v))
    }

    pub fn step(self, v: f32) -> Self {
        self.prop(PropKey::Step, PropValue::F32(v))
    }

    pub fn selected_index(self, idx: i32) -> Self {
        self.prop(PropKey::SelectedIndex, PropValue::F32(idx as f32))
    }

    pub fn image_name(self, name: impl Into<String>) -> Self {
        self.prop(PropKey::ImageName, PropValue::String(name.into()))
    }

    pub fn color(self, c: ColorValue) -> Self {
        self.prop(PropKey::Color, PropValue::Color(c))
    }

    pub fn date_value(self, date: impl Into<String>) -> Self {
        self.prop(PropKey::DateValue, PropValue::String(date.into()))
    }

    pub fn child(mut self, child: impl Into<Component>) -> Self {
        self.inner.children.push(child.into());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Component>) -> Self {
        self.inner.children.extend(children);
        self
    }

    pub fn build(self) -> Component {
        self.inner
    }
}

impl From<ComponentBuilder> for Component {
    fn from(b: ComponentBuilder) -> Self {
        b.build()
    }
}

/// An owned, immutable snapshot of the UI description at a point in time.
#[derive(Debug, Clone)]
pub struct ComponentTree {
    pub root: Component,
    pub version: u64,
}

impl ComponentTree {
    pub fn new(root: impl Into<Component>) -> Self {
        Self {
            root: root.into(),
            version: 0,
        }
    }
}
