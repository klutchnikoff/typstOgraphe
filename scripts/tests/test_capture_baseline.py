"""Exercise the comparison guard with real, tiny PNGs; no GUI is needed."""
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zlib

spec = importlib.util.spec_from_file_location("capture_baseline", Path(__file__).parents[1] / "capture_baseline.py")
baseline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(baseline)


def png(path, color):
    def chunk(name, data):
        return struct.pack(">I", len(data)) + name + data + struct.pack(">I", zlib.crc32(name + data))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(b"\x00" + bytes(color))) + chunk(b"IEND", b""))


class ComparisonTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.reference = Path(self.directory.name) / "reference"
        self.current = Path(self.directory.name) / "current"
        self.reference.mkdir()
        self.current.mkdir()
        for name in baseline.SCENARIOS:
            for folder in [self.reference, self.current]:
                png(folder / f"{name}.png", [255, 255, 255, 255])
        self.manifest = self.reference / "manifest.json"
        self.manifest.write_text(json.dumps({"version": 1, "environment": baseline.environment(), "images": {
            name: baseline.image_record(self.reference / f"{name}.png") for name in baseline.SCENARIOS
        }}))
        self.original = self.manifest.read_bytes()

    def test_identical_captures_match(self):
        self.assertEqual(baseline.compare(self.reference, self.current, baseline.environment()), [])

    def test_changed_pixel_fails_without_altering_reference(self):
        png(self.current / "editor.png", [0, 0, 0, 255])
        self.assertTrue(any("editor" in error for error in baseline.compare(self.reference, self.current, baseline.environment())))
        self.assertEqual(self.manifest.read_bytes(), self.original)

    def test_missing_capture_fails(self):
        (self.current / "spelling.png").unlink()
        self.assertTrue(any("spelling" in error for error in baseline.compare(self.reference, self.current, baseline.environment())))

    def test_different_environment_fails(self):
        environment = {**baseline.environment(), "system": "another-system"}
        self.assertTrue(any("environment" in error for error in baseline.compare(self.reference, self.current, environment)))

    def test_capture_accepts_the_fresh_directory_created_by_main(self):
        directory = self.current / "fresh"
        directory.mkdir()
        def fake_run(binary, arguments, log):
            png(Path(arguments[1]), [255, 255, 255, 255])
            log.write_text("capture")
        with patch.object(baseline, "fixtures", return_value=(Path("editor.typsto"), Path("recovery"))), patch.object(baseline, "run", side_effect=fake_run):
            baseline.capture(Path("application"), directory)
        self.assertEqual({path.stem for path in directory.glob("*.png")}, set(baseline.SCENARIOS))
