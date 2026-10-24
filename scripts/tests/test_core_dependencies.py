import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "core_dependencies", Path(__file__).parents[1] / "check_core_dependencies.py")
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class CoreDependencyTests(unittest.TestCase):
    def test_graphics_localization_and_transport_are_rejected_including_repeated_nodes(self):
        tree = "\n".join(f"{name} v1.0 (*)" for name in
                         ["egui", "eframe", "winit", "rfd", "muda", "egui",
                          "fluent-bundle", "fluent-syntax", "i18n-embed", "i18n-embed-fl",
                          "tokio", "tokio-util", "rmcp", "rmcp-macros", "interprocess"])
        self.assertEqual(checker.forbidden_dependencies(tree),
                         sorted(["egui", "eframe", "winit", "rfd", "muda", "fluent-bundle",
                                 "fluent-syntax", "i18n-embed", "i18n-embed-fl",
                                 "tokio", "tokio-util", "rmcp", "rmcp-macros", "interprocess"]))

    def test_typst_rendering_and_unicode_dependencies_are_allowed(self):
        self.assertEqual(checker.forbidden_dependencies(
            "typstographe-core v0.1.0 (/a path/core)\ntypst-ide v0.15.1\n"
            "resvg v0.47.0\nunic-langid v0.9\n"), [])

    def test_future_localization_and_transport_subcrates_are_also_rejected(self):
        self.assertEqual(checker.forbidden_dependencies(
            "fluent-new-module v1.0\ni18n-embed-new v1.0\ntokio-new v1.0\nrmcp-new v1.0\n"),
            ["fluent-new-module", "i18n-embed-new", "rmcp-new", "tokio-new"])
