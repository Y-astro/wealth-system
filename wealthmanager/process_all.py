"""Batch statement analysis. Optional explicit bank import; holdings remain snapshots."""
import argparse
import json
import os
import subprocess
import sys
import urllib.request
from pathlib import Path

from analyze_statement import analyze_statement
from local_ai import discover_model

SCRIPT_DIR = Path(__file__).resolve().parent


def get_lm_studio_model():
    """Compatibility helper for callers; never falls back to a hardcoded model."""
    return discover_model()["id"]


def wealthfolio_request(path, payload=None):
    base = os.environ.get("WEALTHFOLIO_BASE", "http://127.0.0.1:8080").rstrip("/")
    request = urllib.request.Request(base + "/api/v1" + path,
        data=json.dumps(payload).encode() if payload is not None else None,
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def import_bank(report):
    if report["kind"] != "bank":
        raise ValueError("Investment holdings are snapshots; open their report in Statements instead of importing fake trades.")
    if any("excluded" in warning for warning in report["warnings"]):
        raise ValueError("Review excluded/ambiguous rows in Statements before importing this bank report.")
    if abs(report["summary"].get("balanceDifference", 0)) > 0.05:
        raise ValueError("Bank balances do not reconcile; review source mappings before importing.")
    currencies = {t["currency"] for t in report["transactions"]}
    if len(currencies) != 1:
        raise ValueError("Bank activity import requires a single currency.")
    currency = next(iter(currencies))
    name = f"Statement - {report['filename']}"
    accounts = wealthfolio_request("/accounts")
    account = next((a for a in accounts if a["name"] == name and a["currency"] == currency), None)
    if account is None:
        account = wealthfolio_request("/accounts", {"name": name, "currency": currency, "accountType": "CASH", "isDefault": False, "isActive": True, "trackingMode": "TRANSACTIONS"})
    activities = [{"date": t["date"], "activityType": "DEPOSIT" if t["direction"] == "in" else "WITHDRAWAL",
                   "symbol": currency, "currency": currency, "quantity": 1, "unitPrice": t["amount"], "amount": t["amount"],
                   "fee": 0, "comment": t["description"], "accountId": account["id"], "isDraft": False, "isValid": True}
                  for t in report["transactions"]]
    opening = report["summary"].get("openingBalance")
    if opening:
        activities.insert(0, {"date": report["period"]["from"], "activityType": "DEPOSIT" if opening > 0 else "WITHDRAWAL",
            "symbol": currency, "currency": currency, "quantity": 1, "unitPrice": abs(opening), "amount": abs(opening),
            "fee": 0, "comment": "Opening Balance", "accountId": account["id"], "isDraft": False, "isValid": True})
    return wealthfolio_request("/activities/import", {"activities": activities})


def process_file(path, output_dir, use_ai=True, import_activities=False):
    if path.suffix.lower() in (".md", ".markdown", ".txt", ".csv"):
        text = path.read_text(encoding="utf-8-sig")
    else:
        result = subprocess.run([sys.executable, str(SCRIPT_DIR / "ingest.py"), str(path)], capture_output=True, text=True, check=True)
        text = result.stdout
    report = analyze_statement(text, path.name, use_ai)
    destination = output_dir / f"{path.name}.report.json"
    destination.write_text(json.dumps(report, ensure_ascii=False, allow_nan=False, indent=2) + "\n", encoding="utf-8")
    print(f"{path.name}: {report['kind']} report saved to {destination}")
    if import_activities:
        result = import_bank(report)
        print(json.dumps(result.get("summary", {})))
    return report


def main():
    parser = argparse.ArgumentParser(description="Generate chart reports from statement files, or every file in inputs/.")
    parser.add_argument("files", nargs="*", type=Path)
    parser.add_argument("--output-dir", type=Path, default=SCRIPT_DIR / "reports")
    parser.add_argument("--no-ai", action="store_true")
    parser.add_argument("--import-bank", action="store_true", help="Explicitly import bank cash activities into the running Wealthfolio server.")
    args = parser.parse_args()
    inputs = SCRIPT_DIR / "inputs"
    files = args.files or (sorted(p for p in inputs.iterdir() if p.suffix.lower() in (".pdf", ".xlsx", ".xls", ".csv", ".md", ".txt")) if inputs.exists() else [])
    if not files:
        parser.error("Pass a statement path or add files to wealthmanager/inputs/.")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    failures = 0
    for path in files:
        try:
            process_file(path, args.output_dir, not args.no_ai, args.import_bank)
        except (ValueError, OSError, subprocess.CalledProcessError) as error:
            failures += 1
            print(f"{path.name}: {error}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
