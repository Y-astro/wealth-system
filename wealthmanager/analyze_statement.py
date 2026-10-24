#!/usr/bin/env python3
"""CLI and JSON-stdin bridge used by the app's statement analysis endpoint."""
import argparse
import json
import sys
from pathlib import Path

from local_ai import discover_model, extract_bank
from statement_parser import analyze


def analyze_statement(text, filename, use_ai=True):
    report = analyze(text, filename)
    if report["kind"] == "bank" and not report["transactions"]:
        if not use_ai:
            raise ValueError("No bank rows could be mapped. Use date/narration/debit/credit headers, or enable local AI for this layout.")
        report = extract_bank(text, filename)
    return report


def main():
    parser = argparse.ArgumentParser(description="Convert Markdown/CSV statements into a versioned chart report with original source mappings.")
    parser.add_argument("file", nargs="?", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--no-ai", action="store_true", help="Never call LM Studio; parse supported tables only.")
    parser.add_argument("--stdin-json", action="store_true", help="Read {text, filename, useAi} on stdin.")
    parser.add_argument("--model", action="store_true", help="Inspect the currently loaded local model.")
    args = parser.parse_args()
    try:
        if args.model:
            result = discover_model()
        elif args.stdin_json:
            payload = json.load(sys.stdin)
            if not isinstance(payload.get("text"), str):
                raise ValueError("text must be a UTF-8 statement string.")
            result = analyze_statement(payload["text"], payload.get("filename", "statement.md"), payload.get("useAi", True))
        elif args.file:
            result = analyze_statement(args.file.read_text(encoding="utf-8-sig"), args.file.name, not args.no_ai)
        else:
            parser.error("Pass a statement file or --stdin-json.")
        output = json.dumps(result, ensure_ascii=False, allow_nan=False, indent=2)
        if args.output:
            args.output.write_text(output + "\n", encoding="utf-8")
        else:
            print(output)
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
