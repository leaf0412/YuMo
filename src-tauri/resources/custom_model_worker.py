#!/usr/bin/env python3
"""Custom-model one-shot worker.

Spawned per action by the Rust side so that long-running operations
(dependency install, model download) cannot block the transcription
daemon. Reads exactly one JSON command from stdin, executes it, prints
exactly one JSON object to stdout, then exits.

Protocol
--------
Input  (stdin, single line, JSON):
    {"action": "check_deps",   "spec_path": "<abs path>"}
    {"action": "install_deps", "spec_path": "<abs path>"}
    {"action": "download",     "spec_path": "<abs path>",
     "voiceink_models_dir": "<abs path>",
     "custom_models_dir":   "<abs path>"}

Output (stdout, exactly one line, JSON):
    {"ok": true,  ...result fields...}
    {"ok": false, "error": "<message>"}

Exit code:
    0   on success (ok=true)
    1   on any failure (ok=false). Stack trace is written to stderr.

Errors are NEVER swallowed: both an `ok=false` JSON line on stdout AND
a non-zero exit code so the Rust side can fail loudly.
"""
import json
import sys
import traceback

import custom_model_shared as shared


def emit(payload: dict) -> None:
    """Write a single JSON line to stdout and flush."""
    print(json.dumps(payload, ensure_ascii=False), flush=True)


def fail(error: str) -> None:
    """Emit a failure response and exit with non-zero status."""
    emit({"ok": False, "error": error})
    sys.exit(1)


def run(cmd: dict) -> dict:
    action = cmd.get("action")
    if action == "check_deps":
        return shared.check_custom_dependencies(cmd["spec_path"])
    if action == "install_deps":
        return shared.install_custom_dependencies(cmd["spec_path"])
    if action == "download":
        return shared.download_custom_model(
            cmd["spec_path"],
            cmd["voiceink_models_dir"],
            cmd["custom_models_dir"],
        )
    raise ValueError(f"unknown action: {action!r}")


def main() -> None:
    line = sys.stdin.readline()
    if not line:
        fail("worker received no input on stdin")

    try:
        cmd = json.loads(line)
    except json.JSONDecodeError as e:
        fail(f"invalid JSON command: {e}")

    try:
        result = run(cmd)
    except Exception as e:
        traceback.print_exc(file=sys.stderr)
        fail(str(e))

    emit({"ok": True, **result})


if __name__ == "__main__":
    main()
