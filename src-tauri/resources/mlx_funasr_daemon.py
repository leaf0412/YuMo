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

# All model cache lives under ~/.voiceink/mlx-cache
_mlx_cache = os.path.join(os.path.expanduser("~"), ".voiceink", "mlx-cache")
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


def check_dependencies():
    """Check if required Python packages are installed."""
    required = ["mlx", "mlx_audio"]
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
    """Get the model cache path under ~/.voiceink/mlx-cache."""
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


def download_model(model_repo):
    """Download model from HuggingFace with byte-level progress reporting."""
    from huggingface_hub import HfApi, hf_hub_download

    log(f"Downloading model: {model_repo}")
    send_response({"status": "downloading", "model": model_repo, "progress": 0})

    try:
        api = HfApi()
        info = api.model_info(model_repo, files_metadata=True)
        files = [(s.rfilename, s.size or 0) for s in info.siblings]
        total_bytes = sum(size for _, size in files)
        log(f"Model has {len(files)} files, total size: {total_bytes / 1024 / 1024:.0f} MB")

        # Shared state for byte-level progress tracking across all files
        state = {"downloaded": 0, "last_progress": -1}

        class DownloadProgress:
            """tqdm-compatible class that reports byte-level progress."""
            def __init__(self, *args, **kwargs):
                self.n = kwargs.get("initial", 0)

            def update(self, n=1):
                self.n += n
                state["downloaded"] += n
                if total_bytes > 0:
                    progress = min(int(state["downloaded"] * 100 / total_bytes), 99)
                    if progress != state["last_progress"]:
                        state["last_progress"] = progress
                        send_response({"status": "downloading", "model": model_repo, "progress": progress})

            def close(self): pass
            def clear(self): pass
            def refresh(self): pass
            def set_postfix_str(self, *a, **kw): pass
            def __enter__(self): return self
            def __exit__(self, *a): self.close()

        for i, (filename, size) in enumerate(files):
            size_mb = size / 1024 / 1024
            log(f"Downloading [{i+1}/{len(files)}]: {filename} ({size_mb:.1f} MB)")
            try:
                hf_hub_download(model_repo, filename, tqdm_class=DownloadProgress)
            except TypeError:
                # Newer huggingface_hub removed tqdm_class parameter
                hf_hub_download(model_repo, filename)
                state["downloaded"] += size
                progress = min(int(state["downloaded"] * 100 / total_bytes), 99) if total_bytes > 0 else 0
                if progress != state["last_progress"]:
                    state["last_progress"] = progress
                    send_response({"status": "downloading", "model": model_repo, "progress": progress})

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
    # transformers/huggingface_hub making API calls (model is already cached)
    old_hf_offline = os.environ.get("HF_HUB_OFFLINE")
    old_tf_offline = os.environ.get("TRANSFORMERS_OFFLINE")
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"

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
            from mlx_audio.stt.utils import load_model as stt_load_model
            model = stt_load_model(model_repo)
            model._daemon_model_type = "qwen3_asr"
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


def _transcribe_qwen3_asr(model, audio_path, language=None, max_tokens=8192, temperature=0.0):
    """Transcribe using Qwen3-ASR model."""
    log(f"Qwen3-ASR transcribing: {audio_path}, language={language}")

    # Qwen3-ASR expects full language names, not ISO codes
    # Map common ISO 639-1 codes to Qwen3-ASR language names
    qwen3_lang_map = {
        "zh": "Chinese",
        "en": "English",
        "ja": "Japanese",
        "ko": "Korean",
        "fr": "French",
        "de": "German",
        "es": "Spanish",
        "pt": "Portuguese",
        "it": "Italian",
        "ru": "Russian",
        "yue": "Cantonese",
        # Also support already-correct formats (case-insensitive)
        "chinese": "Chinese",
        "english": "English",
        "japanese": "Japanese",
        "korean": "Korean",
        "french": "French",
        "german": "German",
        "spanish": "Spanish",
        "portuguese": "Portuguese",
        "italian": "Italian",
        "russian": "Russian",
        "cantonese": "Cantonese",
    }

    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        # Qwen3-ASR supports language parameter for better accuracy
        generate_kwargs = {
            "max_tokens": max_tokens,
            "temperature": temperature,
            "top_p": 1.0,  # Disable nucleus sampling to avoid MLX boolean indexing bug
            "verbose": False,
        }
        if language and language != "auto":
            # Map ISO code to Qwen3-ASR language name
            mapped_lang = qwen3_lang_map.get(language.lower(), language)
            generate_kwargs["language"] = mapped_lang
            log(f"Qwen3-ASR using language: {mapped_lang} (from {language})")

        result = model.generate(audio_path, **generate_kwargs)
    finally:
        sys.stdout = old_stdout

    text = result.text if hasattr(result, 'text') else str(result)
    log(f"Qwen3-ASR raw text: '{text}'")

    if text:
        text = text.strip()

    log(f"Final text: '{text}' (length: {len(text)})")
    return text


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
    """Generate raw text from FunASR model, passing audio array directly."""
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

    # Append 1s silence padding to prevent truncation
    padding_samples = int(sample_rate * 1.0)
    silence = np.zeros(padding_samples, dtype=audio_data.dtype)
    audio_data = np.concatenate([audio_data, silence])
    log(f"Padded audio duration: {len(audio_data) / sample_rate:.2f}s (+1.0s padding)")

    lang_param = _build_funasr_lang_param(language)

    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        # Pass numpy array directly to model.generate() — no temp file needed
        result = _run_funasr_streaming(model, audio_data, lang_param, max_tokens=max_tokens, temperature=temperature)
    finally:
        sys.stdout = old_stdout
        del audio_data

    text = result.text if hasattr(result, 'text') else str(result)
    _log_funasr_result(result)
    return text


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
    """Stream model output and stop early on character repetition."""
    full_text = ""
    last_char = ""
    repeat_count = 0
    max_repeats = 5

    for chunk in model.generate(audio_input, **params):
        if isinstance(chunk, str):
            full_text += chunk
            if chunk and chunk == last_char:
                repeat_count += 1
                if repeat_count >= max_repeats:
                    log(f"Detected repetition, stopping early at {len(full_text)} chars")
                    break
            else:
                repeat_count = 0
                last_char = chunk
        else:
            return chunk

    return STTOutput(text=full_text, language=None, task="transcribe", duration=0, tokens=[])


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
    text = re.sub(r'<\d*>', '', text)
    text = re.sub(r'(.)\1{3,}$', '', text)
    text = re.sub(r'\s*[<>]+\s*', '', text)
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
