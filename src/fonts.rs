//! Catalogue embarqué, commun à l’éditeur et au rendu. Aucun accès aux polices
//! de la machine : un document garde les mêmes secours sur les trois systèmes.
use std::{collections::BTreeSet, path::Path, sync::OnceLock};
use typst::{foundations::Bytes, layout::FrameItem, text::Font};

pub struct Fallback {
    pub name: &'static str,
    pub data: &'static [u8],
}

pub const CJK: [Fallback; 2] = [
    Fallback {
        name: "NotoSansCJKjp-Regular.otf",
        data: include_bytes!("../assets/fonts/noto-cjk/NotoSansCJKjp-Regular.otf"),
    },
    Fallback {
        name: "NotoSansCJKjp-Bold.otf",
        data: include_bytes!("../assets/fonts/noto-cjk/NotoSansCJKjp-Bold.otf"),
    },
];
const LICENSE: &[u8] = include_bytes!("../assets/fonts/noto-cjk/LICENSE");

pub fn typst_fonts() -> Vec<Font> {
    static FONTS: OnceLock<Vec<Font>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            typst_assets::fonts()
                .chain(CJK.iter().map(|font| font.data))
                .flat_map(|data| Font::iter(Bytes::new(data)))
                .collect()
        })
        .clone()
}

/// Exporter uniquement les faces de secours effectivement composées. Le PDF
/// contient déjà ses sous-ensembles ; ces fichiers servent à recompiler le Typst.
pub fn export_used(document: &typst_layout::PagedDocument, root: &Path) -> std::io::Result<()> {
    fn collect(frame: &typst::layout::Frame, used: &mut BTreeSet<usize>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => collect(&group.frame, used),
                FrameItem::Text(text) => {
                    for (index, font) in CJK.iter().enumerate() {
                        if text.font.data().as_slice() == font.data {
                            used.insert(index);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut used = BTreeSet::new();
    for page in document.pages() {
        collect(&page.frame, &mut used);
    }
    if used.is_empty() {
        return Ok(());
    }
    let directory = root.join("fonts");
    std::fs::create_dir(&directory)?;
    for index in used {
        let font = &CJK[index];
        std::fs::write(directory.join(font.name), font.data)?;
    }
    std::fs::write(directory.join("LICENSE-Noto-CJK.txt"), LICENSE)?;
    std::fs::write(
        directory.join("README.txt"),
        "Noto Sans CJK 2.004 — SIL Open Font License 1.1\n\n\
         Compile from the export directory using the included fonts:\n\
         typst compile --font-path fonts main.typ\n\n\
         Upstream: https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004\n",
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn cjk_source_exports_include_only_used_faces_and_the_license() {
        use crate::{
            document::Document,
            document_types::{DocumentType, Metadata},
            rendering,
        };
        for (source, expected) in [
            ("Bonjour", vec![]),
            ("日本", vec![0]),
            ("#strong[日本]", vec![1]),
            ("日本 #strong[中国]", vec![0, 1]),
        ] {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path().join("export");
            let mut world = rendering::TypstWorld::default();
            let document = Document::from_source(source).unwrap();
            let template = "#let courrier(..args, body) = body";
            let request = rendering::RenderRequest {
                revision: 1,
                source: document.source().into(),
                source_map: document.source_map.clone(),
                metadata: Metadata::demo(DocumentType::Courrier),
                template: template.into(),
                assets: Default::default(),
                dependencies: crate::packages::for_template(template),
                export_dir: Some(directory.clone()),
            };
            rendering::render(&mut world, &request).unwrap();
            assert!(directory.join("courrier.pdf").exists());
            assert!(directory.join("main.typ").exists());
            assert_eq!(directory.join("fonts").exists(), !expected.is_empty());
            for (index, font) in CJK.iter().enumerate() {
                let path = directory.join("fonts").join(font.name);
                assert_eq!(path.exists(), expected.contains(&index), "{source}");
                if path.exists() {
                    assert!(std::fs::read(path).unwrap() == font.data);
                }
            }
            if !expected.is_empty() {
                assert!(
                    std::fs::read(directory.join("fonts/LICENSE-Noto-CJK.txt")).unwrap() == LICENSE
                );
                assert!(
                    std::fs::read_to_string(directory.join("fonts/README.txt"))
                        .unwrap()
                        .contains("--font-path fonts main.typ")
                );
            }
        }
    }
}
