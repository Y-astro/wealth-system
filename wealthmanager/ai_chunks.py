"""Small, resumable local extraction requests. Checkpoints stay on disk."""
import hashlib
import json
import os
from pathlib import Path
import re


def checkpoint_root(text, model, prompt):
    base = Path(os.environ.get("WEALTH_STATEMENT_LIBRARY", str(Path(__file__).resolve().parent / "data/statements")))
    key = hashlib.sha256((text + "\n" + model["id"] + "\n" + prompt + "\nv2").encode()).hexdigest()
    folder = base / "ai-checkpoints" / key
    folder.mkdir(parents=True, exist_ok=True, mode=0o700)
    return folder


def chunk_requests(model, chunks, headers, folder, extract, error_type):
    def run(lines):
        text = "Column/context hints (do not extract these as rows):\n" + "\n".join(headers[:8]) + "\nExtract only this chunk:\n" + "\n".join(lines)
        key = hashlib.sha256(text.encode()).hexdigest()
        file = folder / (key + ".json")
        if file.exists():
            try:
                result = json.loads(file.read_text())
            except (ValueError, OSError):
                result = None
        else:
            result = None
        if result is None:
            try:
                result = extract(model, text)
            except error_type as error:
                # Retry smaller requests on truncation/timeouts, never accept a
                # half JSON response. Split at date-led transaction boundaries.
                if not re.search(r"truncat|timed out|disconnected", str(error), re.I):
                    raise
                boundaries = [i for i, line in enumerate(lines) if i and re.match(r'L\d+\s+[|"\s]*(?:\d{4}-\d{2}-\d{2}|\d{1,2}[-/](?:[A-Za-z]{3}|\d{1,2})[-/]\d{2,4})', line)]
                if not boundaries:
                    raise error_type("One transaction could not fit or finish in the local model. Export this statement as CSV/Excel to parse it without AI.") from error
                middle = min(boundaries, key=lambda i: abs(i - len(lines) / 2))
                yield from run(lines[:middle])
                yield from run(lines[middle:])
                return
        # Caller validates dates, amounts and original line references before
        # resuming this generator; only then is the chunk persisted.
        yield lines, result
        temporary = file.with_suffix(".tmp")
        with temporary.open("w", encoding="utf-8") as stream:
            os.chmod(temporary, 0o600)
            json.dump(result, stream, allow_nan=False)
        temporary.replace(file)
    for lines in chunks:
        yield from run(lines)
