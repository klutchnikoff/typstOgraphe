import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[2] / ".github/scripts/ci_plan.py"
spec = importlib.util.spec_from_file_location("ci_plan", SCRIPT)
planner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(planner)


class CIPlanTests(unittest.TestCase):
    def test_documentation_and_no_change_do_not_start_expensive_checks(self):
        for files in [[], ["README.md", "app/README.md", "ROADMAP.md", "docs/cli.md", "docs/mockups/editor.png"]]:
            self.assertEqual(planner.plan(files), {"code": False, "dependencies": False})

    def test_all_source_families_and_new_unknown_paths_are_checked(self):
        for name in ["app/src/main.ts", "crates/core/src/lib.rs", "scripts/check_file_length.py",
                     "assets/font.ttf", "examples/article.typsto", ".cargo/config.toml",
                     "new-component/config.json", "docs/commands.json", "app/src/embedded.md",
                     "crates/core/assets/vendor/README.md"]:
            self.assertTrue(planner.plan([name])["code"], name)

    def test_manifests_locks_and_ci_changes_keep_dependency_audits(self):
        for name in ["Cargo.lock", "crates/core/Cargo.toml", "package-lock.json",
                     "app/package.json", "deny.toml", ".cargo/config.toml",
                     ".github/workflows/ci.yml", ".github/scripts/ci_plan.py"]:
            self.assertEqual(planner.plan([name]), {"code": True, "dependencies": True}, name)
        self.assertFalse(planner.plan(["app/src/main.ts"])["dependencies"])

    def test_unknown_or_initial_history_never_skips_checks(self):
        self.assertEqual(planner.plan(None), {"code": True, "dependencies": True})
        for base in ["", "0" * 40, "not-a-commit", "--help"]:
            self.assertIsNone(planner.changed_paths(base))

    def test_unavailable_commit_and_remote_conservatively_run_all_checks(self):
        with tempfile.TemporaryDirectory() as root:
            subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
            paths = planner.changed_paths("1" * 40, root=root)
            self.assertIsNone(paths)
            self.assertEqual(planner.plan(paths), {"code": True, "dependencies": True})

    def test_renaming_source_into_documentation_keeps_the_deletion_visible(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.DEVNULL)

            git("init")
            source = root / "source.txt"
            source.write_text("source\n", encoding="utf-8")
            git("add", ".")
            git("-c", "user.name=CI test", "-c", "user.email=ci@example.invalid", "commit", "-m", "base")
            base = git("rev-parse", "HEAD").decode().strip()
            (root / "docs").mkdir()
            source.rename(root / "docs/source.md")
            git("add", ".")
            git("-c", "user.name=CI test", "-c", "user.email=ci@example.invalid", "commit", "-m", "rename")
            paths = planner.changed_paths(base, root=root)
            self.assertEqual(set(paths), {"source.txt", "docs/source.md"})
            self.assertTrue(planner.plan(paths)["code"])
            output = root / "output"
            env = {**os.environ, "GITHUB_OUTPUT": str(output)}
            result = subprocess.check_output(
                ["python3", str(SCRIPT), "--base", base], cwd=root, env=env)
            self.assertTrue(json.loads(result)["code"])
            self.assertEqual(output.read_text(), "code=true\ndependencies=false\n")


if __name__ == "__main__":
    unittest.main()
