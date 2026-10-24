import tempfile
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ingest import extract_excel
from analyze_statement import analyze_statement

class SpreadsheetLayoutTests(unittest.TestCase):
    def convert(self, rows):
        import pandas as pd
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "fictional-bank.xlsx"
            pd.DataFrame(rows).to_excel(file, index=False, header=False)
            text = extract_excel(str(file))
        return text, analyze_statement(text, "fictional-bank.xlsx", False)

    def test_au_metadata_headers_and_space_separated_dates(self):
        text, report = self.convert([
            ["ACCOUNT STATEMENT"],
            ["Opening Balance (₹)", 1000],
            ["Closing Balance (₹)", 1150],
            ["Transaction Date", "Value Date", "Description/Narration", "Cheque/Reference No", "Debit (₹)", "Credit (₹)", "Balance (₹)"],
            ["05 Apr 2025", "05 Apr 2025", "Payroll salary", "", 0, 200, 1200],
            ["06 Apr 2025", "06 Apr 2025", "Supermarket groceries", "", 50, 0, 1150],
        ])
        self.assertEqual(len(report["transactions"]), 2)
        self.assertEqual(report["transactions"][0]["date"], "2025-04-05")
        self.assertEqual(report["summary"]["balanceDifference"], 0)
        self.assertIn("Source row", text)

    def test_axis_aliases_and_long_footers_do_not_reach_the_model(self):
        text, report = self.convert([
            ["Account statement"],
            ["Opening Balance", 1000],
            ["Closing Balance", 850],
            ["SRL No", "Tran Date", "CHQNO", "PARTICULARS", "DR", "CR", "BAL", "SOL"],
            [1, "12-07-2025", "", "Monthly rent", 100, "", 900, 227],
            [2, "13-07-2025", "", "Supermarket", 50, "", 850, 227],
            ["Long footer " * 2000],
        ])
        self.assertEqual(len(report["transactions"]), 2)
        self.assertEqual(report["summary"]["balanceDifference"], 0)
        self.assertEqual([row["balance"] for row in report["transactions"]], [900, 850])
        self.assertLess(len(text), 1000)

    def test_header_currency_is_preserved(self):
        _, report = self.convert([
            ["Date", "Narration", "Debit (USD)", "Credit (USD)", "Balance (USD)"],
            ["01-Apr-2026", "Salary", "", 200, 1200],
        ])
        self.assertEqual(report["currency"], "USD")
        self.assertEqual(report["transactions"][0]["currency"], "USD")

    def test_unrecognized_transaction_dates_are_explicitly_flagged(self):
        text, report = self.convert([
            ["Date", "Narration", "Debit", "Credit", "Balance"],
            ["01-Apr-2026", "Salary", "", 200, 1200],
            ["31-Feb-2026", "Invalid date row", 10, "", 1190],
        ])
        self.assertIn("excluded spreadsheet row", text)
        self.assertEqual(len(report["transactions"]), 1)

if __name__ == "__main__": unittest.main()
