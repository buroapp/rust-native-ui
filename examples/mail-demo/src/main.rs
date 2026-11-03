//! mail-demo — macOS Mail-style app demonstrating SplitView, FlatList, RichText, Toolbar.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() -> Result<(), rn::RnError> {
    use std::sync::{Arc, Mutex};
    use rn::{ComponentTree, HandlerId, UpdateMessage, PropKey, PropValue, StyledSpan, ToolbarAction};
    use rn::backend::MacosRenderer;

    env_logger::init();
    rn::thread::MainThreadGuard::init_current();
    let mut renderer = MacosRenderer::new().map_err(rn::RnError::Render)?;

    #[derive(Clone)]
    struct State {
        selected_folder: usize,
        selected_msg: usize,
        action_log: String,
    }

    let state = Arc::new(Mutex::new(State {
        selected_folder: 0, selected_msg: 0, action_log: String::new(),
    }));

    enum Msg { Folder(usize), Message(usize), Toolbar(String), Close }

    let tx = rn::update_sender();
    let close_id = HandlerId::new();
    rn::on(close_id, |_| std::process::exit(0));

    let folders = ["Inbox", "Sent", "Drafts", "Spam", "Trash"];
    let mut folder_ids: Vec<HandlerId> = Vec::new();
    for i in 0..5usize {
        let id = HandlerId::new();
        let tx2 = tx.clone();
        rn::on(id, move |_| { let _ = tx2.send(UpdateMessage::new(Msg::Folder(i))); });
        folder_ids.push(id);
    }
    let folder_ids = Arc::new(folder_ids);

    let mut msg_ids: Vec<HandlerId> = Vec::new();
    for i in 0..100usize {
        let id = HandlerId::new();
        let tx2 = tx.clone();
        rn::on(id, move |_| { let _ = tx2.send(UpdateMessage::new(Msg::Message(i))); });
        msg_ids.push(id);
    }
    let msg_ids = Arc::new(msg_ids);

    let reply_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(reply_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::Toolbar("reply".into()))); }); }
    let del_id = HandlerId::new();
    { let tx = tx.clone(); rn::on(del_id, move |_| { let _ = tx.send(UpdateMessage::new(Msg::Toolbar("delete".into()))); }); }

    let toolbar = vec![
        ToolbarAction::new("reply", reply_id).label("Reply").icon("arrowshape.turn.up.left"),
        ToolbarAction::new("delete", del_id).label("Delete").icon("trash"),
    ];

    let build_view = {
        let state = Arc::clone(&state);
        let folder_ids = Arc::clone(&folder_ids);
        let msg_ids = Arc::clone(&msg_ids);
        let toolbar = toolbar.clone();
        move || {
            let s = state.lock().map(|g| g.clone()).unwrap_or(State {
                selected_folder: 0, selected_msg: 0, action_log: String::new(),
            });

            let folder_items: Vec<rn::Component> = folders.iter().enumerate().map(|(i, lbl)| {
                rn::sidebar_item(*lbl, folder_ids[i])
                    .prop(PropKey::Selected, PropValue::Bool(i == s.selected_folder))
                    .build()
            }).collect();
            let left = rn::sidebar().children(folder_items).build();

            let msg_rows: Vec<rn::Component> = (0..100usize).map(|i| {
                rn::touchable_opacity(msg_ids[i])
                    .child(rn::hstack().gap(4.0).padding(8.0)
                        .child(rn::label_w(
                            if i == s.selected_msg { "▶" } else { "  " }, 16.0
                        ))
                        .child(rn::label(format!("Message {} — Subject line here", i + 1)))
                        .build())
                    .build()
            }).collect();

            let middle = rn::scroll_view().children(msg_rows).build();

            let spans = vec![
                StyledSpan::new(format!("Message {}\n\n", s.selected_msg + 1)).bold(),
                StyledSpan::new("From: sender@example.com\n\n").italic()
                    .color(rn::ColorToken::Muted),
                StyledSpan::new("This is the body of the selected message. ")
                    .color(rn::ColorToken::Foreground),
                StyledSpan::new("Bold text here.").bold().color(rn::ColorToken::Primary),
            ];

            let right = rn::vstack().gap(0.0)
                .child(rn::toolbar(toolbar.clone()).build())
                .child(rn::separator())
                .child(rn::scroll_view()
                    .child(rn::rich_text(spans)
                        .prop(PropKey::TextWrap, PropValue::Bool(true))
                        .build())
                    .build())
                .child(rn::label_w(
                    if s.action_log.is_empty() { "Toolbar: —" } else { &s.action_log },
                    300.0
                ))
                .build();

            ComponentTree::new(
                rn::window()
                    .title("Mail Demo")
                    .size(900.0, 600.0)
                    .on_close(close_id)
                    .child(
                        rn::split_view()
                            .child(left)
                            .child(middle)
                            .child(right)
                            .build()
                    )
                    .build()
            )
        }
    };

    rn::render(&mut renderer, build_view())?;

    {
        let state = Arc::clone(&state);
        let bv = build_view.clone();
        rn::set_tick(move || {
            let batch = rn::drain_updates();
            if batch.is_empty() { return; }
            let mut dirty = false;
            if let Ok(mut g) = state.lock() {
                for msg in batch {
                    match msg.downcast::<Msg>() {
                        Ok(Msg::Folder(i)) => { g.selected_folder = i; dirty = true; }
                        Ok(Msg::Message(i)) => { g.selected_msg = i; dirty = true; }
                        Ok(Msg::Toolbar(a)) => { g.action_log = format!("Toolbar: {a}"); dirty = true; }
                        Ok(Msg::Close) | Err(_) => {}
                    }
                }
            }
            if dirty { if let Err(e) = rn::rerender(bv()) { eprintln!("rerender: {e}"); } }
        });
    }

    rn::run(&mut renderer)
}

#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() { eprintln!("mail-demo requires --features macos on macOS"); }
