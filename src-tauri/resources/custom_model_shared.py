#!/usr/bin/env python3
"""Shared logic for custom YAML-defined models.

This module is imported by BOTH:
  - mlx_funasr_daemon.py  — to load + transcribe a custom-spec model
                             (long-running, no download/install actions)
  - custom_model_worker.py — to check / install dependencies / download
                             (one-shot per invocation, no model loading)

Splitting the long-running parts (load / transcribe) from the long-running-
but-blocking parts (download / install) lets the worker run in its own
process so that downloads cannot stall the transcription daemon.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Optional


# ---------------------------------------------------------------------------
# Spec parsing and small utils
# ---------------------------------------------------------------------------

def parse_spec(spec_path: str) -> dict:
    """Read and YAML-parse a custom model spec file. Lazy yaml import."""
    import yaml
    with open(spec_path, "r") as f:
        return yaml.safe_load(f)


def split_pip_pkg_name(pkg_spec: str) -> str:
    """'mimo_mlx>=0.1.0' -> 'mimo_mlx'. Handles >=, ==, ~=, <, >, !=, ;markers."""
    return re.split(r"[<>=!~;\s]", pkg_spec, 1)[0].strip()


def resolve_attr(module_name: str, dotted_path: str):
    """Resolve 'pkg.submod.func' -> callable, after importing pkg.

    The first segment of the dotted path may equal the module name (so
    'mimo_mlx.load_asr' and 'load_asr' both work). Attribute lookup
    falls back to importing as a deeper submodule when an attribute is
    actually a module.
    """
    import importlib
    parts = dotted_path.split(".")
    if parts[0] == module_name:
        attrs = parts[1:]
    else:
        attrs = parts
    obj = importlib.import_module(module_name)
    for i, a in enumerate(attrs):
        try:
            obj = getattr(obj, a)
        except AttributeError:
            full = ".".join([module_name] + attrs[: i + 1])
            obj = importlib.import_module(full)
    return obj


def render_kwargs(
    kwargs: dict,
    voiceink_models_dir: str,
    paths: Optional[dict] = None,
    repo_dirs: Optional[list] = None,
) -> dict:
    """Replace {voiceink_models_dir}, {paths.X}, {repo_dirs[N]} in string values."""
    rendered = {}
    paths = paths or {}
    repo_dirs = repo_dirs or []
    for k, v in kwargs.items():
        if isinstance(v, str):
            v = v.replace("{voiceink_models_dir}", voiceink_models_dir)
            for m in list(re.finditer(r"\{paths\.([^}]+)\}", v)):
                key = m.group(1)
                if key not in paths:
                    raise KeyError(f"paths.{key} not resolved (download must run first)")
                v = v.replace(m.group(0), paths[key])
            for m in list(re.finditer(r"\{repo_dirs\[(\d+)\]\}", v)):
                idx = int(m.group(1))
                if idx >= len(repo_dirs):
                    raise IndexError(f"repo_dirs[{idx}] out of range (only {len(repo_dirs)} repos)")
                v = v.replace(m.group(0), str(repo_dirs[idx]))
        rendered[k] = v
    return rendered


# ---------------------------------------------------------------------------
# Dependency check / install
# ---------------------------------------------------------------------------

def check_custom_dependencies(spec_path: str) -> dict:
    """Check whether the YAML spec's pip_packages are importable.

    Returns: {installed: [...], missing: [pkg_spec, ...], all_installed: bool}
    """
    import importlib.util
    spec = parse_spec(spec_path)
    pip_packages = spec.get("pip_packages") or [spec["python_module"]]

    installed, missing = [], []
    for pkg_spec in pip_packages:
        name = split_pip_pkg_name(pkg_spec)
        if importlib.util.find_spec(name) is not None:
            installed.append(name)
        else:
            missing.append(pkg_spec)
    return {
        "installed": installed,
        "missing": missing,
        "all_installed": not missing,
    }


def install_custom_dependencies(spec_path: str) -> dict:
    """Run pip install for the spec's pip_packages.

    Returns: {success: bool, stdout: str, stderr: str, error: str|None}
    """
    spec = parse_spec(spec_path)
    pkgs = spec.get("pip_packages") or [spec["python_module"]]
    cmd = [sys.executable, "-m", "pip", "install", *pkgs]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if result.returncode == 0:
        return {
            "success": True,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "error": None,
        }
    return {
        "success": False,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "error": result.stderr.strip() or "pip install failed",
    }


# ---------------------------------------------------------------------------
# Download / load
# ---------------------------------------------------------------------------

def download_custom_model(
    spec_path: str,
    voiceink_models_dir: str,
    custom_models_dir: str,
) -> dict:
    """Run the download step declared in the YAML spec; write sidecar paths.json."""
    spec = parse_spec(spec_path)
    download = spec.get("download")
    if not download:
        return {"success": True, "paths": {}, "note": "no download step declared"}

    paths_out: dict = {}

    if "function" in download:
        func = resolve_attr(spec["python_module"], download["function"])
        rendered = render_kwargs(download.get("kwargs", {}), voiceink_models_dir)
        result = func(**rendered)
        returns = download.get("returns", "tuple")
        if returns == "tuple":
            names = download.get("path_names", [])
            if len(names) != len(result):
                raise ValueError(
                    f"path_names has {len(names)} entries but function returned {len(result)} values"
                )
            paths_out = {n: str(p) for n, p in zip(names, result)}
        elif returns == "dict":
            paths_out = {k: str(v) for k, v in result.items()}
        elif returns == "path":
            names = download.get("path_names", ["model_dir"])
            paths_out = {names[0]: str(result)}
        else:
            raise ValueError(f"unknown returns kind: {returns}")
    elif "hf_repos" in download:
        from huggingface_hub import snapshot_download
        repo_dirs = []
        for repo in download["hf_repos"]:
            sanitized = repo.replace("/", "--")
            local_dir = Path(voiceink_models_dir) / f"custom-{sanitized}"
            snapshot_download(repo, local_dir=str(local_dir))
            repo_dirs.append(str(local_dir))

        raw_paths = download.get("paths", {})
        if not raw_paths:
            paths_out = {f"repo_{i}": p for i, p in enumerate(repo_dirs)}
        else:
            paths_out = render_kwargs(raw_paths, voiceink_models_dir, repo_dirs=repo_dirs)
            paths_out = {k: str(v) for k, v in paths_out.items()}
    else:
        raise ValueError("download must declare either 'function' or 'hf_repos'")

    cache_dir = Path(custom_models_dir) / ".cache"
    cache_dir.mkdir(parents=True, exist_ok=True)
    sidecar = cache_dir / f"{spec['id']}.paths.json"
    tmp = sidecar.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(paths_out, indent=2))
    tmp.replace(sidecar)

    return {"success": True, "paths": paths_out}


def load_custom_model(spec_path: str, voiceink_models_dir: str, custom_models_dir: str):
    """Load a custom-spec model: read sidecar, render kwargs, call load.function."""
    spec = parse_spec(spec_path)
    sidecar = Path(custom_models_dir) / ".cache" / f"{spec['id']}.paths.json"
    if not sidecar.exists():
        raise FileNotFoundError(
            f"paths.json sidecar missing at {sidecar} — run download_custom_model first"
        )
    paths = json.loads(sidecar.read_text())

    load = spec["load"]
    rendered = render_kwargs(load.get("kwargs", {}), voiceink_models_dir, paths=paths)

    func = resolve_attr(spec["python_module"], load["function"])
    model = func(**rendered)
    model._daemon_model_type = "custom"
    model._daemon_custom_spec = spec
    return model
