"""Check cost boundaries and that hooks inspect committed/staged, not working, data."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / f'scripts/{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


verify, hooks = load('verify'), load('git_hooks')


class VerificationTests(unittest.TestCase):
    def test_routine_levels_do_not_launch_browser_coverage_or_packaging(self):
        for level in ['quick', 'lint', 'test']:
            text = str(verify.commands(level))
            for forbidden in ['playwright', 'test:browser', 'llvm-cov', 'test:coverage', 'build:recipe']:
                self.assertNotIn(forbidden, text)
        self.assertNotIn('clippy', str(verify.commands('test')))
        self.assertNotIn("'test'", str(verify.commands('quick')))
        self.assertIn('clippy', str(verify.commands('lint')))

    def test_local_push_preserves_contract_checks_and_conservative_browser_selection(self):
        self.assertIn('test:contract', str(verify.commands('push', [])))
        self.assertIn('@smoke', str(verify.commands('push', [])))
        self.assertIn('--grep-invert', str(verify.commands('push', ['tests/browser/bibliography.spec.ts'])))
        full = verify.commands('push', None)[-1]
        self.assertEqual(full[-2:], ['--project', 'chromium'])

    def test_staged_blob_is_checked_even_when_the_working_file_is_fixed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '--quiet', directory], check=True)
            source = root / 'example.py'
            source.write_text('invalid python !!!\n')
            subprocess.run(['git', 'add', 'example.py'], cwd=root, check=True)
            source.write_text('print("valid working copy")\n')
            with patch.object(hooks, 'ROOT', root):
                with self.assertRaises(SyntaxError):
                    hooks.staged()
            self.assertEqual(source.read_text(), 'print("valid working copy")\n')

    def test_failed_snapshot_check_removes_worktree_and_preserves_dirty_work(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '--quiet', directory], check=True)
            (root / 'scripts').mkdir()
            (root / 'scripts/verify.py').write_text('# fixture\n')
            (root / '.github/scripts').mkdir(parents=True)
            shutil.copy(ROOT / '.github/scripts/ci_plan.py', root / '.github/scripts/ci_plan.py')
            source = root / 'source.py'
            source.write_text('committed = True\n')
            subprocess.run(['git', 'add', '.'], cwd=root, check=True)
            subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                            'commit', '--quiet', '-m', 'fixture'], cwd=root, check=True)
            oid = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
            source.write_text('private = True\n')
            real_run = hooks.run

            def failing(*args, **kwargs):
                if args[0] == 'npm':
                    self.assertEqual((kwargs['cwd'] / 'source.py').read_text(), 'committed = True\n')
                    raise subprocess.CalledProcessError(1, args)
                return real_run(*args, **kwargs)

            with patch.object(hooks, 'ROOT', root), patch.object(hooks, 'run', side_effect=failing):
                with self.assertRaises(subprocess.CalledProcessError):
                    hooks.push([f'refs/heads/test {oid} refs/heads/test {"0"*40}'])
            self.assertEqual(source.read_text(), 'private = True\n')
            worktrees = subprocess.check_output(['git', 'worktree', 'list', '--porcelain'], cwd=root, text=True)
            self.assertEqual(worktrees.count('worktree '), 1)

    def test_deleting_a_remote_ref_does_not_start_verification(self):
        self.assertEqual(list(hooks.tips([f'(delete) {"0"*40} refs/heads/old {"a"*40}'])), [])


    def test_installation_does_not_replace_an_existing_hook_configuration(self):
        installer = load('install_hooks')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '--quiet', directory], check=True)
            subprocess.run(['git', 'config', '--local', 'core.hooksPath', 'custom-hooks'], cwd=root, check=True)
            with patch.object(installer, 'ROOT', root):
                with self.assertRaises(RuntimeError):
                    installer.install()
            value = subprocess.check_output(['git', 'config', '--get', 'core.hooksPath'], cwd=root, text=True)
            self.assertEqual(value.strip(), 'custom-hooks')
