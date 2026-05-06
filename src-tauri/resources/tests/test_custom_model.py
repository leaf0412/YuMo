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


import subprocess

class InstallDepsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.daemon = load_daemon_module()

    def test_install_invokes_pip_with_correct_packages(self):
        captured = {}
        def fake_run(cmd, **kwargs):
            captured["cmd"] = cmd
            class Result:
                returncode = 0
                stdout = "Successfully installed fake_asr_pkg-0.1.0\n"
                stderr = ""
            return Result()

        original = self.daemon.subprocess.run
        self.daemon.subprocess.run = fake_run
        try:
            spec_path = FIXTURES_DIR / "specs" / "mimo_like.yaml"
            result = self.daemon.install_custom_dependencies(str(spec_path))
        finally:
            self.daemon.subprocess.run = original

        self.assertTrue(result["success"])
        self.assertIn("pip", captured["cmd"])
        self.assertIn("install", captured["cmd"])
        self.assertIn("fake_asr_pkg", captured["cmd"])

    def test_install_failure_propagates_stderr(self):
        def fake_run(cmd, **kwargs):
            class Result:
                returncode = 1
                stdout = ""
                stderr = "ERROR: could not find package\n"
            return Result()

        original = self.daemon.subprocess.run
        self.daemon.subprocess.run = fake_run
        try:
            spec_path = FIXTURES_DIR / "specs" / "mimo_like.yaml"
            result = self.daemon.install_custom_dependencies(str(spec_path))
        finally:
            self.daemon.subprocess.run = original

        self.assertFalse(result["success"])
        self.assertIn("could not find", result["error"])


import json
import tempfile

class DownloadFunctionVariantTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.daemon = load_daemon_module()

    def test_function_variant_writes_sidecar(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            custom_dir = tmp / "custom_models"
            custom_dir.mkdir()
            voiceink_dir = tmp / "models"
            voiceink_dir.mkdir()
            spec_path = custom_dir / "stub.yaml"
            spec_path.write_text((FIXTURES_DIR / "specs" / "mimo_like.yaml").read_text())

            result = self.daemon.download_custom_model(
                str(spec_path),
                voiceink_models_dir=str(voiceink_dir),
                custom_models_dir=str(custom_dir),
            )
            self.assertTrue(result["success"])

            sidecar = custom_dir / ".cache" / "stub-mimo.paths.json"
            self.assertTrue(sidecar.exists())
            paths = json.loads(sidecar.read_text())
            self.assertIn("asr_dir", paths)
            self.assertIn("tokenizer_dir", paths)
            self.assertTrue(Path(paths["asr_dir"]).exists())
            self.assertTrue(Path(paths["tokenizer_dir"]).exists())


class DownloadHfReposVariantTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.daemon = load_daemon_module()

    def test_hf_repos_variant_calls_snapshot_download(self):
        calls = []
        def fake_snapshot(repo_id, local_dir, **kw):
            calls.append((repo_id, local_dir))
            Path(local_dir).mkdir(parents=True, exist_ok=True)
            return local_dir

        # Monkeypatch the daemon's huggingface_hub
        import huggingface_hub
        original = huggingface_hub.snapshot_download
        huggingface_hub.snapshot_download = fake_snapshot
        try:
            with tempfile.TemporaryDirectory() as tmp:
                tmp = Path(tmp)
                custom_dir = tmp / "custom_models"; custom_dir.mkdir()
                voiceink_dir = tmp / "models"; voiceink_dir.mkdir()
                spec_path = custom_dir / "hf.yaml"
                spec_path.write_text((FIXTURES_DIR / "specs" / "hf_repos.yaml").read_text())

                result = self.daemon.download_custom_model(
                    str(spec_path),
                    voiceink_models_dir=str(voiceink_dir),
                    custom_models_dir=str(custom_dir),
                )
                self.assertTrue(result["success"])
                self.assertEqual(len(calls), 1)
                self.assertEqual(calls[0][0], "foo/bar")
                self.assertIn("asr_dir", result["paths"])
        finally:
            huggingface_hub.snapshot_download = original


if __name__ == "__main__":
    unittest.main()
