//! Adaptateur synchrone : aucune logique documentaire ni préférence graphique.
pub mod args;
mod output;
use args::{Action, CatalogueAction, Cli};
use serde_json::{Value, json};
use typstographe_core::engine::*;
use typstographe_i18n::tr;

pub struct Report {
    pub outcome: Value,
    pub text: String,
    pub exit: u8,
}

pub fn execute(cli: Cli) -> Result<Report, CommandError> {
    match cli.action {
        Action::Commands { action } => catalogue_report(action),
        Action::Inspect { file } => {
            let engine = open(file)?;
            let outcome = json!({
                "document":engine.query(Query::Inspect(Empty {}))?,
                "metadata":engine.query(Query::Metadata(Empty {}))?,
                "structure":engine.query(Query::Structure(Empty {}))?,
                "statistics":engine.query(Query::Statistics(StatisticsParams::default()))?,
            });
            Ok(Report {
                text: output::inspect(&outcome),
                outcome,
                exit: 0,
            })
        }
        Action::Check {
            file,
            for_export,
            spelling,
        } => {
            let engine = open(file)?;
            let validation = engine.validate(GuardParams {
                guard: engine.target_guard(),
            })?;
            let issues: Option<SpellingIssues> = spelling
                .then(|| {
                    engine
                        .query(Query::SpellingIssues(Empty {}))
                        .map(|value| serde_json::from_value(value).expect("spelling issues"))
                })
                .transpose()?;
            let passed = validation.valid
                && (!for_export || validation.ready_for_export)
                && issues
                    .as_ref()
                    .is_none_or(|s| s.available && s.issues.is_empty());
            let outcome = json!({"passed":passed,"for_export":for_export,"validation":validation,"spelling":issues});
            Ok(Report {
                text: output::check(&outcome),
                outcome,
                exit: u8::from(!passed),
            })
        }
        Action::Export {
            file,
            format,
            output,
            pages,
        } => {
            let mut engine = open(file)?;
            let outcome = engine.run(Command::Export(ExportParams {
                guard: engine.target_guard(),
                directory: output.clone(),
                format: format.into(),
                page_images: pages,
            }))?;
            Ok(Report {
                outcome: serde_json::to_value(outcome).expect("serializable outcome"),
                text: format!(
                    "{}\n",
                    tr!("cli-exported", path = output.display().to_string())
                ),
                exit: 0,
            })
        }
    }
}

fn open(file: std::path::PathBuf) -> Result<Engine, CommandError> {
    let mut engine = Engine::empty();
    engine.run(Command::OpenDocument(OpenDocumentParams {
        guard: engine.target_guard(),
        path: file,
        read_only: true,
        discard_changes: false,
    }))?;
    Ok(engine)
}

fn catalogue_report(action: CatalogueAction) -> Result<Report, CommandError> {
    let outcome = match action {
        CatalogueAction::List { markdown: true, .. } => {
            return Ok(Report {
                outcome: Value::Null,
                text: documentation(),
                exit: 0,
            });
        }
        CatalogueAction::List { query, .. } => {
            let entries: Vec<_> = catalogue()
                .iter()
                .filter(|s| !query || s.family == Family::Query)
                .collect();
            json!({"entries": entries})
        }
        CatalogueAction::Describe { id } => {
            let spec = catalogue()
                .iter()
                .find(|s| s.id.as_str() == id)
                .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
            serde_json::to_value(spec).expect("serializable specification")
        }
    };
    Ok(Report {
        text: output::catalogue(&outcome),
        outcome,
        exit: 0,
    })
}

pub fn documentation() -> String {
    let mut text = String::from(
        "# CLI typstOgraphe\n\nDocument généré par `cargo run --locked -p typstographe-cli -- commands list --markdown`.\n\nLe premier lot permet de consulter le catalogue, inspecter, vérifier et exporter un fichier `.typsto`, sans fenêtre ni préférences personnelles implicites.\n\n```sh\ntypstographe-cli commands list --json\ntypstographe-cli commands describe metadata.set --json\ntypstographe-cli inspect courrier.typsto --json\ntypstographe-cli check courrier.typsto --for-export\ntypstographe-cli check courrier.typsto --spelling\ntypstographe-cli export courrier.typsto --format pdf --output export/\n```\n\nLes lectures utilisent la dernière version enregistrée, même si le document est ouvert dans l’application. L’export crée un répertoire ou utilise un répertoire vide, avec `--pages` pour ajouter des images PNG.\n\n`--lang fr|en` choisit la langue d’interface ; sinon langue système, repli anglais. La langue du document est indépendante. `--json` produit un objet `{\"version\":1,\"ok\":true,\"outcome\":…}` ou `{\"version\":1,\"ok\":false,\"error\":{\"code\",\"message\",\"params\"}}`. Aucun autre texte sur stdout en mode JSON.\n\nCodes de sortie : 0 succès, 1 contrôle négatif ou opération refusée, 2 arguments invalides, 3 entrée/sortie. Un contrôle négatif a `ok: true` (exécution réussie), `outcome.passed: false` et le code de sortie 1. L’orthographe est facultative ; avec `--spelling`, un dictionnaire indisponible ou une suspicion donne un contrôle négatif.\n\nCréation, `query`, `run`, scripts atomiques et images de pages dédiées : lots suivants. Les identités de session ne sont pas des références persistantes entre deux exécutions. MCP est prévu en phase 5.\n\n## Catalogue du moteur\n\nLe catalogue décrit les contrats du moteur, y compris les opérations qui ne disposent pas encore d’un déclencheur CLI. L’ouverture accepte l’option additive `read_only` (false par défaut).\n\n| Identifiant | Famille | Description |\n| --- | --- | --- |\n",
    );
    for spec in catalogue() {
        let family = serde_json::to_value(spec.family).expect("family");
        text.push_str(&format!(
            "| `{}` | {} | {} |\n",
            spec.id.as_str(),
            family.as_str().unwrap(),
            spec.description.replace('|', "\\|")
        ));
    }
    text
}
