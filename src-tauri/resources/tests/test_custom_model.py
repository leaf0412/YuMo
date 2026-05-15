import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from conftest_helpers import (
    add_fixtures_to_path,
    load_daemon_module,
    load_shared_module,
    FIXTURES_DIR,
)


class CheckDepsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.shared = load_shared_module()

    def test_all_installed(self):
        spec_path = FIXTURES_DIR / "specs" / "mimo_like.yaml"
        result = self.shared.check_custom_dependencies(str(spec_path))
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
            result = self.shared.check_custom_dependencies(str(bad_spec))
            self.assertFalse(result["all_installed"])
            self.assertIn("definitely_nonexistent_pkg_xyz", result["missing"][0])
        finally:
            bad_spec.unlink(missing_ok=True)


class InstallDepsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.shared = load_shared_module()

    def test_install_invokes_pip_with_correct_packages(self):
        captured = {}
        def fake_run(cmd, **kwargs):
            captured["cmd"] = cmd
            class Result:
                returncode = 0
                stdout = "Successfully installed fake_asr_pkg-0.1.0\n"
                stderr = ""
            return Result()

        original = self.shared.subprocess.run
        self.shared.subprocess.run = fake_run
        try:
            spec_path = FIXTURES_DIR / "specs" / "mimo_like.yaml"
            result = self.shared.install_custom_dependencies(str(spec_path))
        finally:
            self.shared.subprocess.run = original

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

        original = self.shared.subprocess.run
        self.shared.subprocess.run = fake_run
        try:
            spec_path = FIXTURES_DIR / "specs" / "mimo_like.yaml"
            result = self.shared.install_custom_dependencies(str(spec_path))
        finally:
            self.shared.subprocess.run = original

        self.assertFalse(result["success"])
        self.assertIn("could not find", result["error"])


class DownloadFunctionVariantTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.shared = load_shared_module()

    def test_function_variant_writes_sidecar(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            custom_dir = tmp / "custom_models"
            custom_dir.mkdir()
            voiceink_dir = tmp / "models"
            voiceink_dir.mkdir()
            spec_path = custom_dir / "stub.yaml"
            spec_path.write_text((FIXTURES_DIR / "specs" / "mimo_like.yaml").read_text())

            result = self.shared.download_custom_model(
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
        cls.shared = load_shared_module()

    def test_hf_repos_variant_calls_snapshot_download(self):
        calls = []
        def fake_snapshot(repo_id, local_dir, **kw):
            calls.append((repo_id, local_dir))
            Path(local_dir).mkdir(parents=True, exist_ok=True)
            return local_dir

        # Monkeypatch huggingface_hub at module level — shared.download_custom_model
        # imports it lazily, so patching the global module works.
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

                result = self.shared.download_custom_model(
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


class LoadCustomTest(unittest.TestCase):
    """Verifies the daemon path: `load_custom_model` is exposed on the
    daemon module (re-exported from custom_model_shared) and is what
    the `action: "load"` branch invokes when provider=='custom'."""

    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.daemon = load_daemon_module()
        cls.shared = load_shared_module()

    def _setup(self, tmp):
        tmp = Path(tmp)
        custom_dir = tmp / "custom_models"; custom_dir.mkdir()
        voiceink_dir = tmp / "models"; voiceink_dir.mkdir()
        spec_path = custom_dir / "stub.yaml"
        spec_path.write_text((FIXTURES_DIR / "specs" / "mimo_like.yaml").read_text())
        return spec_path, voiceink_dir, custom_dir

    def test_load_after_download_succeeds(self):
        with tempfile.TemporaryDirectory() as tmp:
            spec_path, voiceink_dir, custom_dir = self._setup(tmp)
            self.shared.download_custom_model(
                str(spec_path), str(voiceink_dir), str(custom_dir)
            )
            model = self.daemon.load_custom_model(
                str(spec_path), str(voiceink_dir), str(custom_dir)
            )
            self.assertEqual(model._daemon_model_type, "custom")
            self.assertEqual(model.precision, "int4")

    def test_load_without_download_errors(self):
        with tempfile.TemporaryDirectory() as tmp:
            spec_path, voiceink_dir, custom_dir = self._setup(tmp)
            with self.assertRaises(FileNotFoundError) as ctx:
                self.daemon.load_custom_model(
                    str(spec_path), str(voiceink_dir), str(custom_dir)
                )
            self.assertIn("paths.json", str(ctx.exception))


class TranscribeCustomTest(unittest.TestCase):
    """Verifies the daemon's transcribe() routes to the custom branch
    when the loaded model has _daemon_model_type == 'custom'."""

    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.daemon = load_daemon_module()
        cls.shared = load_shared_module()

    def _load_stub(self, tmp):
        tmp = Path(tmp)
        custom_dir = tmp / "custom_models"; custom_dir.mkdir()
        voiceink_dir = tmp / "models"; voiceink_dir.mkdir()
        spec_path = custom_dir / "stub.yaml"
        spec_path.write_text((FIXTURES_DIR / "specs" / "mimo_like.yaml").read_text())
        self.shared.download_custom_model(
            str(spec_path), str(voiceink_dir), str(custom_dir)
        )
        return self.daemon.load_custom_model(
            str(spec_path), str(voiceink_dir), str(custom_dir)
        )

    def test_dispatches_to_custom_branch(self):
        with tempfile.TemporaryDirectory() as tmp:
            model = self._load_stub(tmp)
            text = self.daemon.transcribe(model, "/path/audio.wav", language="zh")
            self.assertEqual(text, "[stub:int4:zh] /path/audio.wav")

    def test_uses_configured_method_and_param_names(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            custom_dir = tmp / "custom_models"; custom_dir.mkdir()
            voiceink_dir = tmp / "models"; voiceink_dir.mkdir()
            yaml_text = (FIXTURES_DIR / "specs" / "mimo_like.yaml").read_text()
            yaml_text += "\ntranscribe_method: transcribe\nlanguage_param: language\n"
            spec_path = custom_dir / "stub.yaml"
            spec_path.write_text(yaml_text)
            self.shared.download_custom_model(
                str(spec_path), str(voiceink_dir), str(custom_dir)
            )
            model = self.daemon.load_custom_model(
                str(spec_path), str(voiceink_dir), str(custom_dir)
            )
            text = self.daemon.transcribe(model, "/x.wav", language="en")
            self.assertIn("[stub:int4:en]", text)


class WorkerProtocolTest(unittest.TestCase):
    """End-to-end: spawn the worker as a child process and verify the
    stdin/stdout protocol that the Rust side relies on."""

    @classmethod
    def setUpClass(cls):
        add_fixtures_to_path()
        cls.worker_path = Path(__file__).resolve().parent.parent / "custom_model_worker.py"
        cls.fixtures_path = FIXTURES_DIR

    def _run_worker(self, cmd_dict):
        env = {
            **__import__("os").environ,
            "PYTHONPATH": (
                f"{self.worker_path.parent}:{self.fixtures_path}"
            ),
        }
        proc = subprocess.run(
            ["python3", str(self.worker_path)],
            input=json.dumps(cmd_dict),
            capture_output=True,
            text=True,
            timeout=30,
            env=env,
        )
        return proc

    def test_check_deps_success_and_zero_exit(self):
        spec_path = self.fixtures_path / "specs" / "mimo_like.yaml"
        proc = self._run_worker({"action": "check_deps", "spec_path": str(spec_path)})
        self.assertEqual(proc.returncode, 0, msg=proc.stderr)
        last_line = [l for l in proc.stdout.splitlines() if l.strip()][-1]
        resp = json.loads(last_line)
        self.assertTrue(resp["ok"])
        self.assertTrue(resp["all_installed"])

    def test_unknown_action_fails_with_nonzero_exit(self):
        proc = self._run_worker({"action": "nope"})
        self.assertNotEqual(proc.returncode, 0)
        last_line = [l for l in proc.stdout.splitlines() if l.strip()][-1]
        resp = json.loads(last_line)
        self.assertFalse(resp["ok"])
        self.assertIn("nope", resp["error"])


if __name__ == "__main__":
    unittest.main()
