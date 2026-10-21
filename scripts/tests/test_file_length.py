import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("check_file_length", Path(__file__).parents[1] / "check_file_length.py")
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class ProductionLengthTests(unittest.TestCase):
    def test_test_function_in_the_middle_does_not_hide_later_production(self):
        source = 'impl Item {\n    #[cfg(test)]\n    fn helper() {\n        if true {\n            call();\n        }\n    }\n    fn real() {}\n}\n'
        self.assertEqual(checker.production_lines(source), 3)

    def test_test_module_and_coverage_attribute_are_excluded(self):
        source = 'fn real() {}\n#[cfg(test)]\n#[cfg_attr(coverage_nightly, coverage(off))]\nmod tests {\n    #[test]\n    fn scenario() {}\n}\nfn after() {}\n'
        self.assertEqual(checker.production_lines(source), 2)

    def test_external_test_module_does_not_hide_following_production(self):
        self.assertEqual(checker.production_lines('#[cfg(test)]\nmod tests;\nfn main() {}\n'), 1)

    def test_tests_files_are_excluded_but_long_production_is_reported(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'tests').mkdir()
            (root / 'tests/case.rs').write_text('test\n' * 10)
            (root / 'tests.rs').write_text('test\n' * 10)
            source = root / 'real.rs'
            source.write_text('production\n' * 3)
            self.assertEqual(checker.violations(root, 2), [(source, 3)])
            self.assertEqual(checker.violations(root, 3), [])

    def test_cfg_enum_variant_does_not_hide_the_rest_of_the_file(self):
        source = "enum Task {\n    #[cfg(test)]\n    Barrier(Sender<()>),\n    Real,\n}\nfn main() {}\n"
        self.assertEqual(checker.production_lines(source), 4)

    def test_empty_test_function_does_not_hide_the_next_function(self):
        self.assertEqual(checker.production_lines("#[cfg(test)]\nfn helper() {}\nfn real() {}\n"), 1)
