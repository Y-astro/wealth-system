import json
import sys
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from analyze_statement import analyze_statement
from local_ai import LocalAIError, discover_model, extract_bank, extract_chunk, parse_json_content
from statement_parser import analyze, normalize_line, number, parse_bank, parse_nsdl

FIXTURES = Path(__file__).parent / "fixtures"


class NsdlTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = parse_nsdl((FIXTURES / "nsdl_values.md").read_text(), "nsdl_values.md")

    def test_all_values_reconcile(self):
        self.assertEqual(self.report["summary"]["reportedValue"], 10000)
        self.assertEqual(self.report["summary"]["parsedValue"], 10000)
        self.assertEqual(self.report["summary"]["difference"], 0)
        self.assertEqual(len(self.report["holdings"]), 5)
        self.assertEqual(len(self.report["history"]), 13)
        self.assertEqual(self.report["history"][-1]["value"], 10000)
        for account in self.report["accounts"]:
            if not account["id"].startswith("summary-") and account["reportedValue"] is not None:
                self.assertEqual(account["value"], account["reportedValue"])

    def test_folio_value_is_not_gain(self):
        holding = next(h for h in self.report["holdings"] if h["isin"] == "INF000D01004")
        self.assertEqual(holding["value"], 5000)
        self.assertEqual(holding["cost"], 3000)
        self.assertEqual(holding["unrealizedGain"], 2000)
        self.assertIn("Direct Plan", holding["name"])

    def test_face_value_pledge_and_rounded_nav(self):
        holding = self.report["holdings"][0]
        self.assertEqual(holding["faceValue"], 10)
        self.assertEqual(holding["quantity"], 10)
        self.assertEqual(holding["price"], 100)
        self.assertEqual(holding["name"], "DEMO ALPHA LIMITED")
        silver = next(h for h in self.report["holdings"] if h["value"] == 2000 and h["isin"] == "INF000C01003")
        self.assertEqual(silver["quantity"], 100)
        self.assertEqual(silver["pledgedQuantity"], 50)
        self.assertNotEqual(silver["value"], silver["quantity"] * silver["price"])

    def test_zero_holdings_and_cross_account_isin_preserved(self):
        self.assertEqual(len([h for h in self.report["holdings"] if h["isin"] == "INE000A01001"]), 2)
        self.assertEqual(next(h for h in self.report["holdings"] if h["isin"] == "INE000B01002")["quantity"], 0)

    def test_demats_are_units_not_bank_cash(self):
        securities = [t for t in self.report["transactions"] if t["unit"] == "units"]
        self.assertEqual(len(securities), 2)
        self.assertTrue(all(t["amount"] is None and t["currency"] is None for t in securities))
        self.assertEqual(len([t for t in self.report["transactions"] if t["category"] == "Pension fees"]), 3)

    def test_original_evidence_and_dummy_id_mismatch(self):
        self.assertTrue(any("identifiers differ" in w for w in self.report["warnings"]))
        source = {row["line"]: row for row in self.report["source"]}
        for mapping in self.report["mappings"]:
            self.assertTrue(all(line in source for line in mapping["sourceLines"]))
        for holding in self.report["holdings"]:
            if holding["isin"]:
                self.assertIn(holding["isin"], source[holding["sourceLines"][0]]["text"])
        self.assertEqual(self.report["source"][0]["text"], (FIXTURES / "nsdl_values.md").read_text().splitlines()[0])

    def test_normalization_does_not_damage_names_or_numbers(self):
        self.assertEqual(normalize_line("CCoonnssoolliiddaatteedd AAccccoouunntt SSttaatteemmeenntt"), "Consolidated Account Statement")
        self.assertEqual(normalize_line("ALLIED 1,000.000 2200"), "ALLIED 1,000.000 2200")

    def test_unknown_nsdl_layout_fails_without_guessing(self):
        with self.assertRaisesRegex(ValueError, "could not map"):
            parse_nsdl("NSDL Consolidated Account Statement\nNo holdings table")


class BankTests(unittest.TestCase):
    def test_bank_amounts_not_running_balance(self):
        report = analyze((FIXTURES / "bank_statement.md").read_text())
        self.assertEqual(report["kind"], "bank")
        self.assertEqual(len(report["transactions"]), 8)
        self.assertEqual(report["transactions"][0]["amount"], 80000)
        self.assertEqual(report["summary"]["income"], 81200)
        self.assertEqual(report["summary"]["expenses"], 27500)
        self.assertEqual(report["summary"]["netCashFlow"], 38700)
        self.assertEqual(report["summary"]["balanceDifference"], 0)
        self.assertEqual(len([t for t in report["transactions"] if t["flowType"] == "transfer"]), 2)

    def test_csv_quoted_indian_values_and_negative_direction(self):
        report = parse_bank('Date,Narration,Amount,DrCr,Balance\n01/04/2026,Salary,"1,20,000.00",CR,"1,30,000.00"\n02/04/2026,Groceries,-2000,,128000')
        self.assertEqual([t["amount"] for t in report["transactions"]], [120000,2000])
        self.assertEqual(report["transactions"][1]["direction"], "out")
        self.assertEqual(report["summary"]["openingBalance"], 10000)

    def test_ambiguous_rows_excluded_and_flagged(self):
        report = parse_bank('Date|Description|Debit|Credit|Balance\n01-Apr-2026|Both|100|200|300\n02-Apr-2026|UPI to friend|50||250')
        self.assertEqual(len(report["transactions"]), 1)
        self.assertTrue(any("excluded" in w for w in report["warnings"]))
        self.assertEqual(report["transactions"][0]["flowType"], "expense")

    def test_positive_amount_without_direction_not_guessed(self):
        report = parse_bank('Date|Description|Amount|Balance\n01-Apr-2026|Unknown|100|900')
        self.assertEqual(len(report["transactions"]), 0)
        self.assertTrue(any("without debit/credit" in w for w in report["warnings"]))

    def test_multi_currency_not_aggregated(self):
        report = parse_bank('Date|Description|Debit|Credit|Currency\n01-Apr-2026|Salary||1000|USD\n02-Apr-2026|Salary||1000|INR')
        self.assertEqual(report["currencies"], ["INR", "USD"])
        self.assertNotIn("income", report["summary"])

    def test_reverse_order_uses_reconciled_running_balances(self):
        report = parse_bank('Date|Description|Debit|Credit|Balance\n02-Apr-2026|Salary||100|200\n01-Apr-2026|Rent|50||100')
        self.assertEqual(report["summary"]["openingBalance"], 150)
        self.assertEqual(report["summary"]["closingBalance"], 200)
        self.assertEqual(report["summary"]["balanceDifference"], 0)
        inconsistent = parse_bank('Date|Description|Debit|Credit|Balance\n02-Apr-2026|Salary||100|999\n01-Apr-2026|Rent|50||100')
        self.assertNotIn("openingBalance", inconsistent["summary"])

    def test_currency_declaration_wins_over_narrations_and_footnotes(self):
        report = parse_bank('Currency: USD\nDate|Description|Debit|Credit|Balance\n01-Apr-2026|INR wire to India|100||900\nINR and EUR not accepted for this account')
        self.assertEqual(report["transactions"][0]["currency"], "USD")
        header = parse_bank('Date|Description|Debit (EUR)|Credit (EUR)|Balance\n01-Apr-2026|GBP conversion||100|900')
        self.assertEqual(header["transactions"][0]["currency"], "EUR")

    def test_invalid_dates_and_reversals_are_not_silently_accepted(self):
        report = parse_bank('Date|Description|Debit|Credit|Balance\n31-Apr-2026|Invalid|10||100\n02-Apr-2026|Reversal|-5||105')
        self.assertEqual(report["transactions"], [])
        self.assertEqual(len([w for w in report["warnings"] if "excluded" in w]), 2)

    def test_unknown_layout_offline_explains_action(self):
        with self.assertRaisesRegex(ValueError, "headers"):
            analyze_statement("This is an unknown layout", "bank.md", False)

    def test_empty_and_nonfinite_rejected(self):
        self.assertIsNone(number("NaN"))
        self.assertIsNone(number("Infinity"))
        self.assertIsNone(number(True))
        with self.assertRaises(ValueError):
            analyze("")


class ModelTests(unittest.TestCase):
    def setUp(self):
        import tempfile
        from pathlib import Path
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        counter = [0]
        def fresh_checkpoint(*args):
            counter[0] += 1
            folder = Path(directory.name) / str(counter[0])
            folder.mkdir()
            return folder
        cache = patch("local_ai.checkpoint_root", side_effect=fresh_checkpoint)
        cache.start()
        self.addCleanup(cache.stop)

    @patch.dict("os.environ", {}, clear=True)
    @patch("local_ai.request_json")
    def test_native_uses_loaded_chat_instance_not_first_download(self, request):
        request.return_value = {"models": [{"key": "downloaded", "type": "llm", "loaded_instances": []},
            {"type": "embedding", "loaded_instances": [{"id": "embed"}]},
            {"type": "llm", "key": "bonsai", "loaded_instances": [{"id": "bonsai-live", "config": {"context_length": 8192}}]}]}
        self.assertEqual(discover_model()["id"], "bonsai-live")
        self.assertEqual(discover_model()["contextLength"], 8192)

    @patch.dict("os.environ", {}, clear=True)
    @patch("local_ai.request_json")
    def test_empty_native_does_not_autoload_downloaded_model(self, request):
        request.return_value = {"models": [{"type": "llm", "key": "qwen", "loaded_instances": []}]}
        with self.assertRaisesRegex(LocalAIError, "No loaded"):
            discover_model()
        self.assertEqual(request.call_count, 1)

    @patch.dict("os.environ", {}, clear=True)
    @patch("local_ai.request_json")
    def test_legacy_fallback_ignores_embeddings(self, request):
        missing = urllib.error.HTTPError("url", 404, "missing", {}, None)
        request.side_effect = [missing, missing, {"data": [{"id": "embedding-model"}, {"id": "any-current-model"}]}]
        self.assertEqual(discover_model()["id"], "any-current-model")

    @patch.dict("os.environ", {}, clear=True)
    @patch("local_ai.request_json")
    def test_v0_selects_loaded_state(self, request):
        request.side_effect = [urllib.error.HTTPError("url", 404, "missing", {}, None), {"data": [
            {"id": "other", "type": "llm", "state": "not-loaded"}, {"id": "bonsai", "type": "llm", "state": "loaded"}]}]
        self.assertEqual(discover_model()["id"], "bonsai")

    def test_reasoning_and_fences_removed_without_greedy_slicing(self):
        self.assertEqual(parse_json_content('<think>{fake}</think>```json\n{"transactions":[]}\n```')["transactions"], [])

    @patch("local_ai.request_json")
    def test_json_mode_rejected_once_then_plain_fallback(self, request):
        request.side_effect = [urllib.error.HTTPError("url", 400, "unsupported", {}, None), {"choices": [{"finish_reason": "stop", "message": {"content": '{"transactions":[]}'}}]}]
        extract_chunk({"id": "bonsai", "contextLength": 4096}, "L1 source")
        self.assertNotIn("response_format", request.call_args.args[1])
        self.assertEqual(request.call_count, 2)

    @patch("local_ai.discover_model", return_value={"id": "bonsai", "contextLength": 4096})
    @patch("local_ai.extract_chunk")
    def test_balance_labels_do_not_depend_on_model_completeness(self, extract, model):
        extract.return_value = {"transactions": [{"date":"2026-04-01", "description":"Salary", "direction":"in", "amount":100, "currency":"INR", "sourceLines":[2]}], "openingBalance": None, "closingBalance": None}
        report = extract_bank("Opening Balance: 1000.00\n01-Apr-2026 Salary credit 100\nClosing Balance: 1100.00", "bank.md")
        self.assertEqual(report["summary"]["openingBalance"], 1000)
        self.assertEqual(report["summary"]["closingBalance"], 1100)
        self.assertEqual(report["summary"]["balanceDifference"], 0)

    @patch("local_ai.request_json")
    def test_truncation_not_accepted(self, request):
        request.return_value = {"choices": [{"finish_reason": "length", "message": {"content": '{"transactions":[]}'}}]}
        with self.assertRaisesRegex(LocalAIError, "truncated"):
            extract_chunk({"id": "bonsai", "contextLength": 4096}, "L1 source")

    @patch("local_ai.discover_model", return_value={"id": "bonsai", "contextLength": 4096})
    @patch("local_ai.extract_chunk")
    def test_ai_source_evidence_and_no_invented_amount(self, extract, model):
        txn = {"date":"2026-04-01", "description":"Salary", "direction":"in", "amount":100, "currency":"INR", "sourceLines":[1]}
        extract.return_value = {"transactions": [txn]}
        report = extract_bank("01-Apr-2026 Salary credit 100 balance 900", "bank.md")
        self.assertEqual(report["extraction"]["model"], "bonsai")
        txn["amount"] = 9999
        with self.assertRaisesRegex(LocalAIError, "absent"):
            extract_bank("01-Apr-2026 Salary credit 100 balance 900", "bank.md")
        txn["sourceLines"] = [99]
        with self.assertRaisesRegex(LocalAIError, "source references"):
            extract_bank("01-Apr-2026 Salary credit 100 balance 900", "bank.md")


if __name__ == "__main__":
    unittest.main()
