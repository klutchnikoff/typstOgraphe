//! Références opaques de rendu et géométrie en points Typst, sans pixels egui.
use super::TextTarget;
pub use crate::navigation::PagePoint;
use crate::{
    navigation::SelectionHighlight,
    session::{SelectionTarget, TargetGuard},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RenderId(pub(super) u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenderReference {
    pub guard: TargetGuard,
    pub id: RenderId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RenderInfo {
    pub reference: RenderReference,
    /// Dimensions des pages en points Typst.
    pub pages: Vec<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "direction", rename_all = "snake_case", deny_unknown_fields)]
pub enum RenderPosition {
    ToPreview {
        #[serde(default)]
        target: TextTarget,
    },
    ToSource {
        point: PagePoint,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocateParams {
    pub render: RenderReference,
    pub position: RenderPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LocateResult {
    pub render: RenderReference,
    /// None si le point désigne une décoration ou un contenu du modèle.
    pub target: Option<SelectionTarget>,
    /// None si aucun glyphe ou objet éditable n’a de position dans le rendu.
    pub point: Option<PagePoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectionBoxesParams {
    pub render: RenderReference,
    #[serde(default)]
    pub target: TextTarget,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SelectionBoxesResult {
    pub render: RenderReference,
    pub boxes: Vec<SelectionHighlight>,
}
