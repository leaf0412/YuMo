import unittest
from pathlib import Path

from conftest_helpers import load_daemon_module, add_fixtures_to_path, FIXTURES_DIR


class CheckDepsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.daemon = load_daemon_module()

    def test_all_installed(self):
        spec_path = FIXTURES_DIR / "specs" / "mimo_like.yaml"
        result = self.daemon.check_custom_dependencies(str(spec_path))
        self.assertTrue(result["all_installed"])
        self.assertEqual(result["missing"], [])
        self.assertIn("fake_asr_pkg", result["installed"])

    def test_reports_missing(self):
        bad_spec = Path("/tmp/test_bad_spec.yaml")
        bad_spec.write_text("""
schema_version: 1
id: bad
name: Bad
size_mb: 1
languages: { en: English }
speed: 5
accuracy: 5
python_module: definitely_nonexistent_pkg_xyz
pip_packages: [definitely_nonexistent_pkg_xyz>=99.0]
load:
  function: definitely_nonexistent_pkg_xyz.load
  kwargs: {}
""")
        try:
            result = self.daemon.check_custom_dependencies(str(bad_spec))
            self.assertFalse(result["all_installed"])
            self.assertIn("definitely_nonexistent_pkg_xyz", result["missing"][0])
        finally:
            bad_spec.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main()
