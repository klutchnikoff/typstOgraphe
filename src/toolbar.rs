//! Présentation de la barre : les actions retournent les commandes du registre.
use crate::{
    commands::{Command, Keymap},
    document::{BlockKind, Mark, Projection},
    icons::Icon,
};
use eframe::egui::{self, Color32, Response, Stroke, WidgetInfo, WidgetType};
use std::ops::Range;

pub const ICON_SIZE: f32 = 18.0;

pub fn neutral_selection(ui: &mut egui::Ui) {
    let ink = ui.visuals().text_color();
    let dark = ui.visuals().dark_mode;
    ui.visuals_mut().selection.bg_fill = Color32::from_gray(if dark { 64 } else { 224 });
    ui.visuals_mut().selection.stroke = Stroke::new(1.0_f32, ink);
}

fn button(ui: &egui::Ui, icon: Icon, active: bool, menu: bool) -> egui::Button<'static> {
    let image = icon.image(ui.ctx(), ICON_SIZE);
    let button = if menu {
        egui::Button::new((image, Icon::ChevronDown.image(ui.ctx(), 12.0)))
    } else {
        egui::Button::new(image)
    };
    button
        .image_tint_follows_text_color(true)
        .min_size(egui::vec2(if menu { 48.0 } else { 32.0 }, 32.0))
        .frame_when_inactive(false)
        .selected(active)
}

fn menu_item(ui: &mut egui::Ui, kind: BlockKind, active: bool, label: String) -> Response {
    neutral_selection(ui);
    let image = Icon::for_block(kind).image(ui.ctx(), ICON_SIZE);
    ui.add(egui::Button::selectable(active, (image, label)).image_tint_follows_text_color(true))
}

fn describe(
    response: Response,
    label: String,
    hint: String,
    active: bool,
    mixed: bool,
) -> Response {
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, active, &label));
    if mixed {
        let rect = egui::Rect::from_center_size(
            response.rect.right_top() + egui::vec2(-5.0, 5.0),
            egui::vec2(5.0, 2.0),
        );
        response.ctx.layer_painter(response.layer_id).rect_filled(
            rect,
            1.0,
            response.ctx.style().visuals.text_color(),
        );
    }
    response.on_hover_text(if mixed {
        crate::tr!("mixed-command", command = hint)
    } else {
        hint
    })
}

pub fn show(
    ui: &mut egui::Ui,
    view: &Projection,
    range: Range<usize>,
    keymap: &Keymap,
) -> Option<Command> {
    let lines = view.selected_lines(range.clone());
    let kind = view.blocks[lines.start].public_kind();
    let mixed = view.blocks[lines]
        .iter()
        .any(|block| block.public_kind() != kind);
    let mut action = None;
    ui.scope(|ui| {
        // Les états de sélection de la barre restent neutres, indépendants de
        // la sélection bleue du texte et des styles sémantiques de l’éditeur.
        neutral_selection(ui);
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        ui.spacing_mut().interact_size.y = 32.0;
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::Label::new(egui::RichText::new("typstOgraphe").size(17.0)).truncate());
            ui.separator();
            let heading = match (mixed, kind) {
                (false, BlockKind::Heading(_)) => Icon::for_block(kind),
                _ => Icon::Heading,
            };
            let active = !mixed && matches!(kind, BlockKind::Heading(_));
            let (response, _) =
                egui::containers::menu::MenuButton::from_button(button(ui, heading, active, true))
                    .ui(ui, |ui| {
                        for definition in keymap
                            .registry()
                            .definitions()
                            .filter(|d| d.in_heading_menu())
                        {
                            let command = definition.command;
                            let Command::Block(block) = command else {
                                continue;
                            };
                            if menu_item(
                                ui,
                                block,
                                !mixed && command == Command::Block(kind),
                                command.short_label(),
                            )
                            .on_hover_text(keymap.hint(ui.ctx(), command))
                            .clicked()
                            {
                                action = Some(command);
                                ui.close();
                            }
                        }
                    });
            describe(
                response,
                crate::tr!("heading-menu"),
                crate::tr!("heading-menu-hint"),
                active,
                mixed,
            );
            ui.separator();
            for (mark, command, icon) in [
                (Mark::Bold, Command::Bold, Icon::Bold),
                (Mark::Italic, Command::Italic, Icon::Italic),
            ] {
                let (active, mixed) = view.mark_state(range.clone(), mark);
                let response = ui.add(button(ui, icon, active, false));
                if describe(
                    response,
                    command.label(),
                    keymap.hint(ui.ctx(), command),
                    active,
                    mixed,
                )
                .clicked()
                {
                    action = Some(command);
                }
            }
            let linked = if range.is_empty() {
                view.typing_style.link.is_some()
            } else {
                view.styles[range.clone()]
                    .iter()
                    .all(|style| style.link.is_some())
            };
            let response = ui.add(button(ui, Icon::Link, linked, false));
            if describe(
                response,
                Command::Link.label(),
                keymap.hint(ui.ctx(), Command::Link),
                linked,
                false,
            )
            .clicked()
            {
                action = Some(Command::Link);
            }
            ui.separator();
            let active = !mixed && kind.is_list();
            let (response, _) = egui::containers::menu::MenuButton::from_button(button(
                ui,
                Icon::List,
                active,
                true,
            ))
            .ui(ui, |ui| {
                for definition in keymap.registry().definitions().filter(|d| d.in_list_menu()) {
                    let command = definition.command;
                    let Command::Block(block) = command else {
                        continue;
                    };
                    let active = !mixed && command == Command::Block(kind);
                    if menu_item(ui, block, active, command.label())
                        .on_hover_text(keymap.hint(ui.ctx(), command))
                        .clicked()
                    {
                        action = Some(if active {
                            Command::Block(BlockKind::Paragraph)
                        } else {
                            command
                        });
                        ui.close();
                    }
                }
            });
            describe(
                response,
                crate::tr!("list-menu"),
                if active {
                    Command::Block(kind).label()
                } else {
                    crate::tr!("list-menu")
                },
                active,
                false,
            );
            let command = Command::Block(BlockKind::Quote);
            let active = !mixed && kind == BlockKind::Quote;
            let response = ui.add(button(ui, Icon::Quote, active, false));
            if describe(
                response,
                command.label(),
                keymap.hint(ui.ctx(), command),
                active,
                false,
            )
            .clicked()
            {
                action = Some(if active {
                    Command::Block(BlockKind::Paragraph)
                } else {
                    command
                });
            }
            let command = Command::InsertImage;
            let response = ui.add(button(ui, Icon::Image, false, false));
            if describe(
                response,
                command.label(),
                keymap.hint(ui.ctx(), command),
                false,
                false,
            )
            .clicked()
            {
                action = Some(command);
            }
        });
    });
    action
}
