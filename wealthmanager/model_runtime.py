"""On-demand LM Studio loading; no saved chats or global settings are modified."""
import os
from pathlib import Path
import shutil
import subprocess


def load_for_statements(models, request_json):
    override = os.environ.get("LM_STUDIO_MODEL")
    choices = [m for m in models if m.get("type") == "llm" and (not override or m.get("key") == override)]
    if not choices:
        raise ValueError("No installed chat model found. Download a model in LM Studio or check LM_STUDIO_MODEL.")
    # Prefer the user's small Bonsai quantization, then the smallest installed LLM.
    model = min(choices, key=lambda m: ("bonsai" not in m["key"].lower(), m.get("size_bytes", 10**15)))
    cli = os.environ.get("LM_STUDIO_CLI") or shutil.which("lms") or str(Path.home() / ".lmstudio/bin/lms")
    if not Path(cli).is_file():
        raise ValueError("LM Studio CLI is required for memory-bounded auto-loading. Set LM_STUDIO_CLI to lms.")
    context = max(4096, min(8192, int(os.environ.get("WEALTH_AI_CONTEXT", "8192"))))
    try:
        subprocess.run([cli, "load", model["key"], "--identifier", "wealth-statements",
                        "--context-length", str(context), "--parallel", "1", "--ttl", "120", "--yes"],
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.PIPE, check=True, timeout=180)
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError("Cannot load the local statement model. Check available memory and the LM Studio server.") from error
    data = request_json("/api/v1/models")
    instance = next((i for m in data.get("models", []) for i in m.get("loaded_instances", [])
                     if i.get("id") == "wealth-statements"), None)
    if not instance:
        raise ValueError("LM Studio did not expose the loaded statement model.")
    return {"id": instance["id"], "contextLength": instance.get("config", {}).get("context_length", context),
            "discovery": "/api/v1/models"}


def release_model(model, request_json):
    # Only unload our own instance. A model explicitly loaded for another app is
    # outside this lifecycle; TTL is also set as a fallback if the worker dies.
    if model.get("id") == "wealth-statements":
        try:
            request_json("/api/v1/models/unload", {"instance_id": model["id"]}, timeout=20)
        except (OSError, ValueError):
            pass
