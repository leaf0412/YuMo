#!/usr/bin/env python3
"""
MLX FunASR Daemon - Long-running process for speech recognition on Apple Silicon
Communicates via stdin/stdout using JSON protocol

Usage:
    python mlx_funasr_daemon.py

Commands (JSON format via stdin):
    {"action": "ping"}                                  - Health check
    {"action": "check_dependencies"}                    - Check required packages
    {"action": "check_model", "model": "..."}           - Check if model downloaded
    {"action": "load", "model": "..."}                  - Load model into memory
    {"action": "transcribe", "audio": "/path/to/file", "max_tokens": 8192}  - Transcribe audio
    {"action": "unload"}                                - Unload model from memory
    {"action": "quit"}                                  - Shutdown daemon
"""

import sys
import os
import json
import gc
import time

# All model cache lives under ~/.voiceink/models
_mlx_cache = os.path.join(os.path.expanduser("~"), ".voiceink", "models")
os.makedirs(_mlx_cache, exist_ok=True)
os.environ["HF_HUB_CACHE"] = _mlx_cache
os.environ["HF_HOME"] = _mlx_cache

# Disable tqdm and reduce logging noise before imports
os.environ["TQDM_DISABLE"] = "1"


def log(message):
    """Log to stderr for debugging."""
    print(f"[MLX-FunASR Daemon] {message}", file=sys.stderr, flush=True)


def send_response(response):
    """Send JSON response to stdout."""
    print(json.dumps(response, ensure_ascii=False), flush=True)


def _has_mlx():
    """Check if MLX framework is available (macOS Apple Silicon only)."""
    try:
        import mlx.core
        return True
    except ImportError:
        return False


def check_dependencies():
    """Check if required Python packages are installed."""
    if _has_mlx():
        required = ["mlx", "mlx_audio"]
    else:
        required = ["transformers", "torch"]
    installed = []
    missing = []

    for pkg in required:
        try:
            __import__(pkg)
            installed.append(pkg)
        except ImportError:
            missing.append(pkg)

    return {
        "installed": installed,
        "missing": missing,
        "all_installed": not missing
    }


def get_install_command(missing_packages):
    """Generate pip install command for missing packages."""
    if not missing_packages:
        return None
    pkg_map = {
        "mlx": "mlx",
        "mlx_audio": "mlx-audio-plus"
    }
    packages = [pkg_map.get(p, p) for p in missing_packages]
    return f"pip install {' '.join(packages)}"


def get_model_cache_path(model_repo):
    """Get the model cache path under ~/.voiceink/models."""
    hf_model_dir = f"models--{model_repo.replace('/', '--')}"
    return os.path.join(_mlx_cache, hf_model_dir)


def check_model_downloaded(model_repo):
    """Check if a model is already downloaded."""
    cache_path = get_model_cache_path(model_repo)
    if not os.path.exists(cache_path):
        return False

    snapshots_dir = os.path.join(cache_path, "snapshots")
    if not os.path.exists(snapshots_dir):
        return False

    # Check for safetensors files in any snapshot (single or sharded)
    for snapshot in os.listdir(snapshots_dir):
        snapshot_path = os.path.join(snapshots_dir, snapshot)
        if not os.path.isdir(snapshot_path):
            continue
        for f in os.listdir(snapshot_path):
            if f.endswith(".safetensors"):
                return True

    return False


def _get_cache_blobs_dir(model_repo):
    """Return the blobs directory where HF stores incomplete/complete downloads."""
    cache_path = get_model_cache_path(model_repo)
    return os.path.join(cache_path, "blobs")


def _measure_blobs_size(blobs_dir):
    """Sum up all file sizes in the blobs directory (includes .incomplete files)."""
    total = 0
    if not os.path.isdir(blobs_dir):
        return 0
    for f in os.listdir(blobs_dir):
        fp = os.path.join(blobs_dir, f)
        if os.path.isfile(fp):
            total += os.path.getsize(fp)
    return total


def download_model(model_repo):
    """Download model from HuggingFace with byte-level progress reporting."""
    import threading
    import time
    from huggingface_hub import HfApi, hf_hub_download

    log(f"Downloading model: {model_repo}")
    send_response({"status": "downloading", "model": model_repo, "progress": 0})

    try:
        api = HfApi()
        info = api.model_info(model_repo, files_metadata=True)
        files = [(s.rfilename, s.size or 0) for s in info.siblings]
        total_bytes = sum(size for _, size in files)
        log(f"Model has {len(files)} files, total size: {total_bytes / 1024 / 1024:.0f} MB")

        if total_bytes == 0:
            total_bytes = 1  # avoid division by zero

        blobs_dir = _get_cache_blobs_dir(model_repo)
        download_error = [None]
        download_done = threading.Event()

        def _do_download():
            try:
                for i, (filename, size) in enumerate(files):
                    size_mb = size / 1024 / 1024
                    log(f"Downloading [{i+1}/{len(files)}]: {filename} ({size_mb:.1f} MB)")
                    hf_hub_download(model_repo, filename)
            except Exception as e:
                download_error[0] = e
            finally:
                download_done.set()

        # Start download in background thread
        t = threading.Thread(target=_do_download, daemon=True)
        t.start()

        # Poll blob sizes and report progress until download finishes
        last_progress = -1
        while not download_done.is_set():
            current = _measure_blobs_size(blobs_dir)
            progress = min(int(current * 100 / total_bytes), 99)
            if progress != last_progress:
                last_progress = progress
                send_response({"status": "downloading", "model": model_repo, "progress": progress})
            download_done.wait(timeout=0.5)

        t.join()
        if download_error[0]:
            raise download_error[0]

        send_response({"status": "download_complete", "model": model_repo})
        log(f"Download complete: {model_repo}")
    except Exception as e:
        log(f"Download failed: {e}")
        send_response({
            "status": "download_error",
            "model": model_repo,
            "error": str(e)
        })
        raise


def detect_model_type(model_repo):
    """Detect model type from local cache config.json (no network calls)."""
    try:
        cache_path = get_model_cache_path(model_repo)
        snapshots_dir = os.path.join(cache_path, "snapshots")
        if not os.path.isdir(snapshots_dir):
            return ""
        for snapshot in os.listdir(snapshots_dir):
            config_file = os.path.join(snapshots_dir, snapshot, "config.json")
            if os.path.exists(config_file):
                with open(config_file, "r") as f:
                    config = json.load(f)
                model_type = config.get("model_type", "")
                log(f"Detected model_type: {model_type}")
                return model_type
        return ""
    except Exception as e:
        log(f"Could not detect model type: {e}")
        return ""


def load_model(model_repo, language=None):
    """Load MLX STT model into memory. Supports FunASR, VibeVoice, GLM-ASR, and Qwen3-ASR."""
    log(f"Loading model: {model_repo}")
    log(f"Model repo details: {model_repo.split('/')[-1] if '/' in model_repo else model_repo}")

    model_type = detect_model_type(model_repo)

    # Force offline mode during loading to prevent network timeout from
    # transformers/huggingface_hub making API calls (model is already cached).
    # BUT: if model is not cached locally, allow network access for first download.
    model_is_cached = _is_model_cached(model_repo)
    old_hf_offline = os.environ.get("HF_HUB_OFFLINE")
    old_tf_offline = os.environ.get("TRANSFORMERS_OFFLINE")
    if model_is_cached:
        os.environ["HF_HUB_OFFLINE"] = "1"
        os.environ["TRANSFORMERS_OFFLINE"] = "1"
        log(f"Model cached locally, using offline mode")
    else:
        os.environ.pop("HF_HUB_OFFLINE", None)
        os.environ.pop("TRANSFORMERS_OFFLINE", None)
        log(f"Model not cached, allowing network access for download")

    # Redirect stdout to stderr during loading to prevent JSON pollution
    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        if model_type == "vibevoice_asr":
            from mlx_audio.stt.utils import load_model as stt_load_model
            model = stt_load_model(model_repo)
            model._daemon_model_type = "vibevoice"
        elif model_type == "glmasr":
            # Workaround: mlx_audio's glmasr passes a bool to mx.synchronize()
            # via wired_limit(), but mlx >=0.30 only accepts Stream|None.
            import mlx.core as mx
            _orig_sync = mx.synchronize
            def _safe_sync(stream=None):
                if isinstance(stream, bool):
                    return _orig_sync()
                return _orig_sync(stream)
            mx.synchronize = _safe_sync

            from mlx_audio.stt.utils import load_model as stt_load_model
            model = stt_load_model(model_repo)
            model._daemon_model_type = "glmasr"
        elif model_type == "qwen3_asr":
            if _has_mlx():
                # macOS: use mlx_audio (GPU accelerated via Apple Silicon)
                from mlx_audio.stt.utils import load_model as stt_load_model
                model = stt_load_model(model_repo)
                model._daemon_model_type = "qwen3_asr"
            else:
                # Windows/Linux: use HuggingFace transformers
                model = _load_qwen3_hf(model_repo)
        elif not _has_mlx() and "qwen3" in model_repo.lower():
            # Fallback: repo name suggests Qwen3-ASR but config model_type not detected
            model = _load_qwen3_hf(model_repo)
        else:
            from mlx_audio.stt.models.funasr import Model
            model = Model.from_pretrained(model_repo)
            model._daemon_model_type = "funasr"
        log(f"Model loaded successfully: {model_repo} (type: {model._daemon_model_type})")
    finally:
        sys.stdout = old_stdout
        # Restore original env values
        if old_hf_offline is None:
            os.environ.pop("HF_HUB_OFFLINE", None)
        else:
            os.environ["HF_HUB_OFFLINE"] = old_hf_offline
        if old_tf_offline is None:
            os.environ.pop("TRANSFORMERS_OFFLINE", None)
        else:
            os.environ["TRANSFORMERS_OFFLINE"] = old_tf_offline

    return model


def transcribe(model, audio_path, language=None, max_tokens=None, temperature=None):
    """Transcribe audio file using the loaded model (single pass with detailed logging)."""
    log(f"Transcribing: {audio_path}")

    model_type = getattr(model, '_daemon_model_type', 'funasr')
    temp = temperature if temperature is not None else 0.0
    log(f"Using model type: {model_type}, temperature: {temp}")

    try:
        if model_type == "vibevoice":
            return _transcribe_vibevoice(model, audio_path, language, temperature=temp)
        elif model_type == "glmasr":
            return _transcribe_glmasr(model, audio_path, language, temperature=temp)
        elif model_type == "qwen3_asr":
            tokens = max_tokens if max_tokens else 8192
            return _transcribe_qwen3_asr(model, audio_path, language, max_tokens=tokens, temperature=temp)
        else:
            tokens = max_tokens if max_tokens else 500
            return _transcribe_funasr(model, audio_path, language, max_tokens=tokens, temperature=temp)
    finally:
        gc.collect()
        # Release MLX metal buffer cache back to the system to prevent
        # GPU memory from growing unboundedly across transcription calls.
        if _has_mlx():
            try:
                import mlx.core as mx
                mx.metal.clear_cache()
            except Exception:
                pass


def _transcribe_vibevoice(model, audio_path, language=None, temperature=0.0):
    """Transcribe using VibeVoice model."""
    log(f"VibeVoice transcribing: {audio_path}")

    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        result = model.generate(
            audio_path,
            max_tokens=8192,
            temperature=temperature,
            top_p=1.0,  # Disable nucleus sampling to avoid MLX boolean indexing bug
            verbose=False,
        )
    finally:
        sys.stdout = old_stdout

    text = result.text if hasattr(result, 'text') else str(result)
    log(f"VibeVoice raw text: '{text}'")

    if text:
        text = text.strip()

    log(f"Final text: '{text}' (length: {len(text)})")
    return text


def _transcribe_glmasr(model, audio_path, language=None, temperature=0.0):
    """Transcribe using GLM-ASR model."""
    log(f"GLM-ASR transcribing: {audio_path}")

    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        result = model.generate(
            audio_path,
            max_tokens=500,
            temperature=temperature,
            top_p=1.0,  # Disable nucleus sampling to avoid MLX boolean indexing bug
            verbose=False,
        )
    finally:
        sys.stdout = old_stdout

    text = result.text if hasattr(result, 'text') else str(result)
    log(f"GLM-ASR raw text: '{text}'")

    if text:
        text = text.strip()

    log(f"Final text: '{text}' (length: {len(text)})")
    return text


def _load_qwen3_hf(model_repo):
    """Load Qwen3-ASR using HuggingFace transformers (for Windows/Linux)."""
    from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor
    import torch

    device = "cuda" if torch.cuda.is_available() else "cpu"
    dtype = torch.float16 if torch.cuda.is_available() else torch.float32
    log(f"Loading Qwen3-ASR via transformers: {model_repo} device={device} dtype={dtype}")

    processor = AutoProcessor.from_pretrained(model_repo, trust_remote_code=True)
    model = AutoModelForSpeechSeq2Seq.from_pretrained(
        model_repo, torch_dtype=dtype, trust_remote_code=True
    ).to(device)
    model._daemon_model_type = "qwen3_asr_hf"
    model._processor = processor
    model._device = device
    model._dtype = dtype
    log(f"Qwen3-ASR (transformers) loaded successfully on {device}")
    return model


def _transcribe_qwen3_hf(model, audio_path, language=None, max_tokens=8192, temperature=0.0):
    """Transcribe using Qwen3-ASR with HuggingFace transformers (non-MLX)."""
    import soundfile as sf
    import torch

    log(f"Qwen3-ASR (HF) transcribing: {audio_path}, language={language}")

    audio_data, sample_rate = sf.read(audio_path, dtype='float32')
    if len(audio_data.shape) > 1:
        audio_data = audio_data.mean(axis=1)

    processor = model._processor
    device = model._device

    inputs = processor(
        audio_data, sampling_rate=sample_rate, return_tensors="pt"
    ).to(device)

    generate_kwargs = {"max_new_tokens": max_tokens}
    if language and language not in ("auto", ""):
        mapped = _QWEN3_LANG_MAP.get(language.lower(), language)
        generate_kwargs["language"] = mapped

    with torch.no_grad():
        predicted_ids = model.generate(**inputs, **generate_kwargs)

    text = processor.batch_decode(predicted_ids, skip_special_tokens=True)[0]
    text = text.strip() if text else ""
    log(f"Qwen3-ASR (HF) result: '{text[:80]}...' (length: {len(text)})" if len(text) > 80 else f"Qwen3-ASR (HF) result: '{text}' (length: {len(text)})")
    return text


_QWEN3_LANG_MAP = {
    "zh": "Chinese", "en": "English", "ja": "Japanese", "ko": "Korean",
    "fr": "French", "de": "German", "es": "Spanish", "pt": "Portuguese",
    "it": "Italian", "ru": "Russian", "yue": "Cantonese",
    "chinese": "Chinese", "english": "English", "japanese": "Japanese",
    "korean": "Korean", "french": "French", "german": "German",
    "spanish": "Spanish", "portuguese": "Portuguese", "italian": "Italian",
    "russian": "Russian", "cantonese": "Cantonese",
}


def _transcribe_qwen3_asr(model, audio_path, language=None, max_tokens=8192, temperature=0.0):
    """Transcribe using Qwen3-ASR model, with VAD chunking for long audio."""
    # Dispatch to HF transformers backend if loaded that way
    if getattr(model, '_daemon_model_type', '') == "qwen3_asr_hf":
        return _transcribe_qwen3_hf(model, audio_path, language, max_tokens, temperature)

    import soundfile as sf
    import numpy as np
    import re as _re

    log(f"Qwen3-ASR transcribing: {audio_path}, language={language}")

    audio_data, sample_rate = sf.read(audio_path, dtype='float32')
    duration_secs = len(audio_data) / sample_rate
    log(f"Qwen3-ASR audio duration: {duration_secs:.2f}s")

    if len(audio_data.shape) > 1:
        audio_data = audio_data.mean(axis=1)

    if sample_rate != 16000:
        from mlx_audio.stt.utils import resample_audio
        audio_data = resample_audio(audio_data, sample_rate, 16000)
        sample_rate = 16000

    max_chunk_secs = 30

    if duration_secs <= max_chunk_secs + 5:
        # Short audio — single pass
        text = _qwen3_generate_single(model, audio_path, language, max_tokens, temperature)
    else:
        # Long audio — VAD split
        log(f"Long audio ({duration_secs:.1f}s), VAD splitting for Qwen3-ASR")
        chunks = _vad_split_audio(audio_data, sample_rate, np,
                                  max_chunk_secs=max_chunk_secs,
                                  min_chunk_secs=15)
        log(f"Qwen3-ASR: {len(chunks)} chunks")

        detected_lang = language
        texts = []
        tmp_dir = os.path.join(os.path.expanduser("~"), ".voiceink", "tmp")
        os.makedirs(tmp_dir, exist_ok=True)

        for idx, (chunk, chunk_offset) in enumerate(chunks):
            chunk_duration = len(chunk) / sample_rate
            log(f"Qwen3 Chunk {idx}: offset={chunk_offset:.1f}s duration={chunk_duration:.1f}s")

            # Write chunk to temp WAV
            tmp_chunk = os.path.join(tmp_dir, f"qwen3_chunk_{idx}.wav")
            sf.write(tmp_chunk, chunk, sample_rate)

            chunk_text = _qwen3_generate_single(model, tmp_chunk, detected_lang, max_tokens, temperature)

            try:
                os.unlink(tmp_chunk)
            except Exception:
                pass

            # Detect language from first chunk
            if idx == 0 and detected_lang in (None, "auto", ""):
                detected_lang = _detect_language_from_text(chunk_text)
                log(f"Qwen3-ASR language detected: {detected_lang}")

            chunk_text = _clean_funasr_text(chunk_text, _re)
            if chunk_text:
                texts.append(chunk_text)
                preview = f"'{chunk_text[:50]}...'" if len(chunk_text) > 50 else f"'{chunk_text}'"
                log(f"Qwen3 Chunk {idx} result: {preview}")

        text = "".join(texts)
        log(f"Qwen3-ASR chunked complete: {len(chunks)} chunks, {len(text)} chars")

    log(f"Final text: '{text}' (length: {len(text)})")
    return text


def _qwen3_generate_single(model, audio_path, language, max_tokens, temperature):
    """Transcribe a single audio file with Qwen3-ASR."""
    generate_kwargs = {
        "max_tokens": max_tokens,
        "temperature": temperature,
        "top_p": 1.0,
        "verbose": False,
    }
    if language and language not in ("auto", ""):
        mapped = _QWEN3_LANG_MAP.get(language.lower(), language)
        generate_kwargs["language"] = mapped

    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        result = model.generate(audio_path, **generate_kwargs)
    finally:
        sys.stdout = old_stdout

    text = result.text if hasattr(result, 'text') else str(result)
    _log_funasr_result(result)
    return text.strip() if text else ""


def _transcribe_funasr(model, audio_path, language=None, max_tokens=500, temperature=0.0):
    """Transcribe using FunASR model."""
    import soundfile as sf
    import numpy as np
    import re

    log(f"FunASR transcribing: {audio_path}")

    text = _funasr_generate_text(
        model, audio_path, language, sf, np,
        max_tokens=max_tokens, temperature=temperature
    )
    text = _clean_funasr_text(text, re)
    log(f"Final text: '{text}' (length: {len(text)})")
    return text


def _funasr_generate_text(model, audio_path, language, sf, np, max_tokens=500, temperature=0.0):
    """Generate raw text from FunASR model, with VAD-based chunking for long audio."""
    import re as _re

    audio_data, sample_rate = sf.read(audio_path, dtype='float32')
    log(f"Total audio duration: {len(audio_data) / sample_rate:.2f}s, sample_rate: {sample_rate}")

    # Convert to mono if stereo
    if len(audio_data.shape) > 1:
        audio_data = audio_data.mean(axis=1)

    # Resample to 16kHz if needed (model expects 16000 Hz)
    target_sr = 16000
    if sample_rate != target_sr:
        log(f"Resampling from {sample_rate} to {target_sr}")
        from mlx_audio.stt.utils import resample_audio
        audio_data = resample_audio(audio_data, sample_rate, target_sr)
        sample_rate = target_sr

    duration_secs = len(audio_data) / sample_rate
    max_chunk_secs = 30   # FunASR works best with ≤30s audio

    if duration_secs <= max_chunk_secs + 5:
        # Short audio: process as single chunk
        text = _funasr_generate_single_chunk(model, audio_data, sample_rate, language, np, max_tokens, temperature)
    else:
        # Long audio: VAD-based split at silence boundaries
        chunks = _vad_split_audio(audio_data, sample_rate, np,
                                  max_chunk_secs=max_chunk_secs,
                                  min_chunk_secs=15)
        log(f"Long audio ({duration_secs:.1f}s), VAD split into {len(chunks)} chunks")

        # For "auto" language: detect language on first chunk, then lock it for all chunks.
        # This prevents per-chunk language switching (e.g., Chinese → English → Chinese).
        detected_lang = language
        texts = []
        for idx, (chunk, chunk_offset) in enumerate(chunks):
            chunk_duration = len(chunk) / sample_rate
            log(f"Chunk {idx}: offset={chunk_offset:.1f}s duration={chunk_duration:.1f}s lang={detected_lang}")

            chunk_text = _funasr_generate_single_chunk(
                model, chunk, sample_rate, detected_lang, np, max_tokens, temperature
            )

            # After first chunk: detect language from output and lock it
            if idx == 0 and detected_lang in (None, "auto", ""):
                detected_lang = _detect_language_from_text(chunk_text)
                log(f"Language detected from chunk 0: {detected_lang}")

            chunk_text = _clean_funasr_text(chunk_text, _re)
            if chunk_text:
                texts.append(chunk_text)
                preview = f"'{chunk_text[:50]}...' ({len(chunk_text)} chars)" if len(chunk_text) > 50 else f"'{chunk_text}'"
                log(f"Chunk {idx} result: {preview}")
            else:
                log(f"Chunk {idx}: empty result (silence/noise)")

        text = "".join(texts)
        log(f"Chunked transcription complete: {len(chunks)} chunks, {len(text)} chars total")

    return text


def _vad_split_audio(audio, sample_rate, np, max_chunk_secs=30, min_chunk_secs=5):
    """Split audio at silence boundaries using energy-based VAD.

    Returns list of (chunk_array, offset_seconds) tuples.
    Each chunk is ≤ max_chunk_secs long, cut at the quietest point near the boundary.
    """
    total_samples = len(audio)
    max_chunk_samples = int(max_chunk_secs * sample_rate)
    min_chunk_samples = int(min_chunk_secs * sample_rate)

    # Precompute frame-level energy (20ms frames)
    frame_size = int(0.02 * sample_rate)  # 320 samples at 16kHz
    n_frames = total_samples // frame_size
    energy = np.array([
        np.mean(audio[i * frame_size:(i + 1) * frame_size] ** 2)
        for i in range(n_frames)
    ])

    # Smooth energy with a small window to avoid cutting on transient dips
    smooth_window = 5  # 100ms
    if len(energy) > smooth_window:
        kernel = np.ones(smooth_window) / smooth_window
        energy = np.convolve(energy, kernel, mode='same')

    chunks = []
    offset = 0

    while offset < total_samples:
        remaining = total_samples - offset

        if remaining <= max_chunk_samples + min_chunk_samples:
            # Last chunk: take everything
            chunks.append((audio[offset:], offset / sample_rate))
            break

        # Look for the quietest point in the search window [min_chunk, max_chunk]
        search_start_frame = (offset + min_chunk_samples) // frame_size
        search_end_frame = min((offset + max_chunk_samples) // frame_size, n_frames)

        if search_start_frame >= search_end_frame:
            # Fallback: hard cut at max_chunk
            cut = offset + max_chunk_samples
        else:
            # Find frame with minimum energy in the search window
            window_energy = energy[search_start_frame:search_end_frame]
            best_frame = search_start_frame + np.argmin(window_energy)
            cut = best_frame * frame_size
            log(f"VAD: silence at {cut / sample_rate:.2f}s (energy={energy[best_frame]:.6f})")

        chunks.append((audio[offset:cut], offset / sample_rate))
        offset = cut

    return chunks


def _funasr_generate_single_chunk(model, audio_data, sample_rate, language, np, max_tokens, temperature):
    """Transcribe a single audio chunk (≤30s recommended)."""
    # Append 1s silence padding to prevent truncation
    padding_samples = int(sample_rate * 1.0)
    silence = np.zeros(padding_samples, dtype=audio_data.dtype)
    padded = np.concatenate([audio_data, silence])

    lang_param = _build_funasr_lang_param(language)

    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        result = _run_funasr_streaming(model, padded, lang_param, max_tokens=max_tokens, temperature=temperature)
    finally:
        sys.stdout = old_stdout
        del padded

    text = result.text if hasattr(result, 'text') else str(result)
    _log_funasr_result(result)
    return text


def _is_model_cached(model_repo):
    """Check if a HuggingFace model is already cached locally."""
    try:
        from huggingface_hub import scan_cache_dir
        cache_info = scan_cache_dir()
        for repo in cache_info.repos:
            if repo.repo_id == model_repo:
                return True
    except Exception:
        pass

    # Also check custom models dir (~/.voiceink/models/)
    home = os.path.expanduser("~")
    custom_dir = os.path.join(home, ".voiceink", "models",
                              f"models--{model_repo.replace('/', '--')}")
    if os.path.isdir(custom_dir):
        return True

    return False


def _detect_language_from_text(text):
    """Detect dominant language from transcribed text.

    Simple heuristic: count CJK characters vs Latin characters.
    Returns a language code suitable for FunASR lang_param.
    """
    if not text:
        return "auto"
    cjk = sum(1 for c in text if '\u4e00' <= c <= '\u9fff')
    latin = sum(1 for c in text if 'a' <= c.lower() <= 'z')
    jp_kana = sum(1 for c in text if '\u3040' <= c <= '\u30ff')
    kr = sum(1 for c in text if '\uac00' <= c <= '\ud7af')

    if jp_kana > len(text) * 0.1:
        return "ja"
    if kr > len(text) * 0.1:
        return "ko"
    if cjk > latin:
        return "zh"
    if latin > cjk:
        return "en"
    return "auto"


def _build_funasr_lang_param(language):
    """Build language parameter dict for FunASR."""
    if not language or language == "auto":
        return {}
    lang_map = {"zh": "中文", "en": "English", "ja": "日文", "ko": "韩文"}
    return {"language": lang_map.get(language, language)}


def _run_funasr_streaming(model, audio_input, lang_param, max_tokens=500, temperature=0.0):
    """Run FunASR streaming generation with repetition detection.

    Args:
        audio_input: File path (str) or numpy array of audio samples.
    """
    from mlx_audio.stt.models.funasr.funasr import STTOutput

    generate_params = {
        "max_tokens": max_tokens,
        "temperature": temperature,
        "top_p": 1.0,  # Disable nucleus sampling to avoid MLX boolean indexing bug
        "verbose": False,
        "stream": True,
    }
    generate_params.update(lang_param)

    original_eos = _fix_eos_tokens(model)
    log(f"Calling model.generate with params: {generate_params}")

    try:
        result = _stream_with_repetition_check(model, audio_input, generate_params, STTOutput)
    finally:
        _restore_eos_tokens(model, original_eos)

    return result


def _fix_eos_tokens(model):
    """Fix EOS tokens to prevent premature stop on < or </."""
    original = model._eos_token_ids.copy() if hasattr(model, '_eos_token_ids') else set()
    log(f"Original EOS tokens: {original}")
    if hasattr(model, '_eos_token_ids'):
        model._eos_token_ids = {151643, 151645}
        log(f"Fixed EOS tokens to: {model._eos_token_ids}")
    return original


def _restore_eos_tokens(model, original_eos):
    """Restore original EOS tokens on the model."""
    if hasattr(model, '_eos_token_ids'):
        model._eos_token_ids = original_eos


def _stream_with_repetition_check(model, audio_input, params, STTOutput):
    """Stream model output and stop early on character repetition or safety limits."""
    chunks = []
    total_len = 0
    last_char = ""
    repeat_count = 0
    max_repeats = 5
    max_length = 50000   # hard cap: no transcription should exceed this
    timeout_sec = 120    # 2 min timeout for generation
    start_time = time.time()
    final_result = None   # capture non-string result (e.g. STTOutput) if yielded

    for chunk in model.generate(audio_input, **params):
        if time.time() - start_time > timeout_sec:
            log(f"Generation timeout after {timeout_sec}s, collected {total_len} chars")
            break

        if isinstance(chunk, str):
            chunks.append(chunk)
            total_len += len(chunk)

            if total_len > max_length:
                log(f"Hit max length {max_length}, stopping")
                break

            if chunk and chunk == last_char:
                repeat_count += 1
                if repeat_count >= max_repeats:
                    log(f"Detected repetition, stopping early at {total_len} chars")
                    break
            else:
                repeat_count = 0
                last_char = chunk
        else:
            # model.generate() may yield a final STTOutput object after all string chunks.
            # Capture it but don't return yet — there may be more string chunks.
            final_result = chunk
            log(f"Received non-string chunk: {type(chunk).__name__}")

    # If we collected string chunks, use them (they are the actual transcription)
    if chunks:
        full_text = "".join(chunks)
        return STTOutput(text=full_text, language=None, task="transcribe", duration=0, tokens=[])

    # If no string chunks but we got a final STTOutput, use it
    if final_result is not None:
        if hasattr(final_result, 'text') and final_result.text:
            log(f"Using final result text: {len(final_result.text)} chars")
            return final_result
        log(f"Final result has no usable text, type={type(final_result).__name__}")

    # Fallback: empty result
    log("No transcription output produced")
    return STTOutput(text="", language=None, task="transcribe", duration=0, tokens=[])


def _safe_unlink(path):
    """Remove a file, ignoring errors."""
    if path:
        try:
            os.unlink(path)
        except Exception:
            pass


def _log_funasr_result(result):
    """Log detailed attributes from FunASR result."""
    log(f"Result type: {type(result)}")
    if hasattr(result, 'duration'):
        log(f"result.duration: {result.duration}")
    if hasattr(result, 'audio_duration'):
        log(f"result.audio_duration: {result.audio_duration}")
    if hasattr(result, 'tokens'):
        log(f"result.tokens count: {len(result.tokens) if result.tokens else 0}")
    if hasattr(result, 'segments'):
        log(f"result.segments: {result.segments}")


def _clean_funasr_text(text, re):
    """Clean special tokens and noise from FunASR output text."""
    if not text:
        return text
    text = text.replace('/sil', '')
    text = re.sub(r'<\d*>', '', text)           # timestamp tokens like <123>
    text = re.sub(r'<[a-zA-Z_]+>', '', text)    # special tokens like <noise>, <eos>, <blank>
    text = re.sub(r'\(+\)+', '', text)           # noise markers like (()), ((()))
    text = re.sub(r'(.)\1{3,}$', '', text)      # trailing repetitions
    text = re.sub(r'\s*[<>]+\s*', '', text)      # stray angle brackets
    return text.strip()


def main():
    log("Starting...")

    # Current model state
    model = None
    model_repo = None

    log("Ready to receive commands")
    send_response({"status": "ready"})

    # Main loop - read commands from stdin
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            cmd = json.loads(line)
        except json.JSONDecodeError as e:
            send_response({"status": "error", "error": f"Invalid JSON: {e}"})
            continue

        action = cmd.get("action")

        if action == "ping":
            send_response({
                "status": "pong",
                "model_loaded": model is not None,
                "model": model_repo
            })

        elif action == "check_dependencies":
            dep_result = check_dependencies()
            install_cmd = get_install_command(dep_result["missing"])
            send_response({
                "status": "dependencies",
                "installed": dep_result["installed"],
                "missing": dep_result["missing"],
                "all_installed": dep_result["all_installed"],
                "install_command": install_cmd
            })

        elif action == "check_model":
            check_repo = cmd.get("model", "")
            if not check_repo:
                send_response({
                    "status": "error",
                    "error": "Model repo not specified"
                })
                continue

            is_downloaded = check_model_downloaded(check_repo)
            send_response({
                "status": "model_status",
                "model": check_repo,
                "downloaded": is_downloaded
            })

        elif action == "download":
            download_repo = cmd.get("model", "")
            if not download_repo:
                send_response({
                    "status": "error",
                    "error": "Model repo not specified"
                })
                continue

            try:
                download_model(download_repo)
            except Exception as e:
                send_response({"status": "error", "error": str(e)})

        elif action == "load":
            new_repo = cmd.get("model", "")
            language = cmd.get("language")

            if not new_repo:
                send_response({
                    "status": "error",
                    "error": "Model repo not specified"
                })
                continue

            if model is not None and model_repo == new_repo:
                log(f"Model {new_repo} already loaded")
                send_response({"status": "loaded", "model": new_repo})
                continue

            try:
                # Unload previous model before loading a different one
                if model is not None:
                    log(f"Unloading previous model: {model_repo}")
                    del model
                    model = None
                    model_repo = None
                    gc.collect()
                    if _has_mlx():
                        try:
                            import mlx.core as mx
                            mx.metal.clear_cache()
                        except Exception:
                            pass

                # Download if not available
                if not check_model_downloaded(new_repo):
                    download_model(new_repo)

                model = load_model(new_repo, language)
                model_repo = new_repo
                log(f"Model switch complete: {new_repo}")
                send_response({"status": "loaded", "model": new_repo})
            except Exception as e:
                log(f"Failed to load model: {e}")
                import traceback
                traceback.print_exc(file=sys.stderr)
                send_response({"status": "error", "error": str(e)})

        elif action == "transcribe":
            audio_path = cmd.get("audio")
            language = cmd.get("language")
            max_tokens = cmd.get("max_tokens")
            temperature = cmd.get("temperature")

            if model is None:
                send_response({
                    "status": "error",
                    "error": "Model not loaded"
                })
                continue

            if not audio_path or not os.path.exists(audio_path):
                send_response({
                    "status": "error",
                    "error": f"Audio file not found: {audio_path}"
                })
                continue

            try:
                text = transcribe(model, audio_path, language, max_tokens, temperature)
                send_response({"status": "success", "text": text})
            except Exception as e:
                log(f"Transcription failed: {e}")
                import traceback
                traceback.print_exc(file=sys.stderr)
                send_response({"status": "error", "error": str(e)})

        elif action == "unload":
            model = None
            model_repo = None
            gc.collect()
            if _has_mlx():
                try:
                    import mlx.core as mx
                    mx.metal.clear_cache()
                except Exception:
                    pass
            log("Model unloaded (memory freed)")
            send_response({"status": "unloaded"})

        elif action == "quit":
            log("Shutting down")
            send_response({"status": "bye"})
            break

        else:
            send_response({
                "status": "error",
                "error": f"Unknown action: {action}"
            })

    log("Daemon stopped")


if __name__ == "__main__":
    main()
