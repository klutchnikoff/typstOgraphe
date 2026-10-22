use super::CommandError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct CommandId(String);
impl TryFrom<String> for CommandId {
    type Error = CommandError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let word = |text: &str| {
            text.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && text
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        };
        if !value
            .rsplit_once('.')
            .is_some_and(|(domain, action)| domain.split('.').all(word) && word(action))
        {
            return Err(CommandError::invalid(
                "command IDs must use dotted namespaces and actions in lowercase ASCII",
            ));
        }
        Ok(Self(value))
    }
}
impl From<CommandId> for String {
    fn from(id: CommandId) -> Self {
        id.0
    }
}
impl CommandId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub id: CommandId,
    pub params: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Edit,
    Selection,
    History,
    Letter,
    Query,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityRule {
    Document,
    Letter,
    Undo,
    Redo,
    LinkSelection,
    Read,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommandSpec {
    pub id: CommandId,
    pub description: &'static str,
    pub family: Family,
    pub params_schema: Value,
    pub availability: AvailabilityRule,
}
impl CommandSpec {
    pub(super) fn new<P: JsonSchema>(
        id: &str,
        description: &'static str,
        family: Family,
        availability: AvailabilityRule,
    ) -> Self {
        Self {
            id: id.to_owned().try_into().expect("valid built-in command ID"),
            description,
            family,
            availability,
            params_schema: serde_json::to_value(schemars::schema_for!(P))
                .expect("serializable schema"),
        }
    }
}

/// Une déclaration définit à la fois l'enum, son protocole et son catalogue.
macro_rules! define_calls {
    ($name:ident, $specs:ident; $( $variant:ident($params:ty) => $id:literal, $description:literal, $family:ident, $availability:ident; )*) => {
        #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(tag = "id", content = "params", deny_unknown_fields)]
        pub enum $name {
            $( #[serde(rename = $id)] $variant($params), )*
        }
        impl $name {
            pub fn id(&self) -> &'static str {
                match self { $( Self::$variant(_) => $id, )* }
            }
            pub fn from_call(id: &str, params: serde_json::Value) -> Result<Self, CommandError> {
                let _: super::CommandId = id.to_owned().try_into()?;
                if !Self::$specs().iter().any(|spec| spec.id.as_str() == id) {
                    return Err(CommandError::new(super::ErrorCode::NotFound, "unknown_command"));
                }
                serde_json::from_value(serde_json::json!({"id": id, "params": params}))
                    .map_err(CommandError::invalid)
            }
            pub fn to_call(&self) -> super::Call {
                let value = serde_json::to_value(self).expect("serializable command");
                super::Call {
                    id: self.id().to_owned().try_into().expect("valid command ID"),
                    params: value["params"].clone(),
                }
            }
            pub fn $specs() -> &'static [super::CommandSpec] {
                static SPECS: std::sync::OnceLock<Vec<super::CommandSpec>> = std::sync::OnceLock::new();
                SPECS.get_or_init(|| vec![$(
                    super::CommandSpec::new::<$params>($id, $description, super::Family::$family, super::AvailabilityRule::$availability),
                )*])
            }
        }
    };
}

/// Diagnostic généré pendant la migration ; les identifiants ne seront gelés
/// qu'une fois toutes les familles de phase 2 migrées et relues.
pub fn catalogue_markdown(value: &Value) -> String {
    let mut text = String::from(
        "# Catalogue documentaire implémenté\n\nFichier généré par `cargo run --locked -- --commands-catalogue`.\nCe catalogue de phase 2 est partiel et ses signatures restent en cours de validation.\nLes actions d’interface et les familles non migrées n’y figurent pas.\nLes schémas JSON sont disponibles avec `--commands-catalogue-json`.\n\n| Identifiant | Famille | Description |\n| --- | --- | --- |\n",
    );
    for spec in value["entries"].as_array().expect("catalogue entries") {
        text.push_str(&format!(
            "| `{}` | {} | {} |\n",
            spec["id"].as_str().unwrap(),
            spec["family"].as_str().unwrap(),
            spec["description"].as_str().unwrap()
        ));
    }
    text
}
pub(super) use define_calls;

pub fn catalogue() -> &'static [CommandSpec] {
    static SPECS: OnceLock<Vec<CommandSpec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let specs: Vec<_> = super::Command::command_specs()
            .iter()
            .chain(super::Query::query_specs())
            .cloned()
            .collect();
        let unique: std::collections::BTreeSet<_> =
            specs.iter().map(|spec| spec.id.as_str()).collect();
        assert_eq!(unique.len(), specs.len(), "duplicate command IDs");
        assert!(specs.iter().all(|spec| !spec.description.trim().is_empty()));
        specs
    })
}
