//! fonts : détail interne de l’adaptateur egui.
use super::*;
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let bundled: Vec<_> = typst_assets::fonts().collect();
    let ordinary_fallback = fonts.families[&FontFamily::Proportional].clone();
    for font in &crate::fonts::CJK {
        fonts.font_data.insert(
            font.name.into(),
            egui::FontData::from_static(font.data).into(),
        );
    }
    // Police d'interface : Inter normal et semi-gras (licence OFL, `assets/fonts/inter`).
    for (name, data) in [
        (
            "Inter-Regular",
            include_bytes!("../../assets/fonts/inter/Inter-Regular.ttf").as_slice(),
        ),
        (
            "Inter-SemiBold",
            include_bytes!("../../assets/fonts/inter/Inter-SemiBold.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), egui::FontData::from_static(data).into());
    }
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .expect("famille proportionnelle")
        .insert(0, "Inter-Regular".into());
    let mut semibold = vec!["Inter-SemiBold".to_owned()];
    semibold.extend(ordinary_fallback.clone());
    fonts.families.insert(crate::theme::semibold(), semibold);
    // Dernier recours seulement : les glyphes latins et leurs métriques gardent
    // leurs polices actuelles. Les champs de l’interface ont le même secours.
    for family in fonts.families.values_mut() {
        family.push(crate::fonts::CJK[0].name.into());
    }
    // Ordre des variantes régulière, grasse, italique et grasse italique dans
    // les ressources de typst-assets 0.15.1, version fixée par Cargo.toml.
    for (family, faces) in [
        (EditorFont::Libertinus, [0, 1, 2, 3]),
        (EditorFont::ComputerModern, [9, 10, 11, 12]),
        (EditorFont::Monospace, [16, 13, 15, 14]),
    ] {
        for (variant, index) in faces.into_iter().enumerate() {
            let font_family = family.font_id(18.0, variant).family;
            let name = format!("{family:?}-{variant}");
            fonts.font_data.insert(
                name.clone(),
                egui::FontData::from_static(bundled[index]).into(),
            );
            let mut fallback = vec![name];
            fallback.extend(ordinary_fallback.clone());
            fallback.push(crate::fonts::CJK[variant % 2].name.into());
            fonts.families.insert(font_family, fallback);
        }
    }
    ctx.set_fonts(fonts);
}
