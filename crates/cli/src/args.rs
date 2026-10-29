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
    #[command(about = tr!("cli-mcp-help"))]
    Mcp {
        #[arg(long, value_name = "INVITATION", conflicts_with_all = ["read_only", "read_roots", "write_roots"], help = tr!("cli-mcp-attach-help"))]
        attach: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["attach", "read_only", "read_roots", "write_roots"], help = tr!("cli-mcp-attach-active-help"))]
        attach_active: bool,
        #[arg(long, help = tr!("cli-mcp-read-only-help"))]
        read_only: bool,
        #[arg(long = "read-root", help = tr!("cli-mcp-read-root-help"))]
        read_roots: Vec<PathBuf>,
        #[arg(long = "write-root", help = tr!("cli-mcp-write-root-help"))]
        write_roots: Vec<PathBuf>,
    },
    #[command(about = tr!("cli-render-help"))]
    Render {
        file: PathBuf,
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..), help = tr!("cli-page-help"))]
        page: u32,
        #[arg(long, help = tr!("cli-png-help"))]
        png: PathBuf,
    },
    #[command(about = tr!("cli-script-help"))]
    Script {
        #[arg(help = tr!("cli-script-file-help"))]
        file: PathBuf,
    },
    #[command(about = tr!("cli-query-call-help"))]
    Query {
        id: String,
        file: Option<PathBuf>,
        #[command(flatten)]
        params: Parameters,
        #[command(flatten)]
        target: Target,
    },
    #[command(about = tr!("cli-run-help"))]
    Run {
        id: String,
        file: PathBuf,
        #[command(flatten)]
        params: Parameters,
        #[command(flatten)]
        target: Target,
        #[arg(long, conflicts_with = "output", help = tr!("cli-save-help"))]
        save: bool,
        #[arg(short, long, help = tr!("cli-document-output-help"))]
        output: Option<PathBuf>,
    },
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

pub use typstographe_mcp::args::Target;

#[derive(Subcommand)]
pub enum CatalogueAction {
    #[command(about = tr!("cli-catalogue-export-help"))]
    Export {
        #[arg(long, conflicts_with = "json", help = tr!("cli-markdown-help"))]
        markdown: bool,
    },
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
impl From<Format> for typstographe_core::protocol::ExportFormat {
    fn from(value: Format) -> Self {
        match value {
            Format::Pdf => Self::Pdf,
            Format::Typst => Self::Typst,
            Format::Both => Self::Both,
        }
    }
}
