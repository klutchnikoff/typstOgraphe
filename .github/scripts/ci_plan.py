"""Conservative CI scope: unknown history runs checks; documentation alone does not."""
import argparse
from fnmatch import fnmatch
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def plan(paths):
    if paths is None:
        return {"code": True, "dependencies": True}
    code = dependencies = False
    for name in paths:
        path = Path(name)
        dependency = (path.name in {"Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "deny.toml"}
                      or name.startswith((".github/", ".cargo/")))
        # Markdown embedded in a source/resource directory may be executable
        # input. Only the known documentation locations are skipped.
        documentation = (name.startswith("docs/") or name == "app/README.md"
                         or (path.parent == Path(".") and path.suffix == ".md"))
        # This generated catalogue is compared with the engine by the tests.
        code |= dependency or not documentation or name in {
            "docs/commands.json", "docs/schemas/type-definition.schema.json",
            "docs/schemas/model-manifest.schema.json",
            "docs/proposals/b1/article-authorship.yaml",
        }
        dependencies |= dependency
    return {"code": code, "dependencies": dependencies}


RECIPES = Path("app/tests/browser")

# Targeted graphical recipes used by the local pre-push check. Full graphical
# verification is explicit; neither routine PR nor main checks launch browsers.
BROWSER_AREAS = (
    (("app/src/mathematics.ts", "app/src/cross-reference-index.ts", "crates/core/src/content/mathematics*",
      "crates/core/templates/capabilities/mathematics.typ", "crates/core/templates/packages/mitex-*/**"), ("mathematics",)),
    (("app/src/*reference*.ts", "app/src/*anchor*.ts", "crates/core/src/content/references*",
      "crates/core/templates/capabilities/cross-references.typ"), ("cross-references",)),
    (("crates/core/src/resources.rs", "app/src/metadata-resource.ts", "app/src/metadata-commands.ts"),
     ("structured-resources", "metadata", "structured-metadata", "recovery-resume")),
    (("crates/core/src/dates.rs", "crates/core/templates/capabilities/dates.typ"),
     ("metadata", "structured-metadata")),
    (("crates/core/src/document_types/model_options*", "crates/core/templates/*.model.yaml",
      "app/src/model-option-control.ts", "app/src/document-options.ts"),
     ("model-options", "metadata", "bibliography", "structured-resources")),
    (("crates/core/types/*/type.yaml", "crates/core/structures/*.yaml"),
     ("document-types", "metadata", "article", "bibliography")),
    (("app/src/side-rail.ts", "app/src/shell-layout.ts", "app/src/resizable-separator.ts"),
     ("sidebar", "splitter")),
    (("app/src/bibliography-*.ts", "app/src/citation-*.ts",
      "crates/bibliography/**", "crates/core/src/bibliography.rs",
      "crates/core/src/content/api/bibliography*.rs"), ("bibliography",)),
    (("app/src/comparison-*.ts",), ("comparison",)),
    (("app/src/version*", "app/src/versions*"), ("versions",)),
    (("app/src/preview*.ts", "app/src/render-queue.ts", "app/src/proposal-preview.ts"), ("preview",)),
    (("app/src/preferences-*.ts", "app/src/shell-preferences.ts"), ("preferences",)),
    (("app/src/*recover*", "app/src/private-*.ts", "app/src/draft-*.ts", "app/src/form-drafts.ts"),
     ("recovery-resume", "draft-conservation")),
    (("app/src/metadata-*.ts", "app/src/shell-metadata.ts", "app/src/record-card.ts",
      "app/src/sortable-cards.ts", "app/src/form-dialog.ts", "app/src/form-field.ts", "app/src/parameter-fields.ts"),
     ("metadata", "metadata-dialogs", "structured-metadata", "record-cards", "abstract-dialogs",
      "title-dialogs")),
    (("app/src/table-*.ts", "app/src/cell-paste.ts"), ("tables",)),
    (("app/src/inline-object*.ts",), ("inline-objects", "badge-spacing", "inline-caret")),
    # Editing, selection and the patched Yjs binding shared by every rich zone.
    (("app/src/paragraph-*.ts", "app/src/selection-edits.ts", "app/src/text-edits.ts",
      "app/src/rich-zone.ts", "app/src/shared-inline-marks.ts", "app/src/editing*.ts",
      "app/src/*-selection.ts", "app/src/immediate-edit.ts", "app/src/clipboard-content.ts",
      "patches/*"),
     ("paragraph-flow", "paragraph-join", "persisted-paragraph-input", "selection-input",
      "input-boundaries", "rich-zone", "bands", "editor", "editing-owner")),
    (("app/src/command*.ts", "app/src/public-commands.ts", "app/src/shell-commands.ts",
      "app/src/menus.ts", "app/src/shell-menus.ts"),
     ("command-availability", "palette", "editor-palette")),
    (("app/src/document-type-picker.ts", "app/src/new-document-dialog.ts",
      "app/src/document-options.ts", "app/src/welcome-*.ts", "app/src/work-list.ts"),
     ("document-types", "welcome")),
    (("app/src/widget*.ts",), ("widget-presentation", "components")),
    (("app/src/document-whitespace.ts",), ("whitespace",)),
    (("app/src/keyboard-navigation.ts", "app/src/editor-focus.ts", "app/src/editor-targets.ts",
      "app/src/editor-reveal.ts"),
     ("keyboard-navigation", "band-focus", "band-return")),
    (("app/src/band-layout.ts",), ("bands", "inline-bands")),
    (("app/src/synchronization-*.ts", "app/src/provider.ts", "app/src/browser-transport.ts"),
     ("synchronization", "backward-sync")),
    (("app/src/proofing.ts", "app/src/assistance.ts"), ("structured-proofing",)),
    (("app/src/structured-commands.ts", "app/src/object-commands.ts", "app/src/resource-commands.ts",
      "app/src/formatting.ts"),
     ("structured-objects", "structured-resources", "structured-formatting", "insertion")),
    (("app/src/close-decision.ts", "app/src/closure-*.ts", "app/src/shell-documents.ts"),
     ("editor",)),
)


def browser_recipes(paths, root=Path(".")):
    """Targeted recipes, relative to app/. Unknown history keeps @smoke only."""
    root = Path(root)
    specs = {path.name.removesuffix(".spec.ts"): path
             for path in (root / RECIPES).glob("*.spec.ts")}
    selected = set()
    for name in paths or ():
        path = Path(name)
        if path.parent == RECIPES and path.suffix == ".ts":
            stem = path.name.removesuffix(".spec.ts")
            if stem != path.name:
                selected.add(stem)
            else:
                # A shared helper: every recipe importing it.
                helper = f"from './{path.stem}'"
                selected |= {stem for stem, spec in specs.items()
                             if helper in spec.read_text(encoding="utf-8")}
        if path.parent == Path("app/src"):
            selected.add(path.name.split(".")[0])
        for patterns, stems in BROWSER_AREAS:
            if any(fnmatch(name, pattern) for pattern in patterns):
                selected.update(stems)
    return sorted((RECIPES / f"{stem}.spec.ts").relative_to("app").as_posix()
                  for stem in selected if stem in specs)


def changed_paths(base, head="HEAD", root=None):
    if not re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", base or "") or set(base) == {"0"}:
        return None

    def git(*args):
        return subprocess.run(["git", *args], cwd=root, check=True,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    try:
        try:
            git("cat-file", "-e", f"{base}^{{commit}}")
        except subprocess.CalledProcessError:
            git("fetch", "--no-tags", "--depth=1", "origin", base)
        # Renaming a source into docs must still report its source-side deletion.
        data = git("diff", "--name-only", "--no-renames", "-z", base, head).stdout
        return [os.fsdecode(name) for name in data.split(b"\0") if name]
    except subprocess.CalledProcessError:
        print("Historique incomplet : toutes les vérifications seront exécutées.", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    paths = None if args.force else changed_paths(args.base)
    result = plan(paths)
    result["rust"] = paths is None or any(
        name.endswith((".rs", "Cargo.toml", "Cargo.lock"))
        or name.startswith((".cargo/", ".github/")) for name in paths)
    recipes = " ".join(browser_recipes(paths))
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a", encoding="utf-8") as stream:
            for key, value in result.items():
                stream.write(f"{key}={str(value).lower()}\n")
            stream.write(f"recipes={recipes}\n")
    print(json.dumps({**result, "recipes": recipes}))


if __name__ == "__main__":
    main()
