//! Présentation de la barre : les actions retournent les commandes du registre.
use crate::{
    commands::{Keymap, UiCommand as Command},
    document::{BlockKind, Mark, Projection},
    icons::Icon,
};
use eframe::egui::{self, Response, Stroke, WidgetInfo, WidgetType};
use std::ops::Range;

pub const ICON_SIZE: f32 = 18.0;

/// États actifs de la barre et des rubriques : fond d'accent doux, trait d'accent.
pub fn neutral_selection(ui: &mut egui::Ui) {
    let t = crate::theme::Tokens::new(ui.visuals().dark_mode);
    ui.visuals_mut().selection.bg_fill = t.accent_soft;
    ui.visuals_mut().selection.stroke = Stroke::new(1.0_f32, t.accent);
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
        .min_size(egui::vec2(if menu { 44.0 } else { 30.0 }, 30.0))
        .frame_when_inactive(false)
        .selected(active)
}

fn menu_item(
    ui: &mut egui::Ui,
    kind: BlockKind,
    active: bool,
    enabled: bool,
    label: String,
) -> Response {
    neutral_selection(ui);
    let image = Command::Block(kind)
        .icon()
        .expect("block icon")
        .image(ui.ctx(), ICON_SIZE);
    ui.add_enabled(
        enabled,
        egui::Button::selectable(active, (image, label)).image_tint_follows_text_color(true),
    )
}

fn describe(
    response: Response,
    label: String,
    hint: String,
    active: bool,
    mixed: bool,
) -> Response {
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::Button, response.enabled(), active, &label)
    });
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
    enabled: impl Fn(Command) -> bool,
) -> Option<Command> {
    let lines = view.selected_lines(range.clone());
    let kind = view.blocks[lines.start].public_kind();
    let mixed = view.blocks[lines]
        .iter()
        .any(|block| block.public_kind() != kind);
    let mut action = None;
    ui.scope(|ui| {
        // États actifs en accent doux, indépendants de la sélection bleue du texte et
        // des styles sémantiques de l’éditeur.
        neutral_selection(ui);
        crate::ui_kit::icon_style(ui);
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        ui.spacing_mut().interact_size.y = 30.0;
        ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);
        ui.horizontal_top(|ui| {
            let right_edge = ui.max_rect().right();
            let reserved = 2.0 * 30.0 + 4.0 + 8.0;
            let width = (ui.available_width() - reserved).max(120.0);
            let left = ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    left_cluster(
                        ui,
                        view,
                        range.clone(),
                        keymap,
                        &enabled,
                        kind,
                        mixed,
                        &mut action,
                    )
                },
            );
            // Annuler et rétablir sont calés sur le bord droit de la barre.
            let slot = egui::Rect::from_min_size(
                egui::pos2(right_edge - (reserved - 4.0), left.response.rect.top()),
                egui::vec2(reserved - 4.0, left.response.rect.height()),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(slot)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    // Dans l'ordre d'apparition de droite à gauche : rétablir, puis annuler.
                    for (command, icon) in
                        [(Command::Redo, Icon::Redo), (Command::Undo, Icon::Undo)]
                    {
                        let response = ui
                            .add_enabled_ui(enabled(command), |ui| {
                                crate::ui_kit::icon_toggle(ui, icon, false, &command.label())
                            })
                            .inner;
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
                    }
                },
            );
        });
    });
    action
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn left_cluster(
    ui: &mut egui::Ui,
    view: &Projection,
    range: Range<usize>,
    keymap: &Keymap,
    enabled: &impl Fn(Command) -> bool,
    kind: BlockKind,
    mixed: bool,
    action: &mut Option<Command>,
) {
    let heading = match (mixed, kind) {
        (false, BlockKind::Heading(_)) => Command::Block(kind).icon().expect("heading icon"),
        _ => Icon::Heading,
    };
    let active = !mixed && matches!(kind, BlockKind::Heading(_));
    let (response, _) = egui::containers::menu::MenuButton::from_button(button(
        ui, heading, active, true,
    ))
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
                enabled(command),
                command.short_label(),
            )
            .on_hover_text(keymap.hint(ui.ctx(), command))
            .clicked()
            {
                *action = Some(command);
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
    for (mark, command) in [(Mark::Bold, Command::Bold), (Mark::Italic, Command::Italic)] {
        let icon = command.definition().icon().expect("format icon");
        let (active, mixed) = view.mark_state(range.clone(), mark);
        let response = ui
            .add_enabled_ui(enabled(command), |ui| {
                crate::ui_kit::icon_toggle(ui, icon, active, &command.label())
            })
            .inner;
        if describe(
            response,
            command.label(),
            keymap.hint(ui.ctx(), command),
            active,
            mixed,
        )
        .clicked()
        {
            *action = Some(command);
        }
    }
    let linked = if range.is_empty() {
        view.typing_style.link.is_some()
    } else {
        view.styles[range.clone()]
            .iter()
            .all(|style| style.link.is_some())
    };
    let response = ui
        .add_enabled_ui(enabled(Command::Link), |ui| {
            crate::ui_kit::icon_toggle(
                ui,
                Command::Link.definition().icon().expect("link icon"),
                linked,
                &Command::Link.label(),
            )
        })
        .inner;
    if describe(
        response,
        Command::Link.label(),
        keymap.hint(ui.ctx(), Command::Link),
        linked,
        false,
    )
    .clicked()
    {
        *action = Some(Command::Link);
    }
    ui.separator();
    let active = !mixed && kind.is_list();
    let (response, _) =
        egui::containers::menu::MenuButton::from_button(button(ui, Icon::List, active, true)).ui(
            ui,
            |ui| {
                for definition in keymap.registry().definitions().filter(|d| d.in_list_menu()) {
                    let command = definition.command;
                    let Command::Block(block) = command else {
                        continue;
                    };
                    let active = !mixed && command == Command::Block(kind);
                    let target = if active {
                        Command::Block(BlockKind::Paragraph)
                    } else {
                        command
                    };
                    if menu_item(ui, block, active, enabled(target), command.label())
                        .on_hover_text(keymap.hint(ui.ctx(), command))
                        .clicked()
                    {
                        *action = Some(target);
                        ui.close();
                    }
                }
            },
        );
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
    let target = if active {
        Command::Block(BlockKind::Paragraph)
    } else {
        command
    };
    let response = ui
        .add_enabled_ui(enabled(target), |ui| {
            crate::ui_kit::icon_toggle(
                ui,
                command.definition().icon().expect("quote icon"),
                active,
                &command.label(),
            )
        })
        .inner;
    if describe(
        response,
        command.label(),
        keymap.hint(ui.ctx(), command),
        active,
        false,
    )
    .clicked()
    {
        *action = Some(target);
    }
    let command = Command::InsertImage;
    let response = ui
        .add_enabled_ui(enabled(command), |ui| {
            crate::ui_kit::icon_toggle(
                ui,
                command.definition().icon().expect("image icon"),
                false,
                &command.label(),
            )
        })
        .inner;
    if describe(
        response,
        command.label(),
        keymap.hint(ui.ctx(), command),
        false,
        false,
    )
    .clicked()
    {
        *action = Some(command);
    }
    end_menu(ui, keymap, enabled, kind, mixed, action);
}

/// Un seul bouton pour le post-scriptum et les pièces jointes (D40) : les commandes,
/// leurs identifiants et leurs raccourcis restent ceux du registre.
fn end_menu(
    ui: &mut egui::Ui,
    keymap: &Keymap,
    enabled: &impl Fn(Command) -> bool,
    kind: BlockKind,
    mixed: bool,
    action: &mut Option<Command>,
) {
    let entries: Vec<_> = [
        (
            Command::InsertPostscript,
            BlockKind::Postscript,
            Icon::PenLine,
        ),
        (
            Command::InsertAttachment,
            BlockKind::Attachment,
            Icon::Paperclip,
        ),
    ]
    .into_iter()
    .filter(|(command, ..)| keymap.registry().contains(*command))
    .collect();
    if entries.is_empty() {
        return;
    }
    let active = !mixed && entries.iter().any(|(_, block, _)| kind == *block);
    let (response, _) =
        egui::containers::menu::MenuButton::from_button(button(ui, Icon::Paperclip, active, true))
            .ui(ui, |ui| {
                for (command, block, icon) in &entries {
                    let image = icon.image(ui.ctx(), ICON_SIZE);
                    let entry = ui.add_enabled(
                        enabled(*command),
                        egui::Button::selectable(
                            !mixed && kind == *block,
                            (image, command.label()),
                        )
                        .image_tint_follows_text_color(true)
                        .shortcut_text(keymap.shortcut_text(ui.ctx(), *command)),
                    );
                    if entry
                        .on_hover_text(keymap.hint(ui.ctx(), *command))
                        .clicked()
                    {
                        *action = Some(*command);
                        ui.close();
                    }
                }
            });
    describe(
        response,
        crate::tr!("end-menu"),
        crate::tr!("end-menu-hint"),
        active,
        false,
    );
}
