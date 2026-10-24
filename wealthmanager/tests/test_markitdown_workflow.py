"""Exercise the installed converter and durable workflow using anonymized NSDL data."""
import base64
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ingest
import statement_library as library
from statement_parser import analyze as analyze_text

class MarkItDownWorkflowTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("markitdown") or (Path.home() / ".local/bin/markitdown").is_file(), "Installed MarkItDown CLI required for integration test")
    def test_nsdl_pdf_becomes_cached_markdown_and_reconciled_report(self):
        import fitz
        fixture = Path(__file__).parent / "fixtures" / "nsdl_values.md"
        expected = analyze_text(fixture.read_text(), "nsdl.md")
        lines = fixture.read_text().splitlines()
        document = fitz.open()
        for start in range(0, len(lines), 100):
            page = document.new_page(width=2000, height=1600)
            page.insert_text((30, 30), "\n".join(lines[start:start+100]), fontsize=7)
        content = document.tobytes()
        document.close()
        with tempfile.TemporaryDirectory() as directory, patch.object(library, "ROOT", Path(directory)), patch.object(library, "start_worker") as spawn:
            item = library.dispatch({"action":"upload", "filename":"fictional-nsdl.pdf", "content":base64.b64encode(content).decode()})
            library.process(item["id"])
            saved = library.dispatch({"action":"list"})[0]
            self.assertEqual(saved["status"], "ready", saved["error"])
            report = saved["report"]
            self.assertEqual(report["kind"], "investment")
            self.assertEqual(len(report["holdings"]), len(expected["holdings"]))
            self.assertEqual(report["summary"]["reportedValue"], expected["summary"]["reportedValue"])
            self.assertAlmostEqual(report["summary"]["parsedValue"], expected["summary"]["parsedValue"], places=2)
            self.assertEqual(len(report["history"]), len(expected["history"]))
            with library.connect() as db:
                converted = db.execute("SELECT converted,conversion_version FROM uploads WHERE id=?", (item["id"],)).fetchone()
                self.assertTrue(converted["converted"].strip())
                self.assertEqual(converted["conversion_version"], 4)
                self.assertEqual((Path(directory) / "markdown" / (item["id"] + ".md")).read_text(), converted["converted"])
            with patch("ingest.extract_pdf", side_effect=AssertionError("Must reuse cached Markdown")):
                library.dispatch({"action":"retry", "id":item["id"]})
                library.process(item["id"])
            duplicate = library.dispatch({"action":"upload", "filename":"renamed.pdf", "content":base64.b64encode(content).decode()})
            self.assertTrue(duplicate["cached"])
            self.assertEqual(len(library.dispatch({"action":"list"})), 1)

    def test_conversion_timeout_is_actionable(self):
        with patch("shutil.which", return_value="/local/markitdown"), patch("subprocess.run", side_effect=subprocess.TimeoutExpired("markitdown", 180)):
            with self.assertRaisesRegex(ValueError, "three minutes"):
                ingest.extract_pdf("fictional.pdf")

    def test_converter_errors_do_not_expose_document_text(self):
        result = subprocess.CompletedProcess([], 1, stderr=b"PRIVATE STATEMENT CONTENT")
        with patch("shutil.which", return_value="/local/markitdown"), patch("subprocess.run", return_value=result):
            with self.assertRaisesRegex(ValueError, "MarkItDown could not convert") as caught:
                ingest.extract_pdf("fictional.pdf")
            self.assertNotIn("PRIVATE", str(caught.exception))

    def test_protected_pdf_has_a_clear_error(self):
        result = subprocess.CompletedProcess([], 1, stderr=b"PDFPasswordIncorrect")
        with patch("shutil.which", return_value="/local/markitdown"), patch("subprocess.run", return_value=result):
            with self.assertRaisesRegex(ValueError, "password-protected"):
                ingest.extract_pdf("fictional.pdf")

    def test_retry_launch_failure_is_not_left_queued(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(library, "ROOT", Path(directory)), patch.object(library, "start_worker", side_effect=OSError("Cannot fork")):
            with self.assertRaises(ValueError):
                library.dispatch({"action":"upload", "filename":"fictional.md", "content":base64.b64encode(b"source").decode()})
            item = library.dispatch({"action":"list"})[0]
            with self.assertRaises(ValueError):
                library.dispatch({"action":"retry", "id":item["id"]})
            self.assertEqual(library.dispatch({"action":"list"})[0]["status"], "failed")

if __name__ == "__main__": unittest.main()
