use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use typstographe_i18n::tr;

#[derive(Parser)]
#[command(name = "typstographe-cli", version, about = tr!("cli-about"))]
pub struct Cli {
    #[arg(long, global = true, help = tr!("cli-json-help"))]
    pub json: bool,
    #[arg(long, global = true, value_enum, help = tr!("cli-lang-help"))]
    pub lang: Option<Language>,
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Language {
    Fr,
    En,
}

#[derive(Subcommand)]
pub enum Action {
    #[command(about = tr!("cli-new-help"))]
    New {
        #[arg(long = "type", help = tr!("cli-type-help"))]
        document_type: String,
        #[arg(long, help = tr!("cli-model-help"))]
        model: Option<String>,
        #[arg(short, long, help = tr!("cli-document-output-help"))]
        output: PathBuf,
        #[command(flatten)]
        params: Parameters,
    },
    #[command(about = tr!("cli-commands-help"))]
    Commands {
        #[command(subcommand)]
        action: CatalogueAction,
    },
    #[command(about = tr!("cli-inspect-help"))]
    Inspect { file: PathBuf },
    #[command(about = tr!("cli-check-help"))]
    Check {
        file: PathBuf,
        #[arg(long, help = tr!("cli-for-export-help"))]
        for_export: bool,
        #[arg(long, help = tr!("cli-spelling-help"))]
        spelling: bool,
    },
    #[command(about = tr!("cli-export-help"))]
    Export {
        file: PathBuf,
        #[arg(long, value_enum, default_value = "pdf")]
        format: Format,
        #[arg(short, long, help = tr!("cli-output-help"))]
        output: PathBuf,
        #[arg(long, help = tr!("cli-pages-help"))]
        pages: bool,
    },
}

#[derive(Args, Default)]
pub struct Parameters {
    #[arg(long, conflicts_with = "params_file", help = tr!("cli-params-help"))]
    pub params: Option<String>,
    #[arg(long, help = tr!("cli-params-file-help"))]
    pub params_file: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum CatalogueAction {
    #[command(about = tr!("cli-list-help"))]
    List {
        #[arg(long, help = tr!("cli-query-help"))]
        query: bool,
        #[arg(long, conflicts_with_all = ["query", "json"], help = tr!("cli-markdown-help"))]
        markdown: bool,
    },
    #[command(about = tr!("cli-describe-help"))]
    Describe { id: String },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Pdf,
    Typst,
    Both,
}
impl From<Format> for typstographe_core::engine::ExportFormat {
    fn from(value: Format) -> Self {
        match value {
            Format::Pdf => Self::Pdf,
            Format::Typst => Self::Typst,
            Format::Both => Self::Both,
        }
    }
}
