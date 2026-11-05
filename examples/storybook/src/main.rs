//! Storybook for Rust Native.
//!
//! A catalog browser built with the framework it documents. Every component kind
//! gets an entry; selecting one shows its demos rendered in isolation with a note
//! about what it is for.
//!
//! Dogfooding is the point. A component library reviewed only through one
//! application demo hides its own defects: the first run of this app showed scroll
//! panes opening at their bottom and every sidebar row being unnamed to assistive
//! technology, neither of which was visible in the demo apps.

// The catalog only exists to serve this app, so it is gated with it. Keeping it
// ungated makes clippy report every type in it as dead code on a build without
// the macOS feature.
#[cfg(all(target_os = "macos", feature = "macos"))]
mod catalog;

#[cfg(all(target_os = "macos", feature = "macos"))]
/// SF Symbol per component. Kept here rather than in the catalogue so the
/// catalogue stays about what a component *is*, not how it is pictured.
fn icon_for(name: &str) -> &'static str {
    match name {
        "Label" => "textformat",
        "Button" => "hand.tap",
        "TextInput" => "character.cursor.ibeam",
        "TextArea" => "text.alignleft",
        "Checkbox" => "checkmark.square",
        "Separator" => "minus",
        "Image" => "photo",
        "Badge" => "tag.fill",
        "Skeleton" => "rectangle.dashed",
        "Slider" => "slider.horizontal.3",
        "ProgressBar" => "chart.bar.fill",
        "Stepper" => "plus.forwardslash.minus",
        "ActivityIndicator" => "arrow.triangle.2.circlepath",
        "CapacityIndicator" => "chart.pie.fill",
        "RatingIndicator" => "star.fill",
        "RelevanceIndicator" => "scope",
        "Dropdown" => "chevron.down.square",
        "PopupButton" => "rectangle.and.arrow.up.right.and.arrow.down.left",
        "RadioGroup" => "largecircle.fill.circle",
        "SegmentedControl" => "rectangle.split.3x1",
        "Switch" => "switch.2",
        "ColorWell" => "paintpalette",
        "DatePicker" => "calendar",
        "TimePicker" => "clock",
        "GraphicalTimePicker" => "clock.fill",
        "Sidebar" => "sidebar.left",
        "SidebarItem" => "list.bullet",
        "SidebarSection" => "textformat.size.smaller",
        "ListTile" => "list.bullet.rectangle",
        "GroupedSection" => "square.grid.2x2",
        "FlatList" => "list.dash",
        "ScrollView" => "scroll",
        "TouchableOpacity" => "hand.tap.fill",
        "LayoutContainer" => "square.split.2x1",
        "Card" => "rectangle",
        "ContentArea" => "rectangle.inset.filled",
        "Scaffold" => "macwindow",
        "SplitView" => "rectangle.split.2x1",
        "TabView" => "rectangle.topthird.inset.filled",
        "TabItem" => "rectangle",
        "SafeAreaView" => "rectangle.dashed",
        "SearchBar" => "magnifyingglass",
        "Window" => "macwindow",
        "TitleBar" => "menubar.rectangle",
        "Toolbar" => "menubar.rectangle",
        "ToolbarIconButton" => "square.grid.2x2",
        "ToolbarSpacer" => "arrow.left.and.right",
        "ToolbarDivider" => "minus",
        "ToolbarPopup" => "ellipsis.circle",
        "StatusBar" => "rectangle.bottomthird.inset.filled",
        "IconButton" => "circle",
        "BackButton" => "chevron.left",
        "DisclosureButton" => "chevron.down.circle",
        "HelpButton" => "questionmark.circle",
        "RichText" => "text.alignleft",
        "Tooltip" => "info.circle",
        "Modal" => "rectangle.portrait",
        "Popover" => "bubble.left",
        "AlertDialog" => "exclamationmark.triangle",
        "Sheet" => "rectangle.portrait.and.arrow.right",
        _ => "square",
    }
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("storybook requires --features macos on macOS");
}

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};

    use catalog::{Ids, CATALOG};

    /// Handler ids handed out to stories. Every story that wires an event takes
    /// one from this pool; it is created once so ids stay stable across renders.
    const HANDLER_POOL: usize = 128;

        use rn::backend::MacosRenderer;
        use rn::{ComponentTree, HandlerId, UpdateMessage};

        env_logger::init();

        // `storybook --list` dumps the catalog as JSON and exits. The doc generator
        // reads this so component docs have exactly one source of truth — the same
        // catalog the app and the drift test use.
        if std::env::args().any(|a| a == "--list") {
            print!("[");
            for (i, e) in CATALOG.iter().enumerate() {
                let comma = if i + 1 < CATALOG.len() { "," } else { "" };
                // Fields are plain ASCII identifiers and short English; escape quotes
                // defensively anyway.
                let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
                print!(
                    "{{\"index\":{},\"category\":\"{}\",\"name\":\"{}\",\"summary\":\"{}\"}}{}",
                    i, esc(e.category), esc(e.name), esc(e.summary), comma
                );
            }
            println!("]");
            return Ok(());
        }

        rn::thread::MainThreadGuard::init_current();
        let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

        #[derive(Clone)]
        struct State {
            selected: usize,
            status: String,
        }

        // Optional deep link: `storybook --select Switch` opens that component
        // directly. Useful for screenshots, for jumping to a component you are
        // working on, and for verifying a page without driving the mouse.
        let initial = std::env::args()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|w| w[0] == "--select")
            .and_then(|w| {
                let want = &w[1];
                want.parse::<usize>().ok().filter(|i| *i < CATALOG.len()).or_else(|| {
                    CATALOG.iter().position(|e| e.name.eq_ignore_ascii_case(want))
                })
            })
            .unwrap_or(0);

        let state = Arc::new(Mutex::new(State {
            selected: initial,
            status: "Select a component from the sidebar".to_string(),
        }));

        enum Msg {
            Select(usize),
            Fired(usize),
        }

        let tx = rn::update_sender();

        let close_id = HandlerId::new();
        rn::on(close_id, |_| std::process::exit(0));

        // One handler per sidebar entry.
        let nav_ids: Vec<HandlerId> = (0..CATALOG.len()).map(|_| HandlerId::new()).collect();
        for (i, id) in nav_ids.iter().enumerate() {
            let tx = tx.clone();
            rn::on(*id, move |_| {
                let _ = tx.send(UpdateMessage::new(Msg::Select(i)));
            });
        }

        // One handler pool for stories, each reporting which slot fired so the
        // status line can prove the event reached the application.
        let pool: Vec<HandlerId> = (0..HANDLER_POOL).map(|_| HandlerId::new()).collect();
        for (i, id) in pool.iter().enumerate() {
            let tx = tx.clone();
            rn::on(*id, move |_| {
                let _ = tx.send(UpdateMessage::new(Msg::Fired(i)));
            });
        }

        let nav_ids = Arc::new(nav_ids);
        let pool = Arc::new(pool);

        let build_view = {
            let state = Arc::clone(&state);
            let nav_ids = Arc::clone(&nav_ids);
            let pool = Arc::clone(&pool);

            move || {
                let s = state.lock().map(|g| g.clone()).unwrap_or(State {
                    selected: 0,
                    status: String::new(),
                });
                let selected = s.selected.min(CATALOG.len().saturating_sub(1));
                let entry = CATALOG.get(selected);

                let mut ids = Ids::new(pool.as_ref().clone());

                // ---- Sidebar: the component groups and scrolls its own rows ----
                let mut nav_items: Vec<rn::Component> = Vec::new();
                let mut last_category = "";
                for (i, e) in CATALOG.iter().enumerate() {
                    if e.category != last_category {
                        last_category = e.category;
                        nav_items.push(rn::sidebar_section(e.category).build());
                    }
                    nav_items.push(
                        rn::sidebar_item(e.name, nav_ids[i])
                            .icon(icon_for(e.name))
                            .prop(rn::PropKey::Selected, rn::PropValue::Bool(i == selected))
                            .build(),
                    );
                }

                // The Sidebar is a vibrancy surface; scrolling is the caller's call.
                let sidebar = rn::sidebar()
                    .prop(rn::PropKey::SidebarWidth, rn::PropValue::F32(260.0))
                    .child(
                        rn::scroll_view()
                            .child(rn::vstack().padding(6.0).gap(2.0).children(nav_items).build())
                            .build(),
                    )
                    .build();

                // ---- Content: header, then one block per demo ----
                let mut blocks: Vec<rn::Component> = Vec::new();

                if let Some(entry) = entry {
                    blocks.push(
                        rn::vstack()
                            .gap(2.0)
                            .child(rn::label(entry.name))
                            .child(rn::label(entry.summary))
                            .build(),
                    );
                    blocks.push(rn::separator());

                    ids.reset();
                    for d in (entry.build)(&mut ids) {
                        blocks.push(rn::label(d.name));
                        blocks.push(
                            rn::grouped_section()
                                .padding(10.0)
                                .child(d.component)
                                .build(),
                        );
                    }
                }

                blocks.push(rn::separator());
                blocks.push(rn::label(format!("Event: {}", s.status)));

                // ContentArea scrolls on its own; nesting a ScrollView here would
                // create two nested scrollers and halve the visible area.
                let content = rn::content_area()
                    .child(rn::vstack().padding(18.0).gap(10.0).children(blocks).build())
                    .build();

                ComponentTree::new(
                    rn::window()
                        .modern(true)
                        .title("Rust Native — Storybook")
                        .size(1000.0, 720.0)
                        .on_close(close_id)
                        .child(
                            rn::hstack()
                                .gap(0.0)
                                .flex(1.0)
                                .child(sidebar)
                                .child(rn::separator())
                                .child(content)
                                .build(),
                        )
                        .build(),
                )
            }
        };

        rn::render(&mut renderer, build_view())?;

        {
            let state = Arc::clone(&state);
            let bv = build_view.clone();
            rn::set_tick(move || {
                let batch = rn::drain_updates();
                if batch.is_empty() {
                    return;
                }
                let mut dirty = false;
                if let Ok(mut g) = state.lock() {
                    for msg in batch {
                        match msg.downcast::<Msg>() {
                            Ok(Msg::Select(i)) => {
                                g.selected = i;
                                g.status = format!(
                                    "Selected {}",
                                    CATALOG.get(i).map_or("?", |e| e.name)
                                );
                                dirty = true;
                            }
                            Ok(Msg::Fired(slot)) => {
                                g.status = format!("handler #{slot} fired");
                                dirty = true;
                            }
                            Err(_) => {}
                        }
                    }
                }
                if dirty {
                    if let Err(e) = rn::rerender(bv()) {
                        eprintln!("storybook: rerender failed: {e}");
                    }
                }
            });
        }

        rn::run(&mut renderer)
}
