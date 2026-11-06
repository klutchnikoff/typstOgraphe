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
                     "crates/core/assets/vendor/README.md", "crates/core/types/article/type.yaml",
                     "docs/schemas/model-manifest.schema.json", "docs/schemas/type-definition.schema.json", "docs/proposals/b1/article-authorship.yaml"]:
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
            self.assertEqual(output.read_text(), "code=true\ndependencies=false\nrust=false\nrecipes=\n")

    def test_pull_requests_run_recipes_of_the_edited_areas_only(self):
        root = SCRIPT.parents[2]
        self.assertEqual(planner.browser_recipes(["app/src/side-rail.ts"], root),
                         ["tests/browser/sidebar.spec.ts", "tests/browser/splitter.spec.ts"])
        self.assertEqual(planner.browser_recipes(["app/src/tables.ts", "app/tests/browser/tables.spec.ts"], root),
                         ["tests/browser/tables.spec.ts"])
        self.assertIn("tests/browser/bands.spec.ts",
                      planner.browser_recipes(["patches/@tiptap+y-tiptap+3.0.10.patch"], root))
        self.assertEqual(planner.browser_recipes(["crates/core/types/article/type.yaml"], root),
                         ["tests/browser/article.spec.ts", "tests/browser/bibliography.spec.ts",
                          "tests/browser/document-types.spec.ts", "tests/browser/metadata.spec.ts"])
        self.assertEqual(planner.browser_recipes(["crates/core/structures/authorship.yaml"], root),
                         planner.browser_recipes(["crates/core/types/article/type.yaml"], root))
        self.assertIn("tests/browser/model-options.spec.ts", planner.browser_recipes(["crates/core/templates/article-working.model.yaml"], root))
        # A shared helper selects each recipe importing it.
        self.assertIn("tests/browser/title-dialogs.spec.ts",
                      planner.browser_recipes(["app/tests/browser/record-dialog-controls.ts"], root))
        for paths in [None, [], ["README.md"], ["crates/core/src/lib.rs"], ["app/src/main.ts"]]:
            self.assertEqual(planner.browser_recipes(paths, root), [], paths)

    def test_date_contract_and_wrapper_changes_run_metadata_recipes(self):
        for source in ["crates/core/src/dates.rs", "crates/core/templates/capabilities/dates.typ"]:
            self.assertEqual(planner.browser_recipes([source]), [
                "tests/browser/metadata.spec.ts", "tests/browser/structured-metadata.spec.ts"])

    def test_shared_bibliography_engine_selects_the_bibliography_recipe(self):
        for source in ["crates/bibliography/src/import.rs", "crates/bibliography/src/search.rs",
                       "app/src/bibliography-manager.ts", "app/src/bibliography-fields.ts",
                       "crates/core/src/bibliography.rs", "crates/core/src/content/api/bibliography.rs"]:
            self.assertEqual(planner.browser_recipes([source]),
                             ["tests/browser/bibliography.spec.ts"])

    def test_resource_contract_changes_cover_metadata_and_recovery(self):
        for source in ["crates/core/src/resources.rs", "app/src/metadata-resource.ts"]:
            recipes = planner.browser_recipes([source])
            self.assertIn("tests/browser/structured-resources.spec.ts", recipes)
            self.assertIn("tests/browser/recovery-resume.spec.ts", recipes)

    def test_every_browser_area_names_existing_sources_and_recipes(self):
        root = SCRIPT.parents[2]
        files = subprocess.check_output(["git", "ls-files"], cwd=root, text=True).splitlines()
        for patterns, stems in planner.BROWSER_AREAS:
            for pattern in patterns:
                self.assertTrue(any(planner.fnmatch(name, pattern) for name in files), pattern)
            for stem in stems:
                self.assertTrue((root / planner.RECIPES / f"{stem}.spec.ts").is_file(), stem)


if __name__ == "__main__":
    unittest.main()
