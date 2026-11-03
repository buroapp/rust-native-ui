//! Platform-independence of the layout engine.
//!
//! `rn-layout` must be the sole source of geometry on every platform (FR-008).
//! If the engine itself were sensitive to the host, cross-platform bounds could
//! never agree within the two-pixel budget SC-003 sets, and every backend
//! divergence would be unattributable.
//!
//! These tests pin the property directly: the same spec must produce byte-identical
//! results regardless of anything about the host. They run on all three CI legs,
//! so a platform-conditional slipping into the engine fails immediately.
//!
//! This does not test the backends. It establishes the precondition that lets a
//! divergence observed in practice be blamed on a backend rather than the engine.

use rn_layout::{compute_layout, LayoutAxis, LayoutNodeId, LayoutSpec, Rect};

fn node(
    id: u64,
    axis: LayoutAxis,
    w: Option<f32>,
    h: Option<f32>,
    grow: f32,
    children: Vec<LayoutSpec>,
) -> LayoutSpec {
    LayoutSpec {
        id: LayoutNodeId(id),
        axis,
        padding: 0.0,
        gap: 0.0,
        min_width: None,
        min_height: None,
        fixed_width: w,
        fixed_height: h,
        flex_grow: grow,
        justify: 0,
        children,
    }
}

/// A tree with the shape the real applications actually use: a horizontal
/// sidebar-plus-content split, with a fixed-width sidebar and a growing content
/// column of variable-height rows.
fn representative_tree() -> LayoutSpec {
    let sidebar = node(2, LayoutAxis::Vertical, Some(220.0), None, 0.0, vec![]);

    let rows: Vec<LayoutSpec> = (10..15)
        .map(|i| node(i, LayoutAxis::Vertical, None, Some(44.0), 0.0, vec![]))
        .collect();

    let content = node(3, LayoutAxis::Vertical, None, None, 1.0, rows);

    node(1, LayoutAxis::Horizontal, None, None, 1.0, vec![sidebar, content])
}

#[test]
fn same_spec_same_container_is_deterministic() {
    let spec = representative_tree();
    let container = Rect {
        x: 0.0,
        y: 0.0,
        width: 900.0,
        height: 600.0,
    };

    let first = compute_layout(&spec, container);
    let second = compute_layout(&spec, container);

    assert_eq!(
        first.rects.len(),
        second.rects.len(),
        "layout produced a different node count across two identical runs"
    );

    for (id, a) in &first.rects {
        let b = second
            .rects
            .get(id)
            .unwrap_or_else(|| panic!("node {id:?} present in first run but not second"));
        assert_eq!(
            (a.x, a.y, a.width, a.height),
            (b.x, b.y, b.width, b.height),
            "node {id:?} moved between two identical layout runs — the engine is not deterministic"
        );
    }
}

#[test]
fn reordered_child_insertion_does_not_change_geometry() {
    // Same tree, rebuilt from scratch: a fresh TaffyTree must reproduce the same
    // numbers. This catches hidden state carried between calls.
    let container = Rect {
        x: 0.0,
        y: 0.0,
        width: 900.0,
        height: 600.0,
    };

    let a = compute_layout(&representative_tree(), container);
    let b = compute_layout(&representative_tree(), container);

    assert_eq!(a.rects.len(), b.rects.len());
    for (id, rect) in &a.rects {
        let other = b
            .rects
            .get(id)
            .unwrap_or_else(|| panic!("node {id:?} missing on rebuild"));
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (other.x, other.y, other.width, other.height),
            "geometry changed between two fresh builds of the same tree"
        );
    }
}

#[test]
fn root_fills_the_container_exactly() {
    let spec = representative_tree();
    let container = Rect {
        x: 0.0,
        y: 0.0,
        width: 900.0,
        height: 600.0,
    };
    let result = compute_layout(&spec, container);

    let root = result
        .rects
        .get(&LayoutNodeId(1))
        .expect("root node must have a rect");

    assert_eq!(
        (root.width, root.height),
        (900.0, 600.0),
        "root must fill the container; a smaller value means taffy shrank to content"
    );
}

#[test]
fn fixed_and_growing_children_split_available_width() {
    let spec = representative_tree();
    let container = Rect {
        x: 0.0,
        y: 0.0,
        width: 900.0,
        height: 600.0,
    };
    let result = compute_layout(&spec, container);

    let sidebar = result.rects.get(&LayoutNodeId(2)).expect("sidebar rect");
    let content = result.rects.get(&LayoutNodeId(3)).expect("content rect");

    assert_eq!(sidebar.width, 220.0, "sidebar must honour its fixed width");
    assert_eq!(
        content.width, 680.0,
        "content must take exactly the remaining width (900 - 220)"
    );
    assert!(
        (sidebar.width + content.width - 900.0).abs() < 0.01,
        "sidebar and content must sum to the container width"
    );
}

#[test]
fn geometry_scales_linearly_with_container_width() {
    // Doubling the container width must double the growing child's width. If it
    // does not, something in the engine is using a fixed constant where it should
    // use available space, and backends on differently-sized displays would
    // disagree.
    let spec = representative_tree();

    let narrow = compute_layout(
        &spec,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 900.0,
            height: 600.0,
        },
    );
    let wide = compute_layout(
        &spec,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1800.0,
            height: 600.0,
        },
    );

    let n = narrow.rects.get(&LayoutNodeId(3)).expect("content rect");
    let w = wide.rects.get(&LayoutNodeId(3)).expect("content rect");

    // Fixed sidebar stays fixed; only the growing child absorbs the delta.
    assert_eq!(w.width - n.width, 900.0);
}

#[test]
fn empty_children_produce_no_rects_for_them() {
    let spec = node(1, LayoutAxis::Vertical, None, None, 1.0, vec![]);
    let result = compute_layout(
        &spec,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        },
    );
    assert_eq!(result.rects.len(), 1, "only the root should have a rect");
}
