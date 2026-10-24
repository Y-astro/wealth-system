"""Inspect whichever chat model is currently loaded; no hardcoded model IDs."""
import json
from local_ai import discover_model, request_json

if __name__ == "__main__":
    model = discover_model()
    response = request_json("/v1/chat/completions", {
        "model": model["id"], "temperature": 0, "max_tokens": 32,
        "messages": [{"role": "user", "content": "Return only the word TEST."}],
    }, timeout=60)
    print(json.dumps({"model": model["id"], "choices": response.get("choices", [])}, indent=2))
