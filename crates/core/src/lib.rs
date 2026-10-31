//! Moteur documentaire, ressources et rendu Typst indépendants de l’interface.
#![doc = include_str!("../../../docs/interface-contract.md")]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod assets;
pub mod bibliography;
pub mod content;
pub mod dates;
pub mod document;
pub mod document_types;
pub mod errors;
pub mod exporting;
mod file_access;
pub use file_access::DocumentLease;
pub mod fonts;
pub mod navigation;
pub mod packages;
pub mod protocol;
pub mod rendering;
pub mod spelling;
pub mod statistics;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod typography_tests;
