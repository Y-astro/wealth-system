"""Durable local statement uploads. JSON bridge for the authenticated web API."""
from __future__ import annotations
import base64
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import sys
import time

ROOT = Path(os.environ.get("WEALTH_STATEMENT_LIBRARY", str(Path(__file__).resolve().parent / "data" / "statements")))
MAX_BYTES = 50_000_000
EXTENSIONS = {".pdf", ".xlsx", ".xls", ".csv", ".md", ".markdown", ".txt", ".json"}

@contextmanager
def connect():
    ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    db = sqlite3.connect(ROOT / "library.sqlite3", timeout=30)
    db.row_factory = sqlite3.Row
    db.execute("PRAGMA journal_mode=WAL")
    db.execute("CREATE TABLE IF NOT EXISTS uploads (id TEXT PRIMARY KEY, filename TEXT NOT NULL, content BLOB NOT NULL, converted TEXT, report TEXT, status TEXT NOT NULL, error TEXT, updated REAL NOT NULL, archived INTEGER NOT NULL DEFAULT 0)")
    if "conversion_version" not in {row[1] for row in db.execute("PRAGMA table_info(uploads)")}:
        db.execute("ALTER TABLE uploads ADD COLUMN conversion_version INTEGER NOT NULL DEFAULT 1")
    try:
        with db:
            yield db
    finally:
        db.close()

def item(row):
    return {"id": row["id"], "filename": row["filename"], "status": row["status"], "error": row["error"], "report": json.loads(row["report"]) if row["report"] else None}

def validate_report(report):
    if not isinstance(report, dict) or report.get("schemaVersion") != 1 or report.get("kind") not in ("bank", "investment"):
        raise ValueError("Choose a version 1 statement report, or upload the original statement.")
    for key in ("source", "mappings", "warnings", "holdings", "transactions"):
        if not isinstance(report.get(key), list):
            raise ValueError("The statement report is incomplete; upload the original statement.")
    if not isinstance(report.get("summary"), dict):
        raise ValueError("The statement report has no valid summary.")
    return report

def start_worker(identifier):
    subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--process", identifier], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)

def dispatch(payload):
    action = payload.get("action")
    with connect() as db:
        if action == "list":
            # Interrupted jobs become retryable, rather than remaining busy forever.
            db.execute("UPDATE uploads SET status='failed', error='Processing was interrupted. Upload the same file to retry.' WHERE status IN ('queued','processing') AND updated < ?", (time.time() - 900,))
            return [item(row) for row in db.execute("SELECT id,filename,status,error,report FROM uploads WHERE archived=0 ORDER BY updated DESC")]
        if action == "retry":
            identifier = str(payload.get("id", ""))
            if not re.fullmatch(r"[a-f0-9]{64}", identifier):
                raise ValueError("Invalid statement identifier.")
            row = db.execute("SELECT * FROM uploads WHERE id=? AND archived=0", (identifier,)).fetchone()
            if not row:
                raise ValueError("Saved original file was not found.")
            if row["status"] in ("queued", "processing"):
                return item(row)
            db.execute("UPDATE uploads SET status='queued',error=NULL,updated=? WHERE id=?", (time.time(), identifier))
            db.commit()
            try:
                start_worker(identifier)
            except OSError:
                db.execute("UPDATE uploads SET status='failed',error='Cannot start local processing. Retry the saved file.' WHERE id=?", (identifier,))
                db.commit()
                raise ValueError("Cannot start local processing. Retry the saved file.")
            return {**item(row), "status":"queued", "error":None}
        if action == "upload":
            filename = Path(str(payload.get("filename", "statement"))).name
            suffix = Path(filename).suffix.lower()
            if suffix not in EXTENSIONS:
                raise ValueError("Supported files: PDF, Excel, CSV, Markdown, text, and exported report JSON.")
            if "uploadPath" in payload:
                # Internal Rust bridge only; no public endpoint accepts a path.
                import tempfile
                path = Path(payload["uploadPath"])
                if path.parent != Path(tempfile.gettempdir()) or not path.name.startswith("wealth-upload-") or path.is_symlink():
                    raise ValueError("Invalid local statement upload buffer.")
                if path.stat().st_size > MAX_BYTES:
                    raise ValueError("Choose a statement up to 50 MB.")
                content = path.read_bytes()
            else:
                content = base64.b64decode(payload["content"], validate=True)
            if not content or len(content) > MAX_BYTES:
                raise ValueError("Choose a nonempty statement up to 50 MB.")
            identifier = hashlib.sha256(content).hexdigest()
            row = db.execute("SELECT * FROM uploads WHERE id=?", (identifier,)).fetchone()
            if row and row["status"] in ("ready", "queued", "processing"):
                db.execute("UPDATE uploads SET archived=0 WHERE id=?", (identifier,))
                return {**item(row), "cached": row["status"] == "ready"}
            db.execute("INSERT INTO uploads (id,filename,content,status,updated) VALUES (?,?,?,'queued',?) ON CONFLICT(id) DO UPDATE SET status='queued',error=NULL,archived=0,updated=excluded.updated", (identifier, filename, content, time.time()))
            db.commit()
            try:
                start_worker(identifier)
            except OSError:
                db.execute("UPDATE uploads SET status='failed',error='Cannot start local processing. Upload the file again to retry.' WHERE id=?", (identifier,))
                db.commit()
                raise ValueError("Cannot start local processing. Upload the file again to retry.")
            return {"id": identifier, "filename": filename, "status": "queued", "report": None, "error": None, "cached": False}
        identifier = str(payload.get("id", ""))
        if not re.fullmatch(r"[a-f0-9]{64}", identifier):
            raise ValueError("Invalid statement identifier.")
        row = db.execute("SELECT * FROM uploads WHERE id=?", (identifier,)).fetchone()
        if not row and action == "save":
            report = validate_report(payload["report"])
            if report.get("id") != identifier:
                raise ValueError("The report identifier does not match this statement.")
            encoded = json.dumps(report, allow_nan=False)
            db.execute("INSERT INTO uploads (id,filename,content,report,status,updated) VALUES (?,?,?,?,'ready',?)", (identifier, Path(str(report.get("filename", "saved.report.json"))).name, encoded.encode(), encoded, time.time()))
            return {"ok": True}
        if not row:
            raise ValueError("Statement was not found in the saved library.")
        if action == "save":
            if row["status"] != "ready":
                raise ValueError("This statement is still processing. Keep the browser report and retry after processing completes.")
            report = validate_report(payload["report"])
            if report.get("id") != identifier:
                raise ValueError("The report identifier does not match this statement.")
            db.execute("UPDATE uploads SET report=?,updated=? WHERE id=? AND status='ready'", (json.dumps(report, allow_nan=False), time.time(), identifier))
        elif action == "archive":
            db.execute("UPDATE uploads SET archived=1 WHERE id=?", (identifier,))
        else:
            raise ValueError("Unknown library action.")
        return {"ok": True}

def _process_locked(identifier):
    import fcntl
    from analyze_statement import analyze_statement
    with connect() as db:
        claimed = db.execute("UPDATE uploads SET status='processing',updated=? WHERE id=? AND status='queued'", (time.time(), identifier)).rowcount
        db.commit()
        if not claimed:
            return
        row = db.execute("SELECT * FROM uploads WHERE id=?", (identifier,)).fetchone()
    try:
        with connect() as db:
            db.execute("UPDATE uploads SET updated=? WHERE id=?", (time.time(), identifier))
        suffix = Path(row["filename"]).suffix.lower()
        if suffix == ".json":
            report = validate_report(json.loads(row["content"]))
            text = ""
        else:
            conversion_version = 4 if suffix == ".pdf" else 3
            text = row["converted"] if row["conversion_version"] >= conversion_version else None
            if text is None:
                if suffix in (".pdf", ".xls", ".xlsx"):
                    import tempfile
                    from ingest import extract_pdf, extract_excel
                    with tempfile.TemporaryDirectory(prefix="wealth-statement-") as directory:
                        path = Path(directory) / ("source" + suffix)
                        path.write_bytes(row["content"])
                        text = extract_pdf(str(path)) if suffix == ".pdf" else extract_excel(str(path))
                else:
                    text = bytes(row["content"]).decode("utf-8-sig")
                if not text.strip():
                    raise ValueError("No readable text was found. Scanned PDFs need OCR; password-protected PDFs must be unlocked first.")
                with connect() as db:
                    db.execute("UPDATE uploads SET converted=?,conversion_version=? WHERE id=?", (text, conversion_version, identifier))
            if suffix == ".pdf":
                markdown_directory = ROOT / "markdown"
                markdown_directory.mkdir(mode=0o700, exist_ok=True)
                temporary = markdown_directory / (identifier + ".md.tmp")
                temporary.write_text(text, encoding="utf-8")
                temporary.chmod(0o600)
                temporary.replace(markdown_directory / (identifier + ".md"))
            # Existing deterministic extraction is tried first; local AI handles unknown layouts.
            report = analyze_statement(text, row["filename"], True)
            report["warnings"].extend(line for line in text.splitlines() if line.startswith("excluded spreadsheet row "))
        report["id"] = identifier
        report["filename"] = row["filename"]
        if report["kind"] == "bank":
            from statement_parser import finalize_bank
            finalize_bank(report)
        with connect() as db:
            db.execute("UPDATE uploads SET report=?,status='ready',error=NULL,updated=? WHERE id=?", (json.dumps(report, allow_nan=False), time.time(), identifier))
    except Exception as error:
        # Do not log statement contents or model output.
        message = "Statement conversion failed. Check that the file is readable and the local model is available."
        if isinstance(error, (ValueError, UnicodeError)):
            message = str(error)[:350]
        elif isinstance(error, ImportError):
            message = "A local PDF/Excel converter dependency is missing. Install PyMuPDF, pandas, openpyxl, or xlrd as appropriate."
        with connect() as db:
            db.execute("UPDATE uploads SET status='failed',error=?,updated=? WHERE id=?", (message, time.time(), identifier))

def process(identifier):
    import fcntl
    import threading
    stopped = threading.Event()
    def heartbeat():
        while not stopped.wait(20):
            try:
                with connect() as db:
                    db.execute("UPDATE uploads SET updated=? WHERE id=? AND status IN ('queued','processing')", (time.time(), identifier))
            except sqlite3.Error:
                pass
    thread = threading.Thread(target=heartbeat, daemon=True)
    thread.start()
    try:
        # Keep queued uploads alive and claim them only when the local converter is free.
        with (ROOT / "worker.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            _process_locked(identifier)
    finally:
        stopped.set()


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--process":
        process(sys.argv[2])
        return
    try:
        print(json.dumps(dispatch(json.load(sys.stdin)), allow_nan=False))
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(json.dumps({"error": str(error)[:350]}), file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
