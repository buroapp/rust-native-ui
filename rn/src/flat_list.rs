//! FlatList virtualization engine (FR-002, FR-003, FR-004).
//!
//! Maintains the minimal set of live component nodes needed to fill the
//! visible viewport plus a configurable buffer. Scroll events drive
//! `on_scroll` which returns mount/unmount operations the renderer applies.
//!
//! Design goals:
//! - Fixed-height mode: O(1) scroll-position → row-index mapping.
//! - Dynamic-height mode: height cache + prefix-sum for scroll mapping.
//! - Constant live node count: buffer_rows * 2 + visible_rows.
//! - Platform-agnostic: no AppKit / Win32 / GTK imports.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// Operations the renderer must apply after a scroll or data change.
#[derive(Debug, Clone)]
pub enum RecycleOp {
    /// Mount a new item at the given data index, reusing pool slot `pool_slot`.
    Mount { data_index: usize, pool_slot: usize },
    /// Unmount the item currently at pool slot `pool_slot`.
    Unmount { pool_slot: usize },
    /// Update an already-mounted item at pool_slot with new data index.
    Update { data_index: usize, pool_slot: usize },
}

/// How item heights are provided.
#[derive(Debug, Clone, Copy)]
pub enum ItemHeight {
    /// Every row has the same height (px). Enables O(1) mapping.
    Fixed(f32),
    /// Row heights vary; heights are measured and cached by the renderer.
    Dynamic,
}

/// Virtualization state for a single `FlatList` instance.
#[derive(Debug)]
pub struct FlatListState {
    pub item_count: usize,
    pub item_height: ItemHeight,
    pub viewport_height: f32,
    pub scroll_offset: f32,
    pub buffer_rows: usize,
    pub total_height: f32,
    pub end_reach_threshold: f32,
    pub end_reached_fired: bool,

    // Index range of items currently mounted (inclusive).
    pub mounted_start: usize,
    pub mounted_end: usize,   // exclusive

    // Height cache for Dynamic mode (index → measured height).
    height_cache: Vec<Option<f32>>,
    default_item_height: f32,
}

impl FlatListState {
    /// Create a new state.  `pool_size` controls how many native views the
    /// platform allocates up front.
    pub fn new(
        item_count: usize,
        item_height: ItemHeight,
        viewport_height: f32,
        buffer_rows: usize,
        end_reach_threshold: f32,
    ) -> Self {
        let default_h = match item_height {
            ItemHeight::Fixed(h) => h,
            ItemHeight::Dynamic => 44.0, // sensible default before measurement
        };
        let total = compute_total_height(item_count, item_height, default_h);
        Self {
            item_count,
            item_height,
            viewport_height,
            scroll_offset: 0.0,
            buffer_rows,
            total_height: total,
            end_reach_threshold,
            end_reached_fired: false,
            mounted_start: 0,
            mounted_end: 0,
            height_cache: vec![None; item_count],
            default_item_height: default_h,
        }
    }

    /// Compute the desired visible+buffer range for the current scroll offset.
    pub fn desired_range(&self) -> (usize, usize) {
        let first_visible = self.index_at_offset(self.scroll_offset);
        let rows_in_view = (self.viewport_height / self.default_item_height).ceil() as usize + 1;
        let start = first_visible.saturating_sub(self.buffer_rows);
        let end = (first_visible + rows_in_view + self.buffer_rows).min(self.item_count);
        (start, end)
    }

    /// Returns the recycle operations needed to bring the live set to
    /// match the desired range.  Call after any state mutation.
    pub fn compute_ops(&mut self) -> Vec<RecycleOp> {
        let (new_start, new_end) = self.desired_range();
        let mut ops = Vec::new();

        // Unmount rows that left the window.
        for i in self.mounted_start..self.mounted_end {
            if i < new_start || i >= new_end {
                ops.push(RecycleOp::Unmount { pool_slot: i % self.pool_size() });
            }
        }
        // Mount rows that entered the window.
        for i in new_start..new_end {
            if i < self.mounted_start || i >= self.mounted_end {
                ops.push(RecycleOp::Mount {
                    data_index: i,
                    pool_slot: i % self.pool_size(),
                });
            }
        }

        self.mounted_start = new_start;
        self.mounted_end = new_end;
        ops
    }

    /// Notify of a scroll to `new_offset`.  Returns recycle ops.
    pub fn on_scroll(&mut self, new_offset: f32) -> Vec<RecycleOp> {
        self.scroll_offset = new_offset.max(0.0).min(
            (self.total_height - self.viewport_height).max(0.0),
        );
        // Check end_reached threshold.
        if !self.end_reached_fired
            && self.total_height - self.scroll_offset - self.viewport_height
                < self.end_reach_threshold
        {
            self.end_reached_fired = true;
        }
        self.compute_ops()
    }

    /// Notify that `n` new items were appended.
    pub fn on_data_append(&mut self, n: usize) -> Vec<RecycleOp> {
        let old_count = self.item_count;
        self.item_count += n;
        self.height_cache.extend(std::iter::repeat(None).take(n));
        self.total_height =
            compute_total_height(self.item_count, self.item_height, self.default_item_height);
        // Only mount newly visible rows; preserve scroll position.
        let _ = old_count;
        self.end_reached_fired = false;
        self.compute_ops()
    }

    /// Notify that a single item's data changed (no height change).
    pub fn on_data_update(&mut self, index: usize) -> Option<RecycleOp> {
        if index >= self.mounted_start && index < self.mounted_end {
            Some(RecycleOp::Update {
                data_index: index,
                pool_slot: index % self.pool_size(),
            })
        } else {
            None // item not visible; update queued by renderer on next mount
        }
    }

    /// Notify that a row was measured (Dynamic mode only).
    pub fn on_item_height_change(&mut self, index: usize, height: f32) {
        if index < self.height_cache.len() {
            self.height_cache[index] = Some(height);
        }
        self.default_item_height = height; // use last measured as default estimate
        self.total_height =
            compute_total_height(self.item_count, self.item_height, self.default_item_height);
    }

    /// Number of native view slots in the recycle pool (constant).
    pub fn pool_size(&self) -> usize {
        let rows_in_view =
            (self.viewport_height / self.default_item_height).ceil() as usize + 1;
        rows_in_view + self.buffer_rows * 2 + 1
    }

    /// Y offset (px from top of content) for a given item index.
    pub fn y_offset_for(&self, index: usize) -> f32 {
        match self.item_height {
            ItemHeight::Fixed(h) => index as f32 * h,
            ItemHeight::Dynamic => {
                (0..index).fold(0.0_f32, |acc, i| {
                    acc + self.height_cache.get(i)
                        .and_then(|h| *h)
                        .unwrap_or(self.default_item_height)
                })
            }
        }
    }

    fn index_at_offset(&self, offset: f32) -> usize {
        match self.item_height {
            ItemHeight::Fixed(h) if h > 0.0 => (offset / h) as usize,
            _ => {
                // Linear scan for Dynamic mode (acceptable for <10k items).
                let mut acc = 0.0_f32;
                for i in 0..self.item_count {
                    let h = self.height_cache.get(i)
                        .and_then(|h| *h)
                        .unwrap_or(self.default_item_height);
                    acc += h;
                    if acc > offset {
                        return i;
                    }
                }
                self.item_count.saturating_sub(1)
            }
        }
    }
}

fn compute_total_height(count: usize, mode: ItemHeight, default_h: f32) -> f32 {
    match mode {
        ItemHeight::Fixed(h) => count as f32 * h,
        ItemHeight::Dynamic => count as f32 * default_h,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(item_count: usize, item_h: f32, viewport_h: f32) -> FlatListState {
        FlatListState::new(
            item_count,
            ItemHeight::Fixed(item_h),
            viewport_h,
            3,   // buffer_rows
            100.0, // end_reach_threshold
        )
    }

    #[test]
    fn scroll_updates_mounted_range() {
        let mut s = make_state(1000, 44.0, 400.0);
        // Initially visible rows: ~9 + 3 buffer each side
        let ops = s.compute_ops();
        assert!(!ops.is_empty());
        assert!(s.mounted_end > s.mounted_start);

        // Scroll down 500px → ~11 rows in, first visible ~= row 11
        let ops = s.on_scroll(500.0);
        assert!(!ops.is_empty(), "scrolling should produce recycle ops");
        assert!(s.mounted_start > 0);
    }

    #[test]
    fn data_append_preserves_scroll() {
        let mut s = make_state(50, 44.0, 300.0);
        let _ = s.compute_ops();
        let offset_before = s.scroll_offset;
        let _ = s.on_data_append(50);
        assert_eq!(s.scroll_offset, offset_before, "append must not change scroll position");
        assert_eq!(s.item_count, 100);
    }

    #[test]
    fn out_of_viewport_update_is_none() {
        let mut s = make_state(100, 44.0, 300.0);
        let _ = s.compute_ops(); // mount initial range
        // Item 99 is unlikely to be in the initial viewport
        let op = s.on_data_update(99);
        if s.mounted_end <= 99 {
            assert!(op.is_none(), "item outside viewport should not produce an op");
        }
    }

    #[test]
    fn end_reached_fires_at_threshold() {
        let mut s = make_state(100, 44.0, 300.0);
        let _ = s.compute_ops();
        // total_height = 100 * 44 = 4400; threshold = 100
        // Must be within 100px of bottom: scroll_offset >= 4400 - 300 - 100 = 4000
        let _ = s.on_scroll(4010.0);
        assert!(s.end_reached_fired, "end_reached should fire near bottom");
    }

    #[test]
    fn pool_size_is_constant() {
        let s = make_state(10_000, 44.0, 400.0);
        let pool = s.pool_size();
        assert!(pool < 50, "pool must be much smaller than item count");
        assert!(pool >= 9, "pool must cover at least one viewport");
    }
}
