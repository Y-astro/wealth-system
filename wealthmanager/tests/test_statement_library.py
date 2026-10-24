"""Regression coverage for durable uploads; uses fictional statements only."""
import base64
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import statement_library as library

FIXTURE = Path(__file__).parent / "fixtures" / "bank_statement.md"

class StatementLibraryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = patch.object(library, "ROOT", Path(self.directory.name))
        self.root.start()
        self.worker = patch.object(library, "start_worker")
        self.spawn = self.worker.start()
    def tearDown(self):
        self.worker.stop()
        self.root.stop()
        self.directory.cleanup()
    def upload(self, content=None, name="bank.md"):
        if content is None: content = FIXTURE.read_bytes()
        return library.dispatch({"action":"upload", "filename":name, "content":base64.b64encode(content).decode()})
    def ready(self):
        queued = self.upload()
        library.process(queued["id"])
        item = library.dispatch({"action":"list"})[0]
        self.assertEqual(item["status"], "ready", item["error"])
        return item
    def test_duplicate_content_does_not_convert_again_even_when_renamed(self):
        queued = self.upload()
        again = self.upload(name="renamed.md")
        self.assertEqual(queued["id"], again["id"])
        self.assertEqual(self.spawn.call_count, 1)
        library.process(queued["id"])
        cached = self.upload(name="another-name.md")
        self.assertTrue(cached["cached"])
        self.assertEqual(self.spawn.call_count, 1)
        self.assertEqual(cached["report"]["summary"]["balanceDifference"], 0)
    def test_report_survives_new_connections_and_review_edits(self):
        item = self.ready()
        report = item["report"]
        report["transactions"][0]["category"] = "Reviewed salary"
        library.dispatch({"action":"save", "id":item["id"], "report":report})
        reloaded = library.dispatch({"action":"list"})[0]
        self.assertEqual(reloaded["report"]["transactions"][0]["category"], "Reviewed salary")
        library.dispatch({"action":"archive", "id":item["id"]})
        self.assertEqual(library.dispatch({"action":"list"}), [])
        restored = self.upload()
        self.assertTrue(restored["cached"])
        self.assertEqual(len(library.dispatch({"action":"list"})), 1)
    def test_pdf_conversion_and_reuse_after_failed_analysis(self):
        import fitz
        document = fitz.open()
        page = document.new_page()
        page.insert_text((40,40), FIXTURE.read_text(), fontsize=8)
        content = document.tobytes()
        queued = self.upload(content, "bank.pdf")
        with patch("analyze_statement.analyze_statement", side_effect=ValueError("Local model unavailable")):
            library.process(queued["id"])
        self.assertEqual(library.dispatch({"action":"list"})[0]["status"], "failed")
        retried = self.upload(content, "bank.pdf")
        with patch("ingest.extract_pdf", side_effect=AssertionError("Already converted")):
            library.process(retried["id"])
        result = library.dispatch({"action":"list"})[0]
        self.assertEqual(result["status"], "ready", result["error"])
    def test_excel_converts_to_readable_bank_rows(self):
        import pandas as pd
        data = pd.DataFrame([
            ["01-Apr-2026", "Payroll salary", 0, 80000, 90000],
            ["02-Apr-2026", "Monthly rent", 20000, 0, 70000],
        ], columns=["Date", "Narration", "Withdrawal", "Deposit", "Balance"])
        output = io.BytesIO()
        data.to_excel(output, index=False)
        queued = self.upload(output.getvalue(), "bank.xlsx")
        library.process(queued["id"])
        result = library.dispatch({"action":"list"})[0]
        self.assertEqual(result["status"], "ready", result["error"])
        self.assertEqual(len(result["report"]["transactions"]), 2)
    def test_unsupported_empty_and_oversize_files_are_rejected(self):
        for content, name in [(b"", "empty.pdf"), (b"abc", "program.exe"), (b"x"*(library.MAX_BYTES + 1), "large.txt")]:
            with self.assertRaises(ValueError): self.upload(content, name)
        self.assertEqual(library.dispatch({"action":"list"}), [])
    def test_upload_above_previous_twenty_mb_limit(self):
        result = self.upload(b"x" * 20_000_001, "large.csv")
        self.assertEqual(result["status"], "queued")
        with library.connect() as db:
            self.assertEqual(db.execute("SELECT length(content) FROM uploads WHERE id=?", (result["id"],)).fetchone()[0], 20_000_001)

    def test_streamed_private_upload_buffer(self):
        with tempfile.NamedTemporaryFile(prefix="wealth-upload-") as file:
            file.write(FIXTURE.read_bytes())
            file.flush()
            result = library.dispatch({"action":"upload", "filename":"bank.md", "uploadPath":file.name})
        self.assertEqual(result["status"], "queued")
        library.process(result["id"])
        self.assertEqual(library.dispatch({"action":"list"})[0]["status"], "ready")

    def test_imported_report_and_invalid_report(self):
        ready = self.ready()["report"]
        queued = self.upload(json.dumps(ready).encode(), "saved.report.json")
        library.process(queued["id"])
        self.assertEqual(library.dispatch({"action":"list"})[0]["status"], "ready")
        invalid = self.upload(b'{"schemaVersion":1}', "bad.json")
        library.process(invalid["id"])
        self.assertEqual(library.dispatch({"action":"list"})[0]["status"], "failed")

    def test_legacy_browser_report_can_be_migrated_without_original_file(self):
        report = self.ready()["report"]
        report["id"] = "b" * 64
        library.dispatch({"action":"save", "id":report["id"], "report":report})
        migrated = next(item for item in library.dispatch({"action":"list"}) if item["id"] == report["id"])
        self.assertEqual(migrated["report"], report)
    def test_worker_launch_failure_is_saved_and_retryable(self):
        self.spawn.side_effect = OSError("Cannot fork")
        with self.assertRaisesRegex(ValueError, "Cannot start"):
            self.upload()
        self.assertEqual(library.dispatch({"action":"list"})[0]["status"], "failed")
        self.spawn.side_effect = None
        self.assertEqual(self.upload()["status"], "queued")

if __name__ == "__main__": unittest.main()
