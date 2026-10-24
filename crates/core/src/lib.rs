//! Moteur documentaire, ressources et rendu Typst indépendants de l’interface.
#![doc = include_str!("../../../docs/interface-contract.md")]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod annotations;
pub mod assets;
pub mod dates;
pub mod document;
pub mod document_types;
pub mod engine;
pub mod errors;
pub mod exporting;
mod file_access;
pub mod fonts;
mod identity;
pub mod navigation;
pub mod packages;
pub mod recovery;
pub mod rendering;
pub mod search;
mod session;
pub mod spelling;
pub mod statistics;
pub mod storage;
pub mod view_sync;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

mod position_map;
