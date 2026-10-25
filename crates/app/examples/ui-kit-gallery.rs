//! Capture de revue des composants communs : `cargo run -p typstographe --example
//! ui-kit-gallery -- sortie.png [light|dark] [gallery|dialog]`.
use eframe::egui;
use std::path::PathBuf;
use typstographe::{
    theme::Tokens,
    ui_kit::{ButtonKind, Dialog, Gallery, button, gallery, segmented, text_field},
};

struct Sheet {
    out: PathBuf,
    dark: bool,
    dialog: bool,
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
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(tokens.bg).inner_margin(24))
            .show(ctx, |ui| {
                ui.set_max_width(520.0);
                gallery(ui, &mut self.state);
            });
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
    let dialog = args.next().as_deref() == Some("dialog");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ui-kit",
        options,
        Box::new(move |_| {
            Ok(Box::new(Sheet {
                out,
                dark,
                dialog,
                state: Gallery::default(),
                frames: 0,
                requested: false,
            }))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
