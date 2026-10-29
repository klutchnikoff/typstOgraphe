use std::path::{Path, PathBuf};
pub fn relative_to(path: &mut PathBuf, base: &Path) {
    if path.is_relative() {
        *path = base.join(&*path);
    }
}
