//! Les dialogues synchrones sont appelés depuis la frame egui, sur le thread principal.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct FileDialogUnavailable;

pub(super) type FileDialogResult = Result<Option<PathBuf>, FileDialogUnavailable>;

pub(super) fn save_file(dialog: rfd::FileDialog) -> FileDialogResult {
    run_dialog(move || dialog.save_file())
}

impl App {
    pub(super) fn pick_file(&mut self, dialog: rfd::FileDialog) -> Option<PathBuf> {
        self.file_dialog_result(run_dialog(move || dialog.pick_file()))
    }

    pub(super) fn file_dialog_result(&mut self, result: FileDialogResult) -> Option<PathBuf> {
        match result {
            Ok(path) => path,
            Err(FileDialogUnavailable) => {
                self.file.error = Some(crate::tr!("file-dialog-unavailable"));
                None
            }
        }
    }
}

fn run_dialog(show: impl FnOnce() -> Option<PathBuf> + std::panic::UnwindSafe) -> FileDialogResult {
    #[cfg(any(target_os = "macos", test))]
    {
        match std::panic::catch_unwind(show) {
            Ok(path) => Ok(path),
            Err(payload) => {
                let message = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied());
                // objc2 panics in Rust when AppKit returns nil from either constructor.
                // In rfd 0.15.4 this happens BEFORE Panel::new, its focus/policy guards,
                // and runModal. No application state is captured by `show`.
                // Do not catch arbitrary failures during the native modal event loop.
                if matches!(
                    message,
                    Some(
                        "unexpected NULL returned from +[NSSavePanel savePanel]"
                            | "unexpected NULL returned from +[NSOpenPanel openPanel]"
                    )
                ) {
                    Err(FileDialogUnavailable)
                } else {
                    std::panic::resume_unwind(payload)
                }
            }
        }
    }
    #[cfg(not(any(target_os = "macos", test)))]
    {
        Ok(show())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn selection_and_cancellation_are_distinct_from_an_unavailable_panel() {
        let path = PathBuf::from("letter.typsto");
        assert_eq!(run_dialog(|| Some(path.clone())), Ok(Some(path)));
        assert_eq!(run_dialog(|| None), Ok(None));
        for message in [
            "unexpected NULL returned from +[NSSavePanel savePanel]",
            "unexpected NULL returned from +[NSOpenPanel openPanel]",
        ] {
            assert_eq!(
                run_dialog(|| panic!("{message}")),
                Err(FileDialogUnavailable)
            );
            assert_eq!(
                run_dialog(|| std::panic::panic_any(message)),
                Err(FileDialogUnavailable)
            );
        }
    }

    #[test]
    fn unrelated_panics_are_resumed_with_their_original_payload() {
        let result = std::panic::catch_unwind(|| run_dialog(|| std::panic::panic_any(42_u32)));
        assert_eq!(*result.unwrap_err().downcast::<u32>().unwrap(), 42);
        let result = std::panic::catch_unwind(|| {
            run_dialog(|| panic!("unexpected NULL returned from +[AnotherClass method]"))
        });
        assert_eq!(
            result.unwrap_err().downcast_ref::<&str>().copied(),
            Some("unexpected NULL returned from +[AnotherClass method]")
        );
    }
}
