//! The text style manager's layout: list rows with their counts and lock,
//! "Styles in use", the two columns that stack in a narrow window, the
//! preview of the draft, and where the style is used (#2413).

use super::text_style_users::{TextKind, Usage};
use super::*;
use codec::entities::{AttributeDefinition, EntityType, MText, Text};
use codec::tables::{BlockRecord, TextStyle};
use codec::types::{Handle, Vector3};
use iced_test::Simulator;

/// A drawing with the styles "Notes" (height 2.5, used by two TEXTs, an
/// MTEXT and an attribute definition in a block), "Spare" (unused) and the
/// referenced "PLAN|Notes"; the manager open on "Notes".
fn manager() -> OpenCADStudio {
    let mut app = OpenCADStudio::new_for_test();
    app.automation_op(r#"{"op":"new"}"#);
    let doc = &mut app.tabs[app.active_tab].scene.document;
    for name in ["Notes", "Spare", "PLAN|Notes"] {
        let mut style = TextStyle::new(name);
        style.height = 2.5;
        style.handle = doc.allocate_handle();
        doc.text_styles.add(style).unwrap();
    }
    let mut block = BlockRecord::new("Title");
    block.handle = doc.allocate_handle();
    let owner = block.handle;
    doc.block_records.add(block).unwrap();
    for _ in 0..2 {
        let mut text = Text::with_value("A", Vector3::ZERO);
        text.style = "Notes".into();
        text.height = 2.5;
        doc.add_entity(EntityType::Text(text)).unwrap();
    }
    let mut mtext = MText::with_value("B", Vector3::ZERO);
    mtext.style = "notes".into();
    doc.add_entity(EntityType::MText(Box::new(mtext))).unwrap();
    let mut attdef = AttributeDefinition::default();
    attdef.text_style = "Notes".into();
    attdef.common.owner_handle = owner;
    let handle = doc
        .add_entity(EntityType::AttributeDefinition(attdef))
        .unwrap();
    add_to_block(&mut app, owner, handle);
    let _ = app.update(Message::TextStyleDialogOpen);
    let _ = app.update(Message::TextStyleDialogSelect("Notes".into()));
    app
}

fn add_to_block(app: &mut OpenCADStudio, block: Handle, entity: Handle) {
    let doc = &mut app.tabs[app.active_tab].scene.document;
    if let Some(record) = doc.block_records.iter_mut().find(|r| r.handle == block) {
        record.entity_handles.push(entity);
    }
}

fn window(app: &OpenCADStudio, width: f32) -> Simulator<'_, Message> {
    Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(width, 760.0),
        app.view_main(),
    )
}

fn shown(simulator: &mut Simulator<'_, Message>, text: &str) -> iced::Rectangle {
    simulator
        .find(text.to_string())
        .unwrap_or_else(|_| panic!("{text:?} is not shown"))
        .visible_bounds()
        .unwrap_or_else(|| panic!("{text:?} is not visible"))
}

#[test]
fn the_counts_come_from_one_walk_and_cover_every_kind() {
    let app = manager();
    assert_eq!(app.textstyle_counts.get("notes"), Some(&4));
    assert_eq!(app.textstyle_counts.get("spare"), None);
    assert_eq!(
        Usage::of(&app.textstyle_users),
        Usage {
            kinds: vec![
                (TextKind::Text, 2),
                (TextKind::MText, 1),
                (TextKind::AttributeDefinition, 1),
            ],
            in_blocks: 1,
            blocks: 1,
        }
    );
}

#[test]
fn the_list_shows_counts_and_styles_in_use_hides_the_unused() {
    let mut app = manager();
    let mut ui = window(&app, 1280.0);
    shown(&mut ui, "Spare");
    shown(&mut ui, "PLAN|Notes");
    shown(&mut ui, "4");
    drop(ui);
    let _ = app.update(Message::TextStyleInUseFilter(true));
    let mut ui = window(&app, 1280.0);
    assert!(ui.find("Spare".to_string()).is_err());
    shown(&mut ui, "Standard");
}

#[test]
fn the_where_used_box_and_the_status_count_the_texts() {
    let app = manager();
    let mut ui = window(&app, 1280.0);
    for text in [
        "Single-line text",
        "MTEXT",
        "Attribute definitions",
        "1 of them inside 1 block definition.",
        "Used in drawing · used by 4 texts",
    ] {
        shown(&mut ui, text);
    }
}

#[test]
fn a_changed_value_says_what_was_stored_and_apply_says_what_it_does() {
    let mut app = manager();
    let _ = app.update(Message::TextStyleEdit {
        field: "height",
        value: "3.5".into(),
    });
    let mut ui = window(&app, 1280.0);
    // As the field showed it.
    shown(&mut ui, "Modified · was 2.5000");
    shown(&mut ui, "On Apply");
}

#[test]
fn the_columns_sit_side_by_side_and_stack_in_a_narrow_window() {
    let mut app = manager();
    let _ = app.update(Message::TextStyleEdit {
        field: "height",
        value: "3.5".into(),
    });
    let mut wide = window(&app, 1280.0);
    let font = shown(&mut wide, "Font style");
    let preview = shown(&mut wide, "Sample text");
    assert!(preview.x > font.x + font.width, "{font:?} {preview:?}");
    drop(wide);

    // The modal keeps its size; a narrow window makes it narrow.
    let mut narrow = window(&app, 780.0);
    let summary = shown(&mut narrow, "On Apply");
    let font = shown(&mut narrow, "Font style");
    assert!(summary.y < font.y, "{summary:?} {font:?}");
    drop(narrow);

    // Narrower still, the toolbar shows icons only, the names as hints.
    let mut compact = window(&app, 600.0);
    assert!(compact.find("Set Current".to_string()).is_err());
    shown(&mut compact, "On Apply");
}

#[test]
fn the_preview_draws_the_draft_with_its_sample_text() {
    let mut app = manager();
    let _ = app.update(Message::TextStyleSample("Hello".into()));
    let _ = app.update(Message::TextStyleEdit {
        field: "width",
        value: "2".into(),
    });
    let mut ui = window(&app, 1280.0);
    shown(&mut ui, "Hello");
    shown(&mut ui, "Where used");
}
