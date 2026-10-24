use clap::Parser;
use serde_json::json;
use std::{
    ffi::OsString,
    io::{self, Write},
    process::ExitCode,
};
use typstographe_cli::args::Cli;
use typstographe_i18n as i18n;

fn main() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().collect();
    // Seulement les options de présentation : clap reste l’unique analyseur des commandes.
    let mut json_output = false;
    let mut requested = None;
    let mut command = None;
    let mut values = arguments.iter().skip(1);
    while let Some(value) = values.next() {
        if value == "--" {
            break;
        }
        if value == "--json" {
            json_output = true;
        }
        if command.is_none() && !value.to_string_lossy().starts_with('-') {
            command = Some(value.clone());
        }
        let language = if value == "--lang" {
            values.next().and_then(|s| s.to_str())
        } else {
            value.to_str().and_then(|s| s.strip_prefix("--lang="))
        };
        if let Some(language @ ("fr" | "en")) = language {
            requested = language.parse().ok();
        }
    }
    // La construction de l'aide clap utilise déjà le chargeur. Choisir le
    // défaut MCP avant cette construction, sans analyser ses paramètres :
    // clap reste responsable de la validation et de l'exécution de la commande.
    if let Some(language) = requested.or_else(|| {
        (command.as_deref() == Some(std::ffi::OsStr::new("mcp")))
            .then(|| "en".parse().expect("MCP default language"))
    }) {
        i18n::initialize(&[language]);
    }
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error) => {
            let exit = u8::from(error.use_stderr()) * 2;
            let value = if exit == 0 {
                json!({"version":1,"ok":true,"outcome":{"help":error.to_string()}})
            } else {
                json!({"version":1,"ok":false,"error":{"code":"usage","message":error.to_string(),"params":{}}})
            };
            let result = if json_output {
                write_json(&value)
            } else if error.use_stderr() {
                write!(io::stderr().lock(), "{error}")
            } else {
                write!(io::stdout().lock(), "{error}")
            };
            return ExitCode::from(if result.is_err() { 3 } else { exit });
        }
    };
    let json_output = cli.json;
    if let typstographe_cli::args::Action::Mcp {
        read_only,
        read_roots,
        write_roots,
    } = cli.action
    {
        return match typstographe_cli::mcp::serve(read_only, read_roots, write_roots) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "{error}");
                ExitCode::from(3)
            }
        };
    }
    let (exit, value, text) = match typstographe_cli::execute(cli) {
        Ok(report) => (
            report.exit,
            json!({"version":1,"ok":true,"outcome":report.outcome}),
            report.text,
        ),
        Err(error) => {
            let exit = if error.code == typstographe_core::engine::ErrorCode::Io {
                3
            } else {
                1
            };
            let text = i18n::command_error(&error);
            (
                exit,
                json!({"version":1,"ok":false,"error":{"code":error.code,"message":text,"params":error.params}}),
                text,
            )
        }
    };
    let result = if json_output {
        write_json(&value)
    } else if value["ok"] == false {
        writeln!(io::stderr().lock(), "{text}")
    } else {
        write!(io::stdout().lock(), "{text}")
    };
    ExitCode::from(if result.is_err() { 3 } else { exit })
}

fn write_json(value: &serde_json::Value) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)
}
