"""Stub ASR package used by custom-model daemon tests."""
from pathlib import Path


def download_models(precision: str, local_root, audio_tokenizer_repo: str = "fake/tok"):
    """Pretend to download. Create empty dirs and return their paths as a tuple."""
    local_root = Path(local_root)
    asr_dir = local_root / f"fake-asr-{precision}"
    tok_dir = local_root / "fake-tokenizer"
    asr_dir.mkdir(parents=True, exist_ok=True)
    tok_dir.mkdir(parents=True, exist_ok=True)
    return (asr_dir, tok_dir)


class StubASR:
    def __init__(self, precision, audio_tokenizer_dir):
        self.precision = precision
        self.audio_tokenizer_dir = audio_tokenizer_dir

    def transcribe(self, audio, language: str = "auto") -> str:
        return f"[stub:{self.precision}:{language}] {audio}"


def load_asr(precision: str = "bf16", audio_tokenizer_dir=None, local_root="models", **_):
    return StubASR(precision, audio_tokenizer_dir)
