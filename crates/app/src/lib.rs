//! Adaptateur graphique egui de typstOgraphe.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub use typstographe_core::{
    annotations, assets, dates, document, document_types, engine, errors, exporting, fonts,
    navigation, packages, recovery, rendering, search, spelling, statistics, storage, view_sync,
};
pub use typstographe_i18n as i18n;
pub use typstographe_i18n::tr;

pub mod app;
pub mod commands;
pub mod editor;
pub mod icons;
pub mod menus;
pub mod preferences;
pub mod profile;
pub mod theme;
pub mod toolbar;
pub mod ui_kit;

mod screenshot;
pub use screenshot::write_screenshot;
