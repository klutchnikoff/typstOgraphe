//! tests : détail interne de l’adaptateur egui.

use super::*;

#[test]
fn empty_and_wrapped_list_items_keep_the_caret_at_the_text_indent() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    for source in [
        "- Premier élément suffisamment long pour se replier dans une colonne étroite\n- ",
        "+ Premier élément suffisamment long pour se replier dans une colonne étroite\n+ ",
        "/ Terme: Définition\n/ : ",
    ] {
        let document = Document::from_source(source).unwrap();
        for width in [160.0, 600.0] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let editor = EditorStyle::default();
                    let galley = layout(ui, &document.view, &document.view.text, width, editor);
                    assert_eq!(galley.text(), document.view.text);
                    for row in &galley.rows {
                        if let Some(first) = row.glyphs.first() {
                            assert!(row.pos.x + first.pos.x >= editor.size * 22.0 / 18.0 - 1.0);
                        }
                    }
                    for range in document.view.line_ranges() {
                        let at = CCursor::new(range.start);
                        let position = galley.pos_from_cursor(at);
                        assert!(
                            position.left() >= editor.size * 22.0 / 18.0 - 1.0,
                            "{source:?}: cursor {at:?} at {position:?}"
                        );
                        assert_eq!(
                            galley.cursor_from_pos(position.center().to_vec2()).index,
                            range.start
                        );
                    }
                });
            });
        }
    }
}
#[test]
fn search_backgrounds_cover_all_results_without_changing_layout_in_either_theme() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    let document = crate::document::Document::from_source("= Été\n\nUn *été* qui revient dans une longue phrase.\n\n/ été: Un texte.\n\n#quote(block: true, attribution: [été])[Un été.]\n\n#figure(image(\"images/parc.svg\"), caption: [été])").unwrap();
    let before = document.clone();
    let mut search = crate::search::Search::default();
    search.query = "été".into();
    search.refresh(1, &document.view);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_theme(theme);
        for width in [160.0, 600.0] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let normal = layout(
                        ui,
                        &document.view,
                        &document.view.text,
                        width,
                        EditorStyle::default(),
                    );
                    let highlighted = layout_with_search(
                        ui,
                        &document.view,
                        &document.view.text,
                        width,
                        EditorStyle::default(),
                        &search.matches,
                        Some(0),
                    );
                    assert_eq!(normal.size(), highlighted.size());
                    for at in 0..=document.view.len() {
                        assert_eq!(
                            normal.pos_from_cursor(CCursor::new(at)),
                            highlighted.pos_from_cursor(CCursor::new(at))
                        );
                    }
                    for section in &highlighted.job.sections {
                        let found = search.matches.iter().any(|range| {
                            let bytes = document.view.byte_index(range.start)
                                ..document.view.byte_index(range.end);
                            bytes.contains(&section.byte_range.start)
                        });
                        assert_eq!(section.format.background != Color32::TRANSPARENT, found);
                    }
                    let rectangles = range_rects(&highlighted, &search.matches);
                    for index in 0..search.matches.len() {
                        assert!(
                            rectangles
                                .iter()
                                .any(|(found, rect)| *found == index && rect.width() > 0.0)
                        );
                    }
                });
            });
        }
    }
    assert_eq!(document, before);
}
use crate::document::Document;

#[test]
fn paragraph_marker_is_visual_only_and_keeps_native_blank_line_cursors() {
    fn markers(shape: &egui::Shape) -> usize {
        match shape {
            egui::Shape::Text(text) => usize::from(text.galley.text() == "¶"),
            egui::Shape::Vec(shapes) => shapes.iter().map(markers).sum(),
            _ => 0,
        }
    }
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_theme(theme);
        let mut document = Document::from_source("Un\nDeux\n\nTrois").unwrap();
        let original = document.clone();
        let editor = EditorStyle::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let view = document.view.clone();
                let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, width| {
                    layout(ui, &view, text.as_str(), width, editor)
                };
                let background = ui.painter().add(egui::Shape::Noop);
                let output = egui::TextEdit::multiline(&mut ProjectionBuffer(&mut document.view))
                    .layouter(&mut layouter)
                    .show(ui);
                paint_decorations(ui, &document.view, &output, editor, background);
                assert_eq!(output.galley.job.text, "Un\nDeux\n\nTrois");
                let mut cursor = CCursor::new(8);
                cursor.prefer_next_row = true;
                let blank = output.galley.pos_from_cursor(cursor);
                let next = output.galley.pos_from_cursor(CCursor::new(9));
                assert!((next.top() - blank.top() - editor.size * 1.55).abs() < 1.0);
                assert_eq!(
                    output
                        .galley
                        .cursor_from_pos((blank.center() + egui::vec2(8.0, 0.0)).to_vec2())
                        .index,
                    8
                );
            });
        });
        assert_eq!(
            output
                .shapes
                .iter()
                .map(|shape| markers(&shape.shape))
                .sum::<usize>(),
            1
        );
        assert_eq!(document, original);
    }
}

#[test]
fn image_layout_keeps_native_indices_wrapping_and_caption_inside_the_frame() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    for caption in [
        ", caption: [Une longue légende à lire et à sélectionner dans son intégralité.]",
        "",
    ] {
        let source = format!("= Visite\n\n#figure(image(\"images/parc.svg\"){caption})\n\nSuite.");
        let document = Document::from_source(&source).unwrap();
        let range = document.view.image_range(1).unwrap();
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            for width in [180.0, 500.0] {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let galley = layout(
                            ui,
                            &document.view,
                            &document.view.text,
                            width,
                            EditorStyle::default(),
                        );
                        assert_eq!(
                            galley
                                .rows
                                .iter()
                                .map(|r| r.glyphs.len() + usize::from(r.ends_with_newline))
                                .sum::<usize>(),
                            document.view.len()
                        );
                        let mut caption = CCursor::new(range.start + 1);
                        caption.prefer_next_row = true;
                        let position = galley.pos_from_cursor(galley.clamp_cursor(&caption));
                        let frame = image_frame(&galley, range.clone());
                        assert!(position.top() >= frame.top() + IMAGE_HEIGHT);
                        assert!(position.height() < EditorStyle::default().size * 2.0);
                        let last = galley.pos_from_cursor(if range.len() == 1 {
                            caption
                        } else {
                            CCursor::new(range.end)
                        });
                        assert!(frame.contains(position.center()));
                        assert!(frame.contains(last.center()));
                        let next = galley.pos_from_cursor(CCursor::new(range.end + 1));
                        assert!(next.top() >= frame.bottom());
                        for index in range.start + 2..range.end {
                            let cursor = CCursor::new(index);
                            let rect = galley.pos_from_cursor(cursor);
                            assert_eq!(
                                galley.cursor_from_pos(rect.center().to_vec2()).index,
                                index
                            );
                        }
                    });
                });
            }
        }
        assert_eq!(document.source(), source);
    }
}

#[test]
fn quote_wrapping_keeps_the_indent_and_native_cursors_for_body_and_attribution() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    let mut document = Document::from_source("Avant.\n\n#quote(block: true, attribution: [Service des équipements, _rapport_])[Une citation assez longue pour occuper plusieurs lignes dans une colonne étroite, avec du *gras*.\n\nUn second paragraphe.]\n\n#quote(block: true)[Sans attribution.]\n\nAprès.").unwrap();
    let original = document.clone();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_theme(theme);
        for width in [180.0, 500.0] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let view = document.view.clone();
                    let editor = EditorStyle::default();
                    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, _| {
                        layout(ui, &view, text.as_str(), width, editor)
                    };
                    let background = ui.painter().add(egui::Shape::Noop);
                    let output =
                        egui::TextEdit::multiline(&mut ProjectionBuffer(&mut document.view))
                            .desired_width(width)
                            .layouter(&mut layouter)
                            .show(ui);
                    assert_eq!(output.galley.text(), view.text);
                    let mut line = 0;
                    for row in &output.galley.rows {
                        if matches!(
                            view.blocks[line],
                            BlockKind::Quote | BlockKind::QuoteAttribution
                        ) {
                            assert!((row.x_offset(0) - editor.size * 1.2).abs() <= 1.0);
                            assert!(row.rect().right() <= width + 1.0);
                        } else {
                            assert_eq!(row.pos.x, 0.0);
                        }
                        line += usize::from(row.ends_with_newline);
                    }
                    for (index, c) in view.text.chars().enumerate() {
                        if c.is_whitespace() {
                            continue;
                        }
                        let rect = output.galley.pos_from_cursor(CCursor::new(index));
                        assert_eq!(
                            output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                            index,
                            "{c} at {index}"
                        );
                    }
                    paint_decorations(ui, &document.view, &output, editor, background);
                });
            });
        }
    }
    assert_eq!(document, original);
}

#[test]
fn description_boxes_and_placeholders_keep_native_cursor_indices_and_source() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    let mut document = Document::from_source(
        "/ Échéance: Le vendredi.\n/ : Un texte dont le terme est vide.\n/ : ",
    )
    .unwrap();
    let original = document.clone();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_theme(theme);
        for width in [200.0, 500.0] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let view = document.view.clone();
                    let editor = EditorStyle::default();
                    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, _| {
                        layout(ui, &view, text.as_str(), width, editor)
                    };
                    let background = ui.painter().add(egui::Shape::Noop);
                    let output =
                        egui::TextEdit::multiline(&mut ProjectionBuffer(&mut document.view))
                            .desired_width(width)
                            .layouter(&mut layouter)
                            .show(ui);
                    let placeholder = description_placeholder(ui, editor);
                    let boxes = description_boxes(
                        &document.view,
                        &output.galley,
                        placeholder.size().x,
                        editor.size,
                    );
                    assert_eq!(
                        boxes
                            .iter()
                            .filter(|label| label.placeholder.is_some())
                            .count(),
                        2
                    );
                    assert_eq!(output.galley.text(), document.view.text);
                    for label in boxes.iter().filter(|label| label.placeholder.is_some()) {
                        let before = output.galley.pos_from_cursor(CCursor::new(label.start));
                        let after = output.galley.pos_from_cursor(CCursor::new(label.start + 1));
                        assert!(before.left() <= label.rect.left() + editor.size * 0.3);
                        if (before.top() - after.top()).abs() < 0.1 {
                            assert!(after.left() - before.left() > placeholder.size().x);
                        }
                    }
                    for (index, c) in document.view.text.chars().enumerate() {
                        if c.is_whitespace() {
                            continue;
                        }
                        let rect = output.galley.pos_from_cursor(CCursor::new(index));
                        assert_eq!(
                            output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                            index
                        );
                    }
                    paint_decorations(ui, &document.view, &output, editor, background);
                });
            });
        }
    }
    assert_eq!(document, original);
    document.sync_source();
    assert!(!document.source().contains("Description"));
    assert_eq!(
        Document::from_source(document.source()).unwrap().view,
        document.view
    );
}

fn format_at<'a>(galley: &'a Galley, text: &str) -> &'a TextFormat {
    let byte = galley.job.text.find(text).unwrap();
    &galley
        .job
        .sections
        .iter()
        .find(|s| s.byte_range.contains(&byte))
        .unwrap()
        .format
}

#[test]
fn image_captions_share_the_quote_attribution_style_without_altering_source() {
    let source = "#figure(image(\"images/parc.svg\"), caption: [Légende.])\n\n#quote(block: true, attribution: [Attribution.])[Citation.]";
    let document = Document::from_source(source).unwrap();
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_theme(theme);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for font in EditorFont::ALL {
                    let editor = EditorStyle { font, size: 20.0 };
                    let galley = layout(ui, &document.view, &document.view.text, 500.0, editor);
                    let caption = format_at(&galley, "Légende");
                    let attribution = format_at(&galley, "Attribution");
                    assert_eq!(caption.font_id, attribution.font_id);
                    assert_eq!(caption.color, attribution.color);
                    assert_eq!(caption.font_id, font.font_id(18.0, 2));
                }
            });
        });
    }
    assert_eq!(document.source(), source);
}

#[test]
fn semantic_styles_follow_the_theme_and_editor_size_without_changing_typst() {
    let document = Document::from_source("= Premier titre\n\nUn *gras*, une _italique_, #strong[#emph[combinaison]] et #link(\"https://example.org\")[*lien*].\n\n== Deuxième titre\n\n=== Troisième titre\n").unwrap();
    let original = document.clone();
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_theme(theme);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for font in EditorFont::ALL {
                    for size in [12.0, 24.0] {
                        let galley = layout(
                            ui,
                            &document.view,
                            &document.view.text,
                            500.0,
                            EditorStyle { font, size },
                        );
                        let bold = format_at(&galley, "gras");
                        let italic = format_at(&galley, "italique");
                        let combined = format_at(&galley, "combinaison");
                        let h1 = format_at(&galley, "Premier titre");
                        let h2 = format_at(&galley, "Deuxième titre");
                        let h3 = format_at(&galley, "Troisième titre");
                        let link = format_at(&galley, "lien");
                        assert!(bold.color.r() > bold.color.g());
                        assert!(italic.color.g() > italic.color.r());
                        assert_eq!(combined.color, bold.color);
                        assert_eq!(combined.font_id, font.font_id(size, 3));
                        assert_eq!(link.color, h1.color);
                        assert!(!link_underlines(&document.view, &galley).is_empty());
                        assert_eq!(h1.color, h2.color);
                        assert_eq!(h2.color, h3.color);
                        for (format, factor) in [(h1, 1.728), (h2, 1.44), (h3, 1.2)] {
                            assert!((format.font_id.size - factor * size).abs() < 0.001);
                        }
                        assert_eq!(h3.font_id.family, font.font_id(size, 2).family);
                        assert_eq!(galley.job.text, document.view.text);
                    }
                }
            });
        });
    }
    assert_eq!(document, original);
    let h3_start = document.view.text[..document.view.text.find("Troisième").unwrap()]
        .chars()
        .count();
    assert!(document.view.styles[h3_start..].iter().all(|s| !s.italic));
}

#[test]
fn wrapped_heading_rule_and_spacing_preserve_cursor_positions_and_text() {
    let mut document = Document::from_source("= Un titre suffisamment long pour occuper plusieurs lignes dans une colonne étroite\n\nParagraphe après le titre.\n\n== Deuxième section\n\nTexte suivant.").unwrap();
    let original = document.clone();
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    let editor = EditorStyle::default();
    let mut expected_rule = None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 1100.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let view = document.view.clone();
                let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, width| {
                    layout(ui, &view, text.as_str(), width, editor)
                };
                let background = ui.painter().add(egui::Shape::Noop);
                let output = egui::TextEdit::multiline(&mut ProjectionBuffer(&mut document.view))
                    .desired_width(280.0)
                    .layouter(&mut layouter)
                    .show(ui);
                let ranges = document.view.line_ranges();
                let end = output
                    .galley
                    .layout_from_cursor(CCursor::new(ranges[0].end));
                assert!(
                    end.row > 0,
                    "le titre doit se répartir sur plusieurs lignes"
                );
                let title_bottom = output
                    .galley
                    .pos_from_cursor(CCursor::new(ranges[0].end))
                    .bottom();
                let body_top = output
                    .galley
                    .pos_from_cursor(CCursor::new(ranges[1].start))
                    .top();
                assert!(body_top - title_bottom > editor.size * 0.4);
                for word in [
                    "suffisamment",
                    "étroite",
                    "Paragraphe",
                    "Deuxième",
                    "suivant",
                ] {
                    let byte = document.view.text.find(word).unwrap();
                    let index = document.view.text[..byte].chars().count() + 2;
                    let rect = output.galley.pos_from_cursor(CCursor::new(index));
                    assert_eq!(
                        output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                        index
                    );
                }
                expected_rule = Some((
                    output.galley_pos.x,
                    output
                        .response
                        .rect
                        .right()
                        .min(output.text_clip_rect.right()),
                    output.galley_pos.y + title_bottom + editor.size * 0.12,
                ));
                paint_decorations(ui, &document.view, &output, editor, background);
                // Le cache brut ne doit pas accumuler les espacements du rendu.
                let repeated = layout(ui, &document.view, &document.view.text, 280.0, editor);
                let again = layout(ui, &document.view, &document.view.text, 280.0, editor);
                assert_eq!(repeated.rect, again.rect);
            });
        },
    );
    let (left, right, y) = expected_rule.unwrap();
    let rules: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::LineSegment { points, .. }
                if (points[0].y - y).abs() < 0.01 && (points[1].y - y).abs() < 0.01 =>
            {
                Some(points)
            }
            _ => None,
        })
        .collect();
    assert_eq!(rules.len(), 1);
    assert!((rules[0][0].x - left).abs() < 0.01);
    assert!((rules[0][1].x - right).abs() < 0.01);
    assert_eq!(document, original);
}
#[test]
fn letter_note_frames_keep_wrapped_and_empty_text_native_cursors_in_both_themes() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    for source in [
        "#postscript[]\n#attachment[]",
        "#postscript[Une information supplémentaire assez longue pour se replier sur plusieurs lignes, avec du *gras*.]\n\n#attachment[Un formulaire signé assez long pour se replier également]\n#attachment[]",
    ] {
        let mut document = Document::from_source(source).unwrap();
        let original = document.clone();
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            for width in [180.0, 500.0] {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let view = document.view.clone();
                        let editor = EditorStyle::default();
                        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, _| {
                            layout(ui, &view, text.as_str(), width, editor)
                        };
                        let background = ui.painter().add(egui::Shape::Noop);
                        let output =
                            egui::TextEdit::multiline(&mut ProjectionBuffer(&mut document.view))
                                .layouter(&mut layouter)
                                .show(ui);
                        for (line, range) in view.line_ranges().iter().enumerate() {
                            if view.blocks[line].is_letter_note() {
                                let mut cursor = CCursor::new(range.start);
                                cursor.prefer_next_row = true;
                                let rect = output.galley.pos_from_cursor(cursor);
                                assert!(
                                    rect.left() >= editor.size * 4.0 - 1.0,
                                    "{range:?}: {rect:?}"
                                );
                            }
                        }
                        for (index, c) in view.text.chars().enumerate() {
                            if !c.is_whitespace() {
                                let rect = output.galley.pos_from_cursor(CCursor::new(index));
                                assert_eq!(
                                    output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                                    index
                                );
                            }
                        }
                        paint_decorations(ui, &view, &output, editor, background);
                    });
                });
            }
        }
        assert_eq!(document, original);
    }
}

#[test]
fn cursor_adapters_preserve_direction_and_line_affinity() {
    let range = CCursorRange {
        primary: CCursor {
            index: 1,
            prefer_next_row: true,
        },
        secondary: CCursor {
            index: 9,
            prefer_next_row: false,
        },
        h_pos: None,
    };
    assert_eq!(to_editor(to_core(range)), range);
    assert_eq!(to_core(range).range(), 1..9);
}

#[test]
fn native_buffer_and_ime_reset_cannot_erase_a_protected_boundary_error() {
    let original = crate::document::Document::from_source("/ Terme: Définition").unwrap();
    let mut input = crate::document::edits::TextInput {
        view: original.view.clone(),
    };
    let mut buffer = InputBuffer(&mut input);
    buffer.delete_char_range(0..7);
    assert_eq!(buffer.insert_text("Invalide", 0), 0);
    assert!(input.view.edit_error().is_some());
    let rejected = input.view.clone();
    input.reset_projection(original.view.clone());
    assert_eq!(input.insert_text("Valide", 0), 0);
    assert!(input.view.validate_edit().is_err());
    let mut healthy = crate::document::edits::TextInput {
        view: original.view.clone(),
    };
    healthy.reset_projection(rejected);
    assert!(healthy.view.validate_edit().is_err());
    // Abandonner le brouillon suffit : la projection publiée reste intacte.
    assert!(original.view.validate_edit().is_ok());
}

#[test]
fn semantic_editor_colors_stay_readable_on_the_interface_surfaces() {
    use crate::theme::{Tokens, contrast_ratio};
    for dark in [false, true] {
        let visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let colors = layout::EditorColors::from_visuals(&visuals);
        let tokens = Tokens::new(dark);
        for (name, color) in [
            ("accent", colors.accent),
            ("gras", colors.bold),
            ("italique", colors.italic),
        ] {
            for (surface, ground) in [("fond", tokens.bg), ("panneau", tokens.panel)] {
                let ratio = contrast_ratio(color, ground);
                assert!(
                    ratio >= 4.5,
                    "dark={dark} : {name} sur {surface} = {ratio:.2}"
                );
            }
        }
    }
}
