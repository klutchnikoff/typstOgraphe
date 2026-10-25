//! Capture de revue des composants communs : `cargo run -p typstographe --example
//! ui-kit-gallery -- sortie.png [light|dark] [gallery|dialog]`.
use eframe::egui;
use std::path::PathBuf;
use typstographe::{
    icons::Icon,
    theme::Tokens,
    ui_kit::{
        ButtonKind, Dialog, Gallery, SidePanel, SideRail, button, gallery, segmented, text_field,
    },
};

struct Sheet {
    out: PathBuf,
    dark: bool,
    dialog: bool,
    rail: bool,
    side: SideRail,
    state: Gallery,
    frames: u32,
    requested: bool,
}

impl eframe::App for Sheet {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.set_theme(if self.dark {
            egui::ThemePreference::Dark
        } else {
            egui::ThemePreference::Light
        });
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        let tokens = Tokens::new(self.dark);
        if self.rail {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(tokens.bg))
                .show(ctx, |ui| {
                    let all = ui.max_rect();
                    let preview = egui::Rect::from_min_max(all.center_top(), all.max);
                    ui.painter()
                        .rect_filled(preview, 0.0, tokens.bg.gamma_multiply(0.9));
                    let panels = [
                        SidePanel {
                            id: "plan",
                            icon: Icon::List,
                            label: "Plan du document".into(),
                        },
                        SidePanel {
                            id: "notes",
                            icon: Icon::Quote,
                            label: "Notes".into(),
                        },
                    ];
                    self.side.open.get_or_insert("plan");
                    self.side.show(ui, preview, &panels, true, |_, ui| {
                        ui.label("Objet : demande de réservation");
                        ui.label("Madame, Monsieur");
                        ui.label("Équipements demandés");
                    });
                });
        } else {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(tokens.bg).inner_margin(24))
                .show(ctx, |ui| {
                    ui.set_max_width(520.0);
                    gallery(ui, &mut self.state);
                });
        }
        if self.dialog {
            let options = [
                (
                    typstographe::ui_kit::Orientation::Portrait,
                    "Portrait".to_owned(),
                ),
                (
                    typstographe::ui_kit::Orientation::Landscape,
                    "Paysage".to_owned(),
                ),
            ];
            Dialog::new("sheet-dialog", "Exporter")
                .description("Le dossier doit être nouveau ou vide.")
                .width(460.0)
                .show(
                    ctx,
                    |ui| {
                        ui.label("Dossier");
                        text_field(ui, "dialog-folder", &mut self.state.text);
                        ui.add_space(8.0);
                        segmented(ui, &mut self.state.orientation, &options);
                    },
                    |ui| {
                        button(ui, ButtonKind::Primary, "Exporter");
                        button(ui, ButtonKind::Ghost, "Annuler");
                    },
                );
        }
        self.frames += 1;
        if self.frames > 8 && !self.requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.requested = true;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            typstographe::write_screenshot(&self.out, &image).expect("capture écrite");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .ok_or("usage : ui-kit-gallery sortie.png [light|dark] [gallery|dialog]")?,
    );
    let dark = args.next().as_deref() == Some("dark");
    let scenario = args.next();
    let dialog = scenario.as_deref() == Some("dialog");
    let rail = scenario.as_deref() == Some("rail");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ui-kit",
        options,
        Box::new(move |cc| {
            typstographe::editor::install_fonts(&cc.egui_ctx);
            typstographe::theme::install(&cc.egui_ctx);
            Ok(Box::new(Sheet {
                out,
                dark,
                dialog,
                rail,
                side: SideRail::default(),
                state: Gallery::default(),
                frames: 0,
                requested: false,
            }))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
