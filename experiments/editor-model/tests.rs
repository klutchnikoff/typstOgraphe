use super::*;
use typstographe::{document_types, rendering};

#[test]
fn model_spike_navigation_is_unicode_and_does_not_change_the_document() {
    let mut table = Table::default();
    let original = table.state.view.clone();
    table.navigate(false);
    assert_eq!(table.state.selection.as_sorted_char_range(), 4..12);
    table.navigate(false);
    assert_eq!(table.state.selection.as_sorted_char_range(), 13..16);
    table.navigate(true);
    assert_eq!(table.state.selection.as_sorted_char_range(), 4..12);
    assert_eq!(table.state.view, original);
    assert!(table.undo.is_empty());
}

#[test]
fn model_spike_rows_columns_and_selection_are_undoable() {
    let mut table = Table::default();
    let original = table.state.view.clone();
    for row in [true, false] {
        table.reshape(row, true);
        assert!(table.valid());
        table.history(true);
        assert_eq!(table.state.view, original);
        assert_eq!(table.state.columns, 2);
        table.history(false);
        table.reshape(row, false);
        assert!(table.valid());
        table.history(true);
        table.history(true);
        assert_eq!(table.state.view, original);
    }
    let source = table.source().unwrap();
    assert!(
        !typst_syntax::Source::detached(source.clone())
            .root()
            .diagnosis()
            .errors
    );
    let mut world = rendering::TypstWorld::default();
    let composed = world
        .compile_with_template(
            &source,
            &document_types::Metadata::demo(document_types::DocumentType::Courrier),
            &document_types::models::Model::Personal.source(),
        )
        .unwrap();
    assert!(!composed.pages().is_empty());
    assert!(
        document::Document::from_source(&source).is_err(),
        "Le parseur de production n’accepte pas encore table."
    );
}

#[test]
fn model_spike_native_cross_cell_replacement_breaks_rectangularity() {
    let mut table = Table::default();
    let before = table.state.clone();
    let ctx = egui::Context::default();
    editor::install_fonts(&ctx);
    table.state.selection = editor::cursor(1..6);
    input_frame(&mut table, &ctx, vec![]);
    input_frame(&mut table, &ctx, vec![egui::Event::Text("X".into())]);
    assert!(!table.valid());
    assert!(table.source().is_none());
    table.history(true);
    assert_eq!(table.state.view, before.view);
}

fn input_frame(table: &mut Table, ctx: &egui::Context, events: Vec<egui::Event>) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| table.editor_ui(ui));
        },
    );
}

#[test]
fn model_spike_egui_accepts_accented_text_and_ime_composition_in_a_cell() {
    let mut table = Table::default();
    let ctx = egui::Context::default();
    editor::install_fonts(&ctx);
    table.state.selection = editor::cursor(13..16); // Zoé
    input_frame(&mut table, &ctx, vec![]);
    input_frame(&mut table, &ctx, vec![egui::Event::Text("Élodie".into())]);
    assert!(table.state.view.text.contains("Élodie\tOui"));
    table.history(true);
    input_frame(&mut table, &ctx, vec![]);
    input_frame(
        &mut table,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Enabled)],
    );
    input_frame(
        &mut table,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into()))],
    );
    input_frame(
        &mut table,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Commit("日本".into()))],
    );
    assert!(
        table.state.view.text.contains("日本\tOui"),
        "{}",
        table.state.view.text
    );
    assert!(table.valid());
}

#[test]
fn model_spike_paragraph_normalization_can_remove_empty_single_column_rows() {
    let mut view = Projection::default();
    let inserted = view.insert_text("\n\n\n", 0);
    assert_eq!(
        inserted, 2,
        "Le tampon applique les règles de paragraphe du corps."
    );
    assert_eq!(view.blocks.len(), 3);
}

#[test]
fn model_spike_tab_moves_the_native_selection_without_creating_an_undo_step() {
    let mut table = Table::default();
    let ctx = egui::Context::default();
    editor::install_fonts(&ctx);
    input_frame(&mut table, &ctx, vec![]);
    input_frame(
        &mut table,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(table.state.selection.as_sorted_char_range(), 4..12);
    assert!(table.undo.is_empty());
}
