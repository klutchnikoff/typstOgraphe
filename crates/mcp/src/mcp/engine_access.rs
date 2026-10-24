//! Ownership belongs to the autonomous worker or to the graphical application.
use std::ops::{Deref, DerefMut};
use typstographe_core::engine::Engine;
pub(super) enum EngineAccess<'a> {
    Owned(Box<Engine>),
    Borrowed(&'a mut Engine),
}
impl Deref for EngineAccess<'_> {
    type Target = Engine;
    fn deref(&self) -> &Engine {
        match self {
            Self::Owned(e) => e,
            Self::Borrowed(e) => e,
        }
    }
}
impl DerefMut for EngineAccess<'_> {
    fn deref_mut(&mut self) -> &mut Engine {
        match self {
            Self::Owned(e) => e,
            Self::Borrowed(e) => e,
        }
    }
}
