//! Text style manager engine: the draft keeps untouched values exact, list
//! operations keep typed values, and Close / Undo bring back the texts a
//! rename touched (#2413).

use super::*;
use codec::entities::{EntityType, MText, Text};
use codec::tables::TextStyle;
use codec::types::{Handle, Vector3};

fn exact_style(name: &str) -> TextStyle {
    let mut style = TextStyle::new(name);
    style.width_factor = 0.123456789;
    style.oblique_angle = 0.1;
    style.height = 2.123456789;
    style
}

/// A clean drawing with the style "Exact", a TEXT and an MTEXT using it.
fn app_with_exact_style() -> (OpenCADStudio, Handle, Handle) {
    let mut app = OpenCADStudio::new_for_test();
    app.automation_op(r#"{"op":"new"}"#);
    let i = app.active_tab;
    let doc = &mut app.tabs[i].scene.document;
    let mut style = exact_style("Exact");
    style.handle = doc.allocate_handle();
    doc.text_styles.add(style).unwrap();
    let mut text = Text::with_value("A", Vector3::ZERO);
    text.style = "Exact".to_string();
    let text = doc.add_entity(EntityType::Text(text)).unwrap();
    let mut mtext = MText::with_value("B", Vector3::ZERO);
    mtext.style = "Exact".to_string();
    let mtext = doc.add_entity(EntityType::MText(Box::new(mtext))).unwrap();
    app.tabs[i].dirty = false;
    (app, text, mtext)
}

fn stored(app: &OpenCADStudio, name: &str) -> Option<TextStyle> {
    app.tabs[app.active_tab]
        .scene
        .document
        .text_styles
        .get(name)
        .cloned()
}

fn style_of(app: &OpenCADStudio, handle: Handle) -> String {
    match app.tabs[app.active_tab].scene.document.get_entity(handle) {
        Some(EntityType::Text(text)) => text.style.clone(),
        Some(EntityType::MText(text)) => text.style.clone(),
        other => panic!("not a text: {other:?}"),
    }
}

fn open_on(app: &mut OpenCADStudio, name: &str) {
    let _ = app.update(Message::TextStyleDialogOpen);
    let _ = app.update(Message::TextStyleDialogSelect(name.to_string()));
}

fn rename(app: &mut OpenCADStudio, old: &str, new: &str) {
    let _ = app.update(Message::StyleRenameStart(StyleKind::Text, old.to_string()));
    let _ = app.update(Message::StyleRenameEdit(new.to_string()));
    let _ = app.update(Message::StyleRenameCommit(StyleKind::Text));
}

#[test]
fn open_apply_switch_and_close_keep_the_style_exact() {
    let (mut app, _, _) = app_with_exact_style();
    let i = app.active_tab;
    let depth = app.tabs[i].history.undo_stack.len();

    open_on(&mut app, "Exact");
    let _ = app.update(Message::TextStyleApply);
    let _ = app.update(Message::TextStyleDialogSelect("Standard".to_string()));
    let _ = app.update(Message::TextStyleDialogSelect("Exact".to_string()));
    let _ = app.update(Message::TextStyleApply);
    app.close_active_modal();

    let mut expected = exact_style("Exact");
    expected.handle = stored(&app, "Exact").unwrap().handle;
    assert_eq!(stored(&app, "Exact"), Some(expected));
    assert!(!app.tabs[i].dirty, "an untouched Apply must not dirty the tab");
    assert_eq!(app.tabs[i].history.undo_stack.len(), depth);
}

#[test]
fn a_changed_field_is_written_and_the_rest_stays_exact() {
    let (mut app, _, _) = app_with_exact_style();
    open_on(&mut app, "Exact");
    let _ = app.update(Message::TextStyleEdit {
        field: "height",
        value: "3.5".to_string(),
    });
    let _ = app.update(Message::TextStyleApply);

    let style = stored(&app, "Exact").unwrap();
    assert_eq!(style.height, 3.5);
    assert_eq!(style.width_factor, 0.123456789);
    assert_eq!(style.oblique_angle, 0.1);
    assert!(app.tabs[app.active_tab].dirty);
}

#[test]
fn rename_then_close_restores_the_texts() {
    let (mut app, text, mtext) = app_with_exact_style();
    open_on(&mut app, "Exact");
    rename(&mut app, "Exact", "Notes");
    assert_eq!(style_of(&app, text), "Notes");

    app.close_active_modal();

    assert!(stored(&app, "Exact").is_some());
    assert!(stored(&app, "Notes").is_none());
    assert_eq!(style_of(&app, text), "Exact");
    assert_eq!(style_of(&app, mtext), "Exact");
    assert!(!app.tabs[app.active_tab].dirty);
}

#[test]
fn rename_apply_then_one_undo_restores_the_texts() {
    let (mut app, text, mtext) = app_with_exact_style();
    open_on(&mut app, "Exact");
    rename(&mut app, "Exact", "Notes");
    let _ = app.update(Message::TextStyleApply);
    app.close_active_modal();
    assert_eq!(style_of(&app, mtext), "Notes");

    let _ = app.update(Message::Undo);

    assert!(stored(&app, "Exact").is_some());
    assert!(stored(&app, "Notes").is_none());
    assert_eq!(style_of(&app, text), "Exact");
    assert_eq!(style_of(&app, mtext), "Exact");

    let _ = app.update(Message::Redo);
    assert_eq!(style_of(&app, text), "Notes");
}

#[test]
fn copy_takes_the_value_typed_before_it() {
    let (mut app, _, _) = app_with_exact_style();
    open_on(&mut app, "Exact");
    let _ = app.update(Message::TextStyleEdit {
        field: "width",
        value: "0.8".to_string(),
    });
    let _ = app.update(Message::TextStyleDialogCopy);

    let copy = stored(&app, "Exact (1)").unwrap();
    assert_eq!(copy.width_factor, 0.8);
    assert_eq!(copy.height, 2.123456789);
    assert_eq!(stored(&app, "Exact").unwrap().width_factor, 0.8);
}

#[test]
fn rename_start_keeps_the_typed_value() {
    let (mut app, _, _) = app_with_exact_style();
    open_on(&mut app, "Exact");
    let _ = app.update(Message::TextStyleEdit {
        field: "height",
        value: "5".to_string(),
    });
    let _ = app.update(Message::StyleRenameStart(StyleKind::Text, "Exact".to_string()));
    assert_eq!(app.textstyle_draft.height.text, "5.0000");
}

#[test]
fn merged_xref_text_style_is_read_only() {
    let (mut app, _, _) = app_with_exact_style();
    let i = app.active_tab;
    app.tabs[i]
        .scene
        .document
        .text_styles
        .add(exact_style("Plan|Notes"))
        .unwrap();
    let before = stored(&app, "Plan|Notes");

    open_on(&mut app, "Plan|Notes");
    assert!(app.selected_textstyle_is_read_only());
    let _ = app.update(Message::TextStyleEdit {
        field: "height",
        value: "9".to_string(),
    });
    let _ = app.update(Message::TextStyleToggle("backward"));
    let _ = app.update(Message::TextStyleDialogSetCurrent);
    let _ = app.update(Message::TextStyleDialogDelete);
    rename(&mut app, "Plan|Notes", "Mine");
    let _ = app.update(Message::TextStyleApply);

    assert_eq!(stored(&app, "Plan|Notes"), before);
    assert!(stored(&app, "Mine").is_none());
    assert_ne!(
        app.tabs[i].scene.document.header.current_text_style_name,
        "Plan|Notes"
    );
}
