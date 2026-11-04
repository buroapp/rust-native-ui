//! A Finder window, rebuilt with Rust Native.
//!
//! This is a capability test, not a Finder clone: every element here is one a
//! real app would need, and building it surfaces what the framework is missing
//! faster than any specification would. The comments name those gaps as they
//! appear so they do not read as deliberate omissions.

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use rn::backend::MacosRenderer;
    use rn::HandlerId;

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();
    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    let close = HandlerId::new();
    rn::on(close, |_| std::process::exit(0));

    let mut ids: Vec<HandlerId> = (0..64).map(|_| HandlerId::new()).collect();
    for id in &ids {
        let id = *id;
        rn::on(id, move |_| log::info!("finder-demo: action {id:?}"));
    }
    let mut next = || ids.pop().unwrap_or_else(HandlerId::new);

    // ---- Sidebar -----------------------------------------------------------
    // Sections and icons are what we just added; selection is a real pill.
    let sidebar = rn::sidebar()
        .prop(rn::PropKey::SidebarWidth, rn::PropValue::F32(200.0))
        .child(
            rn::vstack()
                .padding(8.0)
                .gap(2.0)
                .child(
                    rn::sidebar_item("Recents", next())
                        .icon("clock")
                        .prop(rn::PropKey::Selected, rn::PropValue::Bool(true))
                        .build(),
                )
                .child(rn::sidebar_item("Shared", next()).icon("person.2").build())
                .child(rn::sidebar_section("Favourites").build())
                .child(rn::sidebar_item("Applications", next()).icon("square.grid.2x2").build())
                .child(rn::sidebar_item("Desktop", next()).icon("desktopcomputer").build())
                .child(rn::sidebar_item("Documents", next()).icon("doc.text").build())
                .child(rn::sidebar_item("Downloads", next()).icon("arrow.down.circle").build())
                .child(rn::sidebar_item("Developer", next()).icon("hammer").build())
                .child(rn::sidebar_section("Locations").build())
                .child(rn::sidebar_item("iCloud Drive", next()).icon("cloud").build())
                .child(rn::sidebar_item("I779886", next()).icon("house").build())
                .child(rn::sidebar_item("Macintosh HD", next()).icon("internaldrive").build())
                .build(),
        )
        .build();

    // ---- Toolbar -----------------------------------------------------------
    // GAP: Finder's toolbar is an NSToolbar inside the titlebar. Ours is a view
    // in the content tree, so it sits below the titlebar instead of merged with it.
    let back = rn::hstack()
        .gap(2.0)
        .child(rn::icon_button("chevron.left", next()).build())
        .child(rn::icon_button("chevron.right", next()).build())
        .build();

    // GAP: view modes want image-only segments. Our segmented control takes text,
    // so four icon buttons stand in for it.
    let view_modes = rn::hstack()
        .gap(2.0)
        .child(rn::icon_button("square.grid.2x2", next()).build())
        .child(rn::icon_button("list.bullet", next()).build())
        .child(rn::icon_button("rectangle.split.3x1", next()).build())
        .child(rn::icon_button("rectangle.portrait", next()).build())
        .build();

    let toolbar = rn::hstack()
        .padding(10.0)
        .gap(10.0)
        .child(back)
        .child(rn::label("Recents"))
        .child(rn::toolbar_spacer().build())
        .child(view_modes)
        .child(rn::icon_button("square.grid.2x2", next()).build())
        .child(rn::toolbar_divider().build())
        .child(rn::icon_button("square.and.arrow.up", next()).build())
        .child(rn::icon_button("tag", next()).build())
        .child(rn::icon_button("ellipsis", next()).build())
        .child(rn::search_bar("Search", next()).build())
        .build();

    // ---- Icon grid ---------------------------------------------------------
    // A wrapping flow: items reflow to the window width instead of being
    // hand-grouped into rows of eight.
    fn grid_item(icon: &str, name: &str) -> rn::Component {
        rn::vstack()
            .gap(6.0)
            .align_center()
            .prop(rn::PropKey::Width, rn::PropValue::F32(104.0))
            .child(rn::image(icon).size(64.0, 64.0).build())
            // align_center stops children stretching, so the label needs the cell
            // width explicitly or it collapses to the 60pt minimum.
            .child(
                rn::label(name)
                    .text_center()
                    .lines(2)
                    .prop(rn::PropKey::Width, rn::PropValue::F32(104.0)),
            )
            .build()
    }

    let files: &[(&str, &str)] = &[
        ("doc.text", "architecture.html"),
        ("doc.richtext", "FHL_Participant_ Guide.pdf"),
        ("doc.text", "Deployment.md"),
        ("doc.text", "Release-Process-(Release-Board).md"),
        ("doc.text", "Engineering-Best-Practices.md"),
        ("doc.text", "App Center Framework.docx"),
        ("hammer", "LocalPasskeyManager.xcodeproj"),
        ("doc.text", "UTKIN_CV_2026_RING.pdf"),
        ("photo", "Screenshot 2026-11-04 at 12.14.07"),
        ("opticaldisc", "arduino-ide_2.3.4_amd64.dmg"),
        ("opticaldisc", "HyprMac.dmg"),
        ("opticaldisc", "HyprMac-0.17.1.dmg"),
        ("archivebox", "agram-1.4.7-macos.zip"),
        ("opticaldisc", "SideloadlySetup.dmg"),
        ("rectangle.on.rectangle", "2608_ServiceReview_Sep_About.pptx"),
        ("doc.text", "SKILL (1).md"),
        ("photo", "Screenshot 2026-03-11 at 11.58.19"),
        ("photo", "image5.png"),
        ("photo", "image4.png"),
        ("photo", "image3.png"),
        ("photo", "1image.png"),
        ("rectangle.on.rectangle", "Travel Payments TH Aug 2026.pptx"),
        ("photo", "222image.png"),
        ("opticaldisc", "Raycast_2.4.1.0_df4162826_arm64.dmg"),
    ];

    let items: Vec<rn::Component> = files
        .iter()
        .map(|(icon, name)| grid_item(icon, name))
        .collect();

    let content = rn::scroll_view()
        .child(
            rn::hstack()
                .wrap()
                .gap(16.0)
                .padding(20.0)
                .children(items)
                .build(),
        )
        .build();

    let tree = rn::ComponentTree::new(
        rn::window()
            .title("Recents")
            .size(1100.0, 700.0)
            .on_close(close)
            .child(
                rn::vstack()
                    .flex(1.0)
                    .gap(0.0)
                    .child(toolbar)
                    .child(rn::separator())
                    .child(
                        rn::hstack()
                            .flex(1.0)
                            .gap(0.0)
                            .child(sidebar)
                            .child(rn::separator())
                            .child(content)
                            .build(),
                    )
                    .build(),
            )
            .build(),
    );

    rn::render(&mut renderer, tree)?;
    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("finder-demo requires --features macos on macOS");
}
