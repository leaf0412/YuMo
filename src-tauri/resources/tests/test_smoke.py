import unittest
from conftest_helpers import load_daemon_module, add_fixtures_to_path


class SmokeTest(unittest.TestCase):
    def test_daemon_loads(self):
        m = load_daemon_module()
        self.assertTrue(hasattr(m, "send_response"))

    def test_stub_pkg_importable(self):
        add_fixtures_to_path()
        import fake_asr_pkg
        asr = fake_asr_pkg.load_asr(precision="int4")
        self.assertEqual(asr.transcribe("/x.wav", language="zh"),
                         "[stub:int4:zh] /x.wav")


if __name__ == "__main__":
    unittest.main()
