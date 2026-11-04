//! taffy-backed layout engine. Computes absolute pixel rects for every node
//! in a `ComponentTree` so platform renderers can position native controls
//! consistently across OSes.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use taffy::TraversePartialTree;

// Public surface is intentionally thin: a single `compute_layout` entry point
// returning a `LayoutResult`. Internal `taffy` types never leak.

/// Rectangular area in logical pixels, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Opaque identifier matching the framework's `NodeId`. We duplicate the type
/// here (as `u64`) to avoid a dependency on the `rn` crate from the layout
/// crate — layout is upstream of the public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutNodeId(pub u64);

/// A fully computed layout keyed by `LayoutNodeId`.
#[derive(Debug, Clone, Default)]
pub struct LayoutResult {
    pub rects: HashMap<LayoutNodeId, Rect>,
}

impl LayoutResult {
    pub fn get(&self, id: LayoutNodeId) -> Option<Rect> {
        self.rects.get(&id).copied()
    }
}

/// A platform-agnostic layout spec: enough information for `taffy` to compute
/// a layout without the renderer having to understand widget semantics.
#[derive(Debug, Clone)]
pub struct LayoutSpec {
    pub id: LayoutNodeId,
    pub axis: LayoutAxis,
    pub padding: f32,
    pub gap: f32,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub fixed_width: Option<f32>,
    pub fixed_height: Option<f32>,
    /// `flex_grow`: 1.0 means fill remaining space; 0.0 (default) means fixed/shrink.
    pub flex_grow: f32,
    /// Main-axis alignment: 0=start, 1=end, 2=center, 3=space-between,
    /// 4=space-around, 5=space-evenly.
    pub justify: u8,
    /// Cross-axis alignment: 0=stretch (default), 1=start, 2=center, 3=end.
    pub align: u8,
    /// Wrap children onto new lines when they overflow: 0=no (default), 1=wrap.
    pub wrap: u8,
    pub children: Vec<LayoutSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAxis {
    Horizontal,
    Vertical,
    None,
}

/// Compute a layout for a `LayoutSpec` tree in a container of the given size.
pub fn compute_layout(spec: &LayoutSpec, container: Rect) -> LayoutResult {
    use taffy::prelude::*;

    let mut taffy: TaffyTree<LayoutNodeId> = TaffyTree::new();
    let mut mapping: HashMap<NodeId, LayoutNodeId> = HashMap::new();

    fn build(
        taffy: &mut TaffyTree<LayoutNodeId>,
        mapping: &mut HashMap<NodeId, LayoutNodeId>,
        spec: &LayoutSpec,
    ) -> Option<NodeId> {
        let flex_direction = match spec.axis {
            LayoutAxis::Horizontal => FlexDirection::Row,
            LayoutAxis::Vertical => FlexDirection::Column,
            LayoutAxis::None => FlexDirection::Column,
        };

        let size = Size {
            width: match spec.fixed_width {
                Some(w) => length(w),
                None => auto(),
            },
            height: match spec.fixed_height {
                Some(h) => length(h),
                None => auto(),
            },
        };

        let min_size = Size {
            width: match spec.min_width {
                Some(w) => length(w),
                None => auto(),
            },
            height: match spec.min_height {
                Some(h) => length(h),
                None => auto(),
            },
        };

        let style = Style {
            display: Display::Flex,
            flex_direction,
            flex_grow: spec.flex_grow,
            justify_content: Some(match spec.justify {
                1 => JustifyContent::End,
                2 => JustifyContent::Center,
                3 => JustifyContent::SpaceBetween,
                4 => JustifyContent::SpaceAround,
                5 => JustifyContent::SpaceEvenly,
                _ => JustifyContent::Start,
            }),
            flex_wrap: if spec.wrap == 1 {
                FlexWrap::Wrap
            } else {
                FlexWrap::NoWrap
            },
            align_items: Some(match spec.align {
                1 => AlignItems::Start,
                2 => AlignItems::Center,
                3 => AlignItems::End,
                _ => AlignItems::Stretch,
            }),
            padding: Rect {
                left: length(spec.padding),
                right: length(spec.padding),
                top: length(spec.padding),
                bottom: length(spec.padding),
            },
            gap: Size {
                width: length(spec.gap),
                height: length(spec.gap),
            },
            size,
            min_size,
            ..Default::default()
        };

        let child_nodes: Vec<NodeId> = spec
            .children
            .iter()
            .filter_map(|c| build(taffy, mapping, c))
            .collect();

        let node = taffy.new_with_children(style, &child_nodes).ok()?;
        taffy.set_node_context(node, Some(spec.id)).ok()?;
        mapping.insert(node, spec.id);
        Some(node)
    }

    let Some(root) = build(&mut taffy, &mut mapping, spec) else {
        return LayoutResult::default();
    };

    // Force root to exactly fill the container — overrides any `auto` size.
    // Without this, taffy shrinks the root to its content size.
    if let Ok(mut style) = taffy.style(root).cloned() {
        style.size = Size {
            width: length(container.width),
            height: length(container.height),
        };
        let _ = taffy.set_style(root, style);
    }

    let root_size = Size {
        width: AvailableSpace::Definite(container.width),
        height: AvailableSpace::Definite(container.height),
    };

    if taffy.compute_layout(root, root_size).is_err() {
        return LayoutResult::default();
    }

    let mut rects = HashMap::new();
    collect_rects(&taffy, root, container.x, container.y, &mapping, &mut rects);
    LayoutResult { rects }
}

fn collect_rects(
    taffy: &taffy::TaffyTree<LayoutNodeId>,
    node: taffy::NodeId,
    origin_x: f32,
    origin_y: f32,
    mapping: &HashMap<taffy::NodeId, LayoutNodeId>,
    out: &mut HashMap<LayoutNodeId, Rect>,
) {
    let Ok(layout) = taffy.layout(node) else {
        return;
    };
    let x = origin_x + layout.location.x;
    let y = origin_y + layout.location.y;
    if let Some(id) = mapping.get(&node).copied() {
        out.insert(
            id,
            Rect {
                x,
                y,
                width: layout.size.width,
                height: layout.size.height,
            },
        );
    }
    let child_count = taffy.child_count(node);
    for i in 0..child_count {
        if let Ok(child) = taffy.child_at_index(node, i) {
            collect_rects(taffy, child, x, y, mapping, out);
        }
    }
}
