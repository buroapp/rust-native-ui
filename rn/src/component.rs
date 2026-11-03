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
