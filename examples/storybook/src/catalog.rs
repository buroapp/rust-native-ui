//! The component catalog.
//!
//! One [`Entry`] per component kind the framework exposes. Each entry builds one
//! or more [`Demo`]s — a named configuration rendered in isolation, which is what
//! makes a component library reviewable rather than a single screensaver of a
//! demo app.
//!
//! Adding a component kind to `rn-core` should mean adding an entry here. The
//! `catalog_covers_every_component_kind` test at the bottom of this module fails
//! when the two drift apart, so this file is not something that can quietly rot.

use rn::{ColorValue, Component, HandlerId, PropKey, PropValue, StyledSpan};

/// A single named configuration of a component.
pub struct Demo {
    pub name: &'static str,
    pub component: Component,
}

/// A documented component, with its demos.
pub struct Entry {
    pub category: &'static str,
    pub name: &'static str,
    /// One line describing what the component is for.
    pub summary: &'static str,
    pub build: fn(&mut Ids) -> Vec<Demo>,
}

/// Deterministic handler-id allocator.
///
/// Ids are handed out in call order, and the traversal resets at the start of
/// every rebuild, so the same story gets the same handler id on every render.
/// Without that, every re-render would rewire every control.
pub struct Ids {
    pool: Vec<HandlerId>,
    next: usize,
}

impl Ids {
    #[must_use]
    pub fn new(pool: Vec<HandlerId>) -> Self {
        Self { pool, next: 0 }
    }

    /// Take the next handler id, wrapping when the pool runs out.
    pub fn take(&mut self) -> HandlerId {
        if self.pool.is_empty() {
            return HandlerId::new();
        }
        let id = self.pool[self.next % self.pool.len()];
        self.next += 1;
        id
    }

    /// Reset allocation so a rebuild reproduces the same ids.
    pub fn reset(&mut self) {
        self.next = 0;
    }
}

fn opts(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn demo(name: &'static str, component: Component) -> Demo {
    Demo { name, component }
}

// ---------------------------------------------------------------------------
// Basic
// ---------------------------------------------------------------------------

fn label_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("Default", rn::label("Hello, Rust Native!")),
        demo("Pinned width", rn::label_w("Minimum 160pt wide", 160.0)),
        demo(
            "Long text",
            rn::label("A considerably longer string that should truncate rather than overflow its container"),
        ),
    ]
}

fn button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("Enabled", rn::button("Click me").on_press(ids.take()).build()),
        demo(
            "Disabled",
            rn::button("Not available").enabled(false).build(),
        ),
        demo("With handler", rn::button("Reports an event").on_press(ids.take()).build()),
    ]
}

fn text_input_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("Empty", rn::text_input().build()),
        demo(
            "With placeholder",
            rn::text_input().placeholder("Type something…").build(),
        ),
        demo("Bound value", rn::text_input().value("Pre-filled").build()),
    ]
}

fn text_area_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Multi-line",
        rn::text_area("First line\nSecond line\nThird line").build(),
    )]
}

fn checkbox_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo(
            "Unchecked",
            rn::checkbox("Enable dark mode", false)
                .on_change(ids.take())
                .build(),
        ),
        demo(
            "Checked",
            rn::checkbox("Agree to terms", true)
                .on_change(ids.take())
                .build(),
        ),
    ]
}

fn separator_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("Horizontal rule", rn::separator()),
        demo("Hairline divider", rn::divider()),
    ]
}

fn image_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("SF Symbol — star", rn::image("star.fill").build()),
        demo("SF Symbol — folder", rn::image("folder").build()),
    ]
}

fn badge_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Variants",
        rn::hstack()
            .gap(8.0)
            .child(rn::badge("Default").build())
            .child(rn::badge("Success").build())
            .child(rn::badge("Destructive").build())
            .build(),
    )]
}

fn skeleton_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Placeholder lines",
        rn::vstack()
            .gap(6.0)
            .child(rn::skeleton(180.0, 16.0).build())
            .child(rn::skeleton(140.0, 16.0).build())
            .build(),
    )]
}

// ---------------------------------------------------------------------------
// Range and progress
// ---------------------------------------------------------------------------

fn slider_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("Mid value", rn::slider(42.0, 0.0, 100.0).build()),
        demo("Maximum", rn::slider(100.0, 0.0, 100.0).build()),
    ]
}

fn progress_bar_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("42%", rn::progress_bar(42.0).build()),
        demo("90%", rn::progress_bar(90.0).build()),
        demo("Complete", rn::progress_bar(100.0).build()),
    ]
}

fn stepper_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Default range", rn::stepper(10.0, 0.0, 100.0, 1.0).build())]
}

fn activity_indicator_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Spinning", rn::activity_indicator().build())]
}

fn capacity_indicator_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("70 of 100", rn::capacity_indicator(70.0, 100.0).build()),
        demo("Nearly full", rn::capacity_indicator(95.0, 100.0).build()),
    ]
}

fn rating_indicator_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("4 of 5", rn::rating_indicator(4.0, 5.0).build()),
        demo("2 of 5", rn::rating_indicator(2.0, 5.0).build()),
    ]
}

fn relevance_indicator_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("High relevance", rn::relevance_indicator(0.9).build()),
        demo("Low relevance", rn::relevance_indicator(0.2).build()),
    ]
}

// ---------------------------------------------------------------------------
// Choice
// ---------------------------------------------------------------------------

fn dropdown_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Three options",
        rn::dropdown(opts(&["Option A", "Option B", "Option C"]), 0).build(),
    )]
}

fn popup_button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Pull-down",
        rn::popup_button(opts(&["Option A", "Option B"]), 0, ids.take()).build(),
    )]
}

fn radio_group_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Three choices",
        rn::radio_group(opts(&["Small", "Medium", "Large"]), 1).build(),
    )]
}

fn segmented_control_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Three segments",
        rn::segmented_control(opts(&["Light", "Dark", "Auto"]), 0, ids.take()).build(),
    )]
}

fn switch_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("On", rn::switch(true, ids.take()).build()),
        demo("Off", rn::switch(false, ids.take()).build()),
    ]
}

fn color_well_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Orange",
        rn::color_well(ColorValue::rgb(255, 128, 0)).build(),
    )]
}

// ---------------------------------------------------------------------------
// Pickers
// ---------------------------------------------------------------------------

fn date_picker_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Fixed date", rn::date_picker("2026-11-04").build())]
}

fn time_picker_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Minutes with stepper", rn::time_picker("10:30").build())]
}

fn graphical_time_picker_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Clock face",
        rn::graphical_time_picker("10:30").build(),
    )]
}

// ---------------------------------------------------------------------------
// Lists and rows
// ---------------------------------------------------------------------------

fn sidebar_demos(ids: &mut Ids) -> Vec<Demo> {
    let items: Vec<Component> = ["Inbox", "Sent", "Drafts", "Trash"]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            rn::sidebar_item(*name, ids.take())
                .prop(PropKey::Selected, PropValue::Bool(i == 0))
                .build()
        })
        .collect();
    vec![demo("Four items", rn::sidebar().children(items).build())]
}

fn sidebar_section_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Section with rows",
        rn::sidebar()
            .child(
                rn::vstack()
                    .padding(6.0)
                    .gap(2.0)
                    .child(rn::sidebar_section("Basic").build())
                    .child(rn::sidebar_item("Buttons", ids.take()).build())
                    .child(rn::sidebar_item("Labels", ids.take()).build())
                    .child(rn::sidebar_section("Layout").build())
                    .child(rn::sidebar_item("TabView", ids.take()).build())
                    .build(),
            )
            .build(),
    )]
}

fn sidebar_item_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo(
            "Selected",
            rn::sidebar_item("General", ids.take())
                .prop(PropKey::Selected, PropValue::Bool(true))
                .build(),
        ),
        demo("Unselected", rn::sidebar_item("Appearance", ids.take()).build()),
    ]
}

fn list_tile_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo(
            "With icon and chevron",
            rn::list_tile("About")
                .prop(PropKey::IconName, PropValue::String("info.circle".into()))
                .on_press(ids.take())
                .build(),
        ),
        demo(
            "With subtitle",
            rn::list_tile("Software Update")
                .prop(
                    PropKey::IconName,
                    PropValue::String("arrow.triangle.2.circlepath".into()),
                )
                .prop(PropKey::Subtitle, PropValue::String("Up to date".into()))
                .build(),
        ),
    ]
}

fn grouped_section_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Three rows with dividers",
        rn::grouped_section()
            .child(rn::label("About"))
            .child(rn::divider())
            .child(rn::label("Storage"))
            .child(rn::divider())
            .child(rn::label("Startup Disk"))
            .build(),
    )]
}

fn flat_list_demos(ids: &mut Ids) -> Vec<Demo> {
    let rows: Vec<Component> = (1..=20).map(|i| rn::label(format!("Row {i}"))).collect();
    vec![demo(
        "Twenty virtualised rows",
        rn::flat_list(ids.take())
            .prop(PropKey::ItemHeightFixed, PropValue::F32(28.0))
            .children(rows)
            .build(),
    )]
}

fn scroll_view_demos(_ids: &mut Ids) -> Vec<Demo> {
    let rows: Vec<Component> = (1..=12).map(|i| rn::label(format!("Scrollable line {i}"))).collect();
    vec![demo(
        "Twelve lines in a viewport",
        rn::scroll_view().children(rows).build(),
    )]
}

fn touchable_opacity_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Press target",
        rn::touchable_opacity(ids.take())
            .child(rn::label("Press anywhere in this row"))
            .build(),
    )]
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn layout_container_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo(
            "Vertical stack",
            rn::vstack()
                .gap(6.0)
                .child(rn::label("First"))
                .child(rn::label("Second"))
                .child(rn::label("Third"))
                .build(),
        ),
        demo(
            "Horizontal, space between",
            rn::hstack()
                .justify_between()
                .child(rn::label("Left"))
                .child(rn::label("Right"))
                .build(),
        ),
    ]
}

fn card_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Card with content",
        rn::card()
            .child(rn::label("A card groups related content"))
            .child(rn::badge("Detail").build())
            .build(),
    )]
}

fn content_area_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Content area",
        rn::content_area()
            .child(rn::label("The scrollable main pane of a window"))
            .build(),
    )]
}

fn scaffold_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Scaffold with title bar",
        rn::scaffold()
            .child(rn::title_bar("Scaffolded").build())
            .child(rn::separator())
            .child(rn::label("Content below the title bar"))
            .build(),
    )]
}

fn split_view_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Two panes",
        rn::split_view()
            .child(rn::sidebar().child(rn::sidebar_item("Left", HandlerId::new()).build()).build())
            .child(rn::vstack().padding(12.0).child(rn::label("Right pane")).build())
            .build(),
    )]
}

fn tab_view_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Three tabs",
        rn::tab_view()
            .child(
                rn::tab_item("General")
                    .child(rn::label("General settings live here"))
                    .build(),
            )
            .child(
                rn::tab_item("Advanced")
                    .child(rn::label("Advanced settings live here"))
                    .build(),
            )
            .child(
                rn::tab_item("About")
                    .child(rn::label("Rust Native 0.1.0"))
                    .build(),
            )
            .build(),
    )]
}

fn tab_item_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "One page in a TabView",
        rn::tab_view()
            .child(
                rn::tab_item("Page")
                    .child(rn::label("A tab item is a page; its children are the content."))
                    .build(),
            )
            .build(),
    )]
}

fn safe_area_view_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Inset container",
        rn::safe_area_view()
            .child(rn::label("Respects the safe area"))
            .build(),
    )]
}

fn search_bar_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "With placeholder",
        rn::search_bar("Search settings…", ids.take()).build(),
    )]
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

fn window_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Not renderable inline",
        rn::label("Window is the root component — a window cannot be nested inside one."),
    )]
}

fn title_bar_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Centred title", rn::title_bar("Window Title").build())]
}

fn toolbar_demos(ids: &mut Ids) -> Vec<Demo> {
    let actions = vec![
        rn::ToolbarAction::new("compose", ids.take())
            .label("Compose")
            .icon("square.and.pencil"),
        rn::ToolbarAction::new("delete", ids.take())
            .label("Delete")
            .icon("trash"),
    ];
    vec![
        demo("Two actions", rn::toolbar(actions).build()),
        // The "sliver toolbar" pattern: the same toolbar as a child of a scroll
        // view, so it scrolls away with the content. No separate component needed.
        demo(
            "Sliver — scrolls with content",
            rn::scroll_view()
                .child(
                    rn::vstack()
                        .child(rn::toolbar(vec![rn::ToolbarAction::new("a", ids.take()).icon("gear")]).build())
                        .children((1..=8).map(|i| rn::label(format!("Content line {i}"))))
                        .build(),
                )
                .build(),
        ),
    ]
}

fn toolbar_icon_button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Gear",
        rn::toolbar_icon_button("gear", ids.take()).build(),
    )]
}

fn toolbar_spacer_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Spacer between items",
        rn::hstack()
            .child(rn::label("Left"))
            .child(rn::toolbar_spacer().build())
            .child(rn::label("Right"))
            .build(),
    )]
}

fn toolbar_divider_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Vertical rule", rn::toolbar_divider().build())]
}

fn toolbar_popup_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Pull-down menu",
        rn::toolbar_popup(opts(&["Sort by date", "Sort by sender"]), 0, ids.take()).build(),
    )]
}

fn status_bar_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Status bar", rn::status_bar().build())]
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

fn icon_button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![
        demo("Star", rn::icon_button("star", ids.take()).build()),
        demo("Heart", rn::icon_button("heart", ids.take()).build()),
    ]
}

fn back_button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Chevron left", rn::back_button(ids.take()).build())]
}

fn disclosure_button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Toggle",
        rn::disclosure_button("Show details", ids.take()).build(),
    )]
}

fn help_button_demos(ids: &mut Ids) -> Vec<Demo> {
    vec![demo("Question mark", rn::help_button("Help", ids.take()).build())]
}

// ---------------------------------------------------------------------------
// Text and media
// ---------------------------------------------------------------------------

fn rich_text_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Styled spans",
        rn::rich_text(vec![
            StyledSpan::new("Bold lead-in. ").bold(),
            StyledSpan::new("Italic aside. ").italic(),
            StyledSpan::new("And plain text to finish."),
        ])
        .build(),
    )]
}

fn tooltip_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Hint text",
        rn::tooltip("Explains what the control does").build(),
    )]
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

fn modal_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Visible modal",
        rn::modal(true)
            .child(rn::label("Modal content"))
            .build(),
    )]
}

fn popover_demos(ids: &mut Ids) -> Vec<Demo> {
    let anchor = ids.take();
    // Popover anchors to a node id, which only exists once the tree is built.
    // The demo shows the component with an unassigned anchor and says so.
    let _ = anchor;
    vec![demo(
        "Requires an anchor node",
        rn::label("Popover anchors to another component's node id; it has no standalone form."),
    )]
}

fn alert_dialog_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Title and message",
        rn::alert_dialog("Delete this item?", "This cannot be undone.")
            .build(),
    )]
}

fn sheet_demos(_ids: &mut Ids) -> Vec<Demo> {
    vec![demo(
        "Attached sheet",
        rn::sheet()
            .child(rn::label("Sheet content"))
            .build(),
    )]
}

// ---------------------------------------------------------------------------
// The catalog
// ---------------------------------------------------------------------------

pub const CATALOG: &[Entry] = &[
    Entry { category: "Basic", name: "Label", summary: "Static text", build: label_demos },
    Entry { category: "Basic", name: "Button", summary: "Push button", build: button_demos },
    Entry { category: "Basic", name: "TextInput", summary: "Single-line text entry", build: text_input_demos },
    Entry { category: "Basic", name: "TextArea", summary: "Multi-line text entry", build: text_area_demos },
    Entry { category: "Basic", name: "Checkbox", summary: "Binary choice with a label", build: checkbox_demos },
    Entry { category: "Basic", name: "Separator", summary: "Rule between regions", build: separator_demos },
    Entry { category: "Basic", name: "Image", summary: "Icon or picture", build: image_demos },
    Entry { category: "Basic", name: "Badge", summary: "Small status label", build: badge_demos },
    Entry { category: "Basic", name: "Skeleton", summary: "Loading placeholder", build: skeleton_demos },

    Entry { category: "Range", name: "Slider", summary: "Continuous value in a range", build: slider_demos },
    Entry { category: "Range", name: "ProgressBar", summary: "Determinate progress", build: progress_bar_demos },
    Entry { category: "Range", name: "Stepper", summary: "Discrete numeric stepper", build: stepper_demos },
    Entry { category: "Range", name: "ActivityIndicator", summary: "Indeterminate progress", build: activity_indicator_demos },
    Entry { category: "Range", name: "CapacityIndicator", summary: "Segmented capacity meter", build: capacity_indicator_demos },
    Entry { category: "Range", name: "RatingIndicator", summary: "Star rating", build: rating_indicator_demos },
    Entry { category: "Range", name: "RelevanceIndicator", summary: "Search relevance meter", build: relevance_indicator_demos },

    Entry { category: "Choice", name: "Dropdown", summary: "List of options", build: dropdown_demos },
    Entry { category: "Choice", name: "PopupButton", summary: "Bezeled pull-down", build: popup_button_demos },
    Entry { category: "Choice", name: "RadioGroup", summary: "Exclusive choices", build: radio_group_demos },
    Entry { category: "Choice", name: "SegmentedControl", summary: "Horizontal exclusive choice", build: segmented_control_demos },
    Entry { category: "Choice", name: "Switch", summary: "On/off toggle", build: switch_demos },
    Entry { category: "Choice", name: "ColorWell", summary: "Colour picker swatch", build: color_well_demos },

    Entry { category: "Pickers", name: "DatePicker", summary: "Calendar date", build: date_picker_demos },
    Entry { category: "Pickers", name: "TimePicker", summary: "Time of day", build: time_picker_demos },
    Entry { category: "Pickers", name: "GraphicalTimePicker", summary: "Clock-face time picker", build: graphical_time_picker_demos },

    Entry { category: "Lists", name: "Sidebar", summary: "Navigation rail", build: sidebar_demos },
    Entry { category: "Lists", name: "SidebarItem", summary: "A sidebar row", build: sidebar_item_demos },
    Entry { category: "Lists", name: "SidebarSection", summary: "Group header in a sidebar", build: sidebar_section_demos },
    Entry { category: "Lists", name: "ListTile", summary: "Settings-style row", build: list_tile_demos },
    Entry { category: "Lists", name: "GroupedSection", summary: "Grouped rows with dividers", build: grouped_section_demos },
    Entry { category: "Lists", name: "FlatList", summary: "Virtualised list", build: flat_list_demos },
    Entry { category: "Lists", name: "ScrollView", summary: "Scrollable region", build: scroll_view_demos },
    Entry { category: "Lists", name: "TouchableOpacity", summary: "Pressing container", build: touchable_opacity_demos },

    Entry { category: "Layout", name: "LayoutContainer", summary: "Stacking container", build: layout_container_demos },
    Entry { category: "Layout", name: "Card", summary: "Grouped content surface", build: card_demos },
    Entry { category: "Layout", name: "ContentArea", summary: "Main window pane", build: content_area_demos },
    Entry { category: "Layout", name: "Scaffold", summary: "Window structure", build: scaffold_demos },
    Entry { category: "Layout", name: "SplitView", summary: "Resizable panes", build: split_view_demos },
    Entry { category: "Layout", name: "TabView", summary: "Native tabbed panes", build: tab_view_demos },
    Entry { category: "Layout", name: "TabItem", summary: "One page of a TabView", build: tab_item_demos },
    Entry { category: "Layout", name: "SafeAreaView", summary: "Inset container", build: safe_area_view_demos },
    Entry { category: "Layout", name: "SearchBar", summary: "Search field", build: search_bar_demos },

    Entry { category: "Chrome", name: "Window", summary: "Root window", build: window_demos },
    Entry { category: "Chrome", name: "TitleBar", summary: "Window title strip", build: title_bar_demos },
    Entry { category: "Chrome", name: "Toolbar", summary: "Action bar", build: toolbar_demos },
    Entry { category: "Chrome", name: "ToolbarIconButton", summary: "Toolbar icon action", build: toolbar_icon_button_demos },
    Entry { category: "Chrome", name: "ToolbarSpacer", summary: "Flexible gap", build: toolbar_spacer_demos },
    Entry { category: "Chrome", name: "ToolbarDivider", summary: "Rule between actions", build: toolbar_divider_demos },
    Entry { category: "Chrome", name: "ToolbarPopup", summary: "Toolbar pull-down", build: toolbar_popup_demos },
    Entry { category: "Chrome", name: "StatusBar", summary: "Bottom status strip", build: status_bar_demos },

    Entry { category: "Buttons", name: "IconButton", summary: "Icon-only button", build: icon_button_demos },
    Entry { category: "Buttons", name: "BackButton", summary: "Navigation chevron", build: back_button_demos },
    Entry { category: "Buttons", name: "DisclosureButton", summary: "Show/hide toggle", build: disclosure_button_demos },
    Entry { category: "Buttons", name: "HelpButton", summary: "Contextual help", build: help_button_demos },

    Entry { category: "Text", name: "RichText", summary: "Mixed formatting", build: rich_text_demos },
    Entry { category: "Text", name: "Tooltip", summary: "Hint on a control", build: tooltip_demos },

    Entry { category: "Dialogs", name: "Modal", summary: "Blocking overlay", build: modal_demos },
    Entry { category: "Dialogs", name: "Popover", summary: "Anchored overlay", build: popover_demos },
    Entry { category: "Dialogs", name: "AlertDialog", summary: "Alert with choice", build: alert_dialog_demos },
    Entry { category: "Dialogs", name: "Sheet", summary: "Window-attached modal", build: sheet_demos },
    Entry {
        category: "Data",
        name: "Table",
        summary: "Sortable, resizable column table",
        build: |_ids| vec![
            demo("Table (Phase A in progress)", rn::label("Table renders here once Phase A ships.")),
        ],
    },
    Entry {
        category: "Data",
        name: "Tree",
        summary: "Expandable outline view",
        build: |_ids| vec![
            demo("Tree (Phase A in progress)", rn::label("Tree renders here once Phase A ships.")),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The point of this file: a component kind that has no catalog entry is a
    /// component nobody reviews. This test fails when `rn-core` gains a kind
    /// that the storybook does not document.
    #[test]
    fn catalog_covers_every_component_kind() {
        let documented: Vec<String> = CATALOG.iter().map(|e| e.name.to_string()).collect();
        let undocumented: Vec<String> = rn::ComponentKind::ALL
            .iter()
            .map(|k| format!("{k:?}"))
            // LayoutContainer carries an axis in its Debug form.
            .map(|n| {
                n.split('(')
                    .next()
                    .map_or(n.clone(), std::string::ToString::to_string)
            })
            .filter(|name| !documented.contains(name))
            .collect();

        assert!(
            undocumented.is_empty(),
            "component kinds with no storybook entry: {undocumented:?}\n\
             add them to CATALOG in examples/storybook/src/catalog.rs"
        );
    }

    #[test]
    fn every_entry_has_at_least_one_demo() {
        let mut ids = Ids::new((0..64).map(|_| HandlerId::new()).collect());
        for entry in CATALOG {
            ids.reset();
            let demos = (entry.build)(&mut ids);
            assert!(
                !demos.is_empty(),
                "entry '{}' produced no demos",
                entry.name
            );
            for d in &demos {
                assert!(!d.name.is_empty(), "entry '{}' has an unnamed demo", entry.name);
            }
        }
    }

    #[test]
    fn no_duplicate_entries() {
        let mut seen: Vec<&str> = Vec::new();
        for entry in CATALOG {
            assert!(
                !seen.contains(&entry.name),
                "duplicate catalog entry for '{}'",
                entry.name
            );
            seen.push(entry.name);
        }
    }

    #[test]
    fn ids_are_deterministic_across_rebuilds() {
        let pool: Vec<HandlerId> = (0..8).map(|_| HandlerId::new()).collect();
        let mut ids = Ids::new(pool);

        ids.reset();
        let first: Vec<HandlerId> = (0..5).map(|_| ids.take()).collect();
        ids.reset();
        let second: Vec<HandlerId> = (0..5).map(|_| ids.take()).collect();

        assert_eq!(
            first, second,
            "handler ids must repeat across rebuilds, or every re-render rewires every control"
        );
    }
}
