"""Install the repository hooks without replacing another hook configuration."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def install():
    existing = subprocess.run(['git', 'config', '--get', 'core.hooksPath'], cwd=ROOT,
                              text=True, capture_output=True)
    if existing.returncode not in (0, 1):
        raise RuntimeError(existing.stderr)
    if existing.returncode == 0 and existing.stdout.strip() != '.githooks':
        raise RuntimeError('An existing core.hooksPath was found; integrate the hooks explicitly instead of replacing it.')
    hook_dir = Path(subprocess.check_output(['git', 'rev-parse', '--git-path', 'hooks'], cwd=ROOT, text=True).strip())
    if not hook_dir.is_absolute():
        hook_dir = ROOT / hook_dir
    if not existing.stdout.strip() and any((hook_dir / name).exists() for name in ['pre-commit', 'pre-push']):
        raise RuntimeError('Existing native hooks found; integrate them explicitly instead of bypassing them.')
    subprocess.run(['git', 'config', '--local', 'core.hooksPath', '.githooks'], cwd=ROOT, check=True)
    print('Repository hooks enabled: staged checks before commit, isolated checks before push.')


if __name__ == '__main__':
    install()
