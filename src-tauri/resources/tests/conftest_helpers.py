"""Shared helpers for daemon tests."""
import importlib.util
import os
import sys
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parent
RESOURCES_DIR = TESTS_DIR.parent
FIXTURES_DIR = TESTS_DIR / "fixtures"


def load_daemon_module():
    """Import mlx_funasr_daemon.py without requiring mlx to be installed."""
    spec_path = RESOURCES_DIR / "mlx_funasr_daemon.py"
    spec = importlib.util.spec_from_file_location("daemon_under_test", spec_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def add_fixtures_to_path():
    """Make fixtures/fake_asr_pkg importable in tests."""
    p = str(FIXTURES_DIR)
    if p not in sys.path:
        sys.path.insert(0, p)
