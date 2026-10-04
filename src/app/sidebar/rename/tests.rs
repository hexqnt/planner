use super::*;
use crate::{app::Planner, model::Document};

fn frame(ctx: &egui::Context, planner: &mut Planner, key: Option<Key>) {
    let events = key
        .map(|key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        })
        .into_iter()
        .collect();
    frame_events(ctx, planner, events);
}

fn frame_events(ctx: &egui::Context, planner: &mut Planner, events: Vec<egui::Event>) {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| planner.sidebar(ui));
            planner.dialogs(ui.ctx());
        },
    )
    .drop_without_applying_deltas();
}

#[test]
fn clicking_outside_saves_a_name_or_discards_an_empty_name() {
    for text in ["  New name  ", "   "] {
        for category in [false, true] {
            let ctx = egui::Context::default();
            let mut planner = Planner::from_document(Document::default(), &ctx, None);
            let target = if category {
                TreeTarget::Category(planner.document.groups[0].categories[0].id)
            } else {
                TreeTarget::Group(0)
            };
            let original = name(&planner, target).to_owned();
            planner.rename_draft = Some(RenameDraft::new(target, &original));
            frame(&ctx, &mut planner, None);
            planner.rename_draft.as_mut().unwrap().text = text.into();
            let pos = egui::pos2(580.0, 850.0);

            for pressed in [true, false] {
                frame_events(
                    &ctx,
                    &mut planner,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            frame(&ctx, &mut planner, None);

            assert!(planner.rename_draft.is_none());
            assert_eq!(
                name(&planner, target),
                if text.trim().is_empty() {
                    &original
                } else {
                    "New name"
                }
            );
            assert!(planner.tree_draft.is_none());
        }
    }
}

fn name(planner: &Planner, target: TreeTarget) -> &str {
    match target {
        TreeTarget::Group(index) => planner.document.groups[index].name.get(planner.language()),
        TreeTarget::Category(id) => planner
            .document
            .category(id)
            .unwrap()
            .name
            .get(planner.language()),
    }
}

#[test]
fn rename_keeps_text_and_following_rows_in_place() {
    for dark in [false, true] {
        for category in [false, true] {
            let ctx = egui::Context::default();
            let document = Document {
                dark,
                ..Document::default()
            };
            let mut planner = Planner::from_document(document, &ctx, None);
            let target = if category {
                TreeTarget::Category(planner.document.groups[0].categories[0].id)
            } else {
                TreeTarget::Group(0)
            };
            let original = name(&planner, target).to_owned();
            let render = |planner: &mut Planner| {
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(600.0, 900.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        egui::CentralPanel::default().show(ui, |ui| planner.sidebar(ui));
                    },
                );
                let text_positions: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if !text.galley.text().is_empty() => {
                            Some((text.galley.text().to_owned(), text.pos))
                        }
                        _ => None,
                    })
                    .collect();
                output.drop_without_applying_deltas();
                text_positions
            };
            render(&mut planner);
            let baseline = render(&mut planner);
            assert!(baseline.iter().any(|(text, _)| text == &original));
            planner.rename_draft = Some(RenameDraft::new(target, &original));
            for _ in 0..3 {
                let editing = render(&mut planner);
                for (text, pos) in &baseline {
                    assert_eq!(
                        editing
                            .iter()
                            .find_map(|(label, position)| (label == text).then_some(*position)),
                        Some(*pos),
                        "Text moved while renaming: {text}"
                    );
                }
            }
        }
    }
}

#[test]
fn inline_rename_saves_on_enter_and_cancels_on_escape() {
    for key in [Key::Enter, Key::Escape] {
        for category in [false, true] {
            let ctx = egui::Context::default();
            let mut planner = Planner::from_document(Document::default(), &ctx, None);
            let target = if category {
                TreeTarget::Category(planner.document.groups[0].categories[0].id)
            } else {
                TreeTarget::Group(0)
            };
            let original = name(&planner, target).to_owned();
            planner.rename_draft = Some(RenameDraft::new(target, &original));
            frame(&ctx, &mut planner, None);
            assert!(ctx.memory(|memory| memory.focused().is_some()));
            assert!(planner.tree_draft.is_none());
            assert!(ctx.top_layer_id().is_none());
            planner.rename_draft.as_mut().unwrap().text = "  New name  ".into();

            frame(&ctx, &mut planner, Some(key));

            assert!(planner.rename_draft.is_none());
            assert_eq!(
                name(&planner, target),
                if key == Key::Enter {
                    "New name"
                } else {
                    &original
                }
            );
            assert!(planner.tree_draft.is_none());
        }
    }
}

#[test]
fn empty_name_keeps_the_editor_open_and_the_original_name() {
    let ctx = egui::Context::default();
    let mut planner = Planner::from_document(Document::default(), &ctx, None);
    let target = TreeTarget::Group(0);
    let original = name(&planner, target).to_owned();
    planner.rename_draft = Some(RenameDraft::new(target, &original));
    frame(&ctx, &mut planner, None);
    planner.rename_draft.as_mut().unwrap().text = "   ".into();

    frame(&ctx, &mut planner, Some(Key::Enter));

    assert!(planner.rename_draft.is_some());
    assert_eq!(name(&planner, target), original);
    assert!(ctx.memory(|memory| memory.focused().is_some()));
    frame(&ctx, &mut planner, Some(Key::Escape));
    assert!(planner.rename_draft.is_none());
}

#[test]
fn created_names_are_focused_selected_and_replaced_by_typing() {
    use crate::app::sidebar::create::CreateTarget;

    for language in [Language::Russian, Language::English] {
        for category in [false, true] {
            let ctx = egui::Context::default();
            let document = Document {
                language,
                ..Document::default()
            };
            let mut planner = Planner::from_document(document, &ctx, None);
            planner.create_tree_entry(if category {
                CreateTarget::Category(0)
            } else {
                CreateTarget::Group
            });
            let target = planner.rename_draft.as_ref().unwrap().target;
            let original = name(&planner, target).to_owned();
            assert_eq!(
                original,
                if category {
                    language.text("Новый календарь", "New calendar")
                } else {
                    language.text("Новая группа", "New group")
                }
            );
            frame(&ctx, &mut planner, None);
            frame(&ctx, &mut planner, None);
            let id = ctx
                .memory(egui::Memory::focused)
                .expect("New name editor is focused");
            let state = egui::TextEdit::load_state(&ctx, id).unwrap();
            assert_eq!(
                state.cursor.char_range().unwrap().as_sorted_char_range(),
                egui::text::CharIndex::from(0)..original.chars().count().into()
            );
            assert!(planner.tree_draft.is_none());
            assert!(ctx.top_layer_id().is_none());

            frame_events(
                &ctx,
                &mut planner,
                vec![egui::Event::Text("Мой календарь".into())],
            );
            frame(&ctx, &mut planner, Some(Key::Enter));
            assert_eq!(name(&planner, target), "Мой календарь");
            assert!(planner.rename_draft.is_none());
        }
    }
}

#[test]
fn checkbox_labels_keep_theme_color_in_both_states() {
    for dark in [false, true] {
        let ctx = egui::Context::default();
        crate::app::appearance::apply_style(&ctx, dark);
        let expected = ctx
            .style_of(if dark {
                egui::Theme::Dark
            } else {
                egui::Theme::Light
            })
            .visuals
            .text_color();
        for mut enabled in [false, true] {
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    calendar_checkbox(ui, &mut enabled, RichText::new("Work").strong());
                });
            });
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == "Work" => Some(text),
                    _ => None,
                })
                .expect("Checkbox label is painted");
            assert!(
                text.galley
                    .job
                    .sections
                    .iter()
                    .all(|section| section.format.color == expected)
            );
            output.drop_without_applying_deltas();
        }
    }
}
