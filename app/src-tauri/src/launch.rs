//! Parse launch options before any window or filesystem mutation.
use std::path::PathBuf;
#[derive(Default, Debug)]
pub struct Options {
    pub root: Option<PathBuf>,
    pub stdio: bool,
    pub diagnostic: Option<String>,
    pub initial_path: Option<String>,
}
impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>, recipe: bool) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut result = Self::default();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--stdio" | "--session-dir" | "--diagnostic" => {
                    if !recipe {
                        return Err(
                            "Recipe options require a binary built with --features recipe-driver."
                                .into(),
                        );
                    }
                    match arg.as_str() {
                        "--stdio" => result.stdio = true,
                        "--session-dir" => {
                            let path = args
                                .next()
                                .filter(|p| !p.is_empty() && !p.starts_with('-'))
                                .ok_or("--session-dir requires a path")?;
                            result.root = Some(PathBuf::from(path))
                        }
                        _ => result.diagnostic = Some("full".into()),
                    }
                }
                _ if arg.starts_with("--diagnostic-") => {
                    if !recipe {
                        return Err(
                            "Diagnostics require a binary built with --features recipe-driver."
                                .into(),
                        );
                    }
                    let mode = arg.trim_start_matches("--diagnostic-");
                    if ![
                        "synchronization-active",
                        "synchronization",
                        "performance",
                        "draft-conservation",
                        "draft-recover",
                        "draft-verify",
                        "publication",
                        "publication-verify",
                        "closure",
                        "closure-verify",
                        "restore",
                    ]
                    .contains(&mode)
                    {
                        return Err(format!("Unknown recipe mode: {mode}"));
                    }
                    result.diagnostic = Some(mode.into());
                }
                _ if arg.starts_with('-') => return Err(format!("Unknown option: {arg}")),
                _ => {
                    if result.initial_path.is_some() {
                        return Err("Only one project can be opened at launch.".into());
                    }
                    result.initial_path = Some(
                        std::path::absolute(arg)
                            .map_err(|e| e.to_string())?
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        if result.root.is_some() && !result.stdio && result.diagnostic.is_none() {
            return Err("An isolated session requires an explicit recipe mode.".into());
        }
        if result.root.is_some() && result.initial_path.is_some() {
            return Err("A recipe session cannot also open a personal project at launch.".into());
        }
        if result.stdio && result.diagnostic.is_some() {
            return Err("Choose either STDIO or a native recipe.".into());
        }
        if (result.stdio || result.diagnostic.is_some()) && result.root.is_none() {
            return Err("Recipes require an explicit --session-dir to isolate their work.".into());
        }
        Ok(result)
    }
}
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    fn parse(args: &[&str], recipe: bool) -> Result<Options, String> {
        Options::parse(args.iter().map(|s| s.to_string()), recipe)
    }
    #[test]
    fn normal_launch_never_enables_diagnostics_or_an_isolated_session() {
        assert!(parse(&[], false).unwrap().root.is_none());
        assert!(
            parse(&["example.typsto"], false)
                .unwrap()
                .initial_path
                .unwrap()
                .ends_with("example.typsto")
        );
        for args in [
            &["--stdio"][..],
            &["--diagnostic"],
            &["--diagnostic-closure"],
            &["--session-dir", "somewhere"],
            &["--unknown"],
        ] {
            assert!(parse(args, false).is_err());
        }
    }
    #[test]
    fn recipes_require_a_mode_and_explicit_isolation_without_ambiguous_options() {
        assert!(parse(&["--diagnostic-closure"], true).is_err());
        assert!(parse(&["--session-dir"], true).is_err());
        assert!(parse(&["--session-dir", "somewhere"], true).is_err());
        assert!(parse(&["--session-dir", ""], true).is_err());
        assert!(parse(&["--session-dir", "--stdio"], true).is_err());
        assert!(parse(&["--session-dir", "somewhere", "personal.typsto"], true).is_err());
        assert!(
            parse(
                &["--diagnostic-unknown", "--session-dir", "somewhere"],
                true
            )
            .is_err()
        );
        assert!(
            parse(
                &["--stdio", "--diagnostic", "--session-dir", "somewhere"],
                true
            )
            .is_err()
        );
        assert!(parse(&["a.typsto", "b.typsto"], true).is_err());
        assert!(
            parse(&["--stdio", "--session-dir", "somewhere"], true)
                .unwrap()
                .stdio
        );
        assert_eq!(
            parse(
                &["--diagnostic-closure", "--session-dir", "somewhere"],
                true
            )
            .unwrap()
            .diagnostic
            .as_deref(),
            Some("closure")
        );
    }
}
