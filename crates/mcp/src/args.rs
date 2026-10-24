//! Sélecteurs communs aux adaptateurs CLI et MCP.
use clap::Args;
use typstographe_i18n::tr;

#[derive(Args, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Target {
    #[arg(long, conflicts_with = "block", help = tr!("cli-select-help"))]
    pub select: Option<String>,
    #[arg(long, requires = "select", value_parser = clap::value_parser!(u32).range(1..), help = tr!("cli-occurrence-help"))]
    pub occurrence: Option<u32>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..), help = tr!("cli-block-help"))]
    pub block: Option<u32>,
}
