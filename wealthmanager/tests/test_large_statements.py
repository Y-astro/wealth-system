"""Fictional fixtures; no financial details from uploaded files."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from statement_parser import analyze
from local_ai import _extract_bank, LocalAIError, extract_bank, extract_chunk
from ai_chunks import chunk_requests
from model_runtime import load_for_statements, release_model

HEADER = '"Date","Sr.No.","Transaction Details","Reward Point Header","Intl.Amount","Amount(in Rs)","BillingAmountSign"\n'

class LargeStatementTests(unittest.TestCase):
    def test_icici_billed_amount_direction_repayment_and_source_lines(self):
        text = '"Account No:","fictional"\n' + HEADER + '"XXXX0000"\n' + (
            '"16/04/2025","1","SHOP, MARKET","999","500","760.00",""\n'
            '"17/04/2025","2","BBPS Payment received","0","0","1000","CR"\n'
            '"18/04/2025","3","Fuel surcharge reversal","0","0","2.50","CR"\n'
            '"XXXX1111"\n'
            '"19/04/2025","4","SHOP | multi\nline narration","10","900","123.45",""\n')
        report = analyze(text, 'card.CSV')
        rows = report['transactions']
        self.assertEqual(len(rows), 4)
        self.assertEqual([r['amount'] for r in rows], [760, 1000, 2.5, 123.45])
        self.assertEqual([r['direction'] for r in rows], ['out', 'in', 'in', 'out'])
        self.assertEqual(rows[1]['flowType'], 'transfer')
        self.assertEqual(rows[-1]['sourceLines'], [8, 9])
        self.assertEqual(rows[0]['currency'], 'INR')
        self.assertFalse(any('INR assumed' in w for w in report['warnings']))

    def test_generic_blank_direction_still_rejected(self):
        report = analyze('Date,Description,Amount,Type\n01/04/2026,Unknown,100,\n', 'unknown.csv')
        self.assertEqual(report['transactions'], [])

    def test_csv_larger_than_old_two_mb_limit_without_ai(self):
        text = 'Date,Narration,Debit,Credit,Balance\n' + (
            '01/04/2026,"Fictional merchant ' + 'x' * 80 + '",1,0,\n') * 18000
        self.assertGreater(len(text.encode()), 2_000_000)
        report = analyze(text, 'large.csv')
        self.assertEqual(len(report['transactions']), 18000)
        self.assertEqual(report['summary']['expenses'], 18000)
        self.assertEqual(report['extraction']['method'], 'deterministic')

    def test_more_than_twelve_ai_chunks_and_2500_lines_are_resumable(self):
        text = '\n'.join('01/04/2026 Fictional merchant ' + 'x' * 70 + ' 1.00' for _ in range(2601))
        calls = []
        def extract(model, numbered):
            import re
            lines = [int(n) for n in re.findall(r'^L(\d+) ', numbered.split('Extract only this chunk:\n')[1], re.M)]
            calls.append(lines)
            return {'transactions': [{'date': '2026-04-01', 'description': 'Fictional merchant',
                    'direction': 'out', 'amount': 1, 'currency': 'INR', 'sourceLines': [line]} for line in lines]}
        model = {'id': 'fake', 'contextLength': 8192}
        with tempfile.TemporaryDirectory() as directory, patch.dict('os.environ', {'WEALTH_STATEMENT_LIBRARY': directory}), patch('local_ai.extract_chunk', side_effect=extract):
            report = _extract_bank(text, 'large.md', model)
            self.assertEqual(len(report['transactions']), 2601)
            self.assertGreater(len(calls), 12)
            count = len(calls)
            again = _extract_bank(text, 'large.md', model)
            self.assertEqual(len(calls), count)
            self.assertEqual(again['summary']['expenses'], 2601)

    def test_truncated_chunk_is_split_and_invalid_json_not_accepted(self):
        lines = ['L1 01/04/2026 Shop 1', 'L2 02/04/2026 Shop 2']
        calls = []
        def extract(model, text):
            calls.append(text)
            if 'L1' in text and 'L2' in text:
                raise LocalAIError('Local model output was truncated')
            return {'transactions': []}
        with tempfile.TemporaryDirectory() as folder:
            result = list(chunk_requests({}, [lines], [], Path(folder), extract, LocalAIError))
            self.assertEqual(len(result), 2)
            self.assertEqual(len(calls), 3)

    def test_completed_chunks_survive_failure_and_resume(self):
        chunks = [['L1 01/04/2026 Shop 1'], ['L2 02/04/2026 Shop 2']]
        calls = []
        def extract(model, text):
            calls.append(text)
            if 'L2' in text:
                raise LocalAIError('server unavailable')
            return {'transactions': []}
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(LocalAIError):
                list(chunk_requests({}, chunks, [], Path(folder), extract, LocalAIError))
            calls.clear()
            def retry(model, text):
                calls.append(text)
                return {'transactions': []}
            result = list(chunk_requests({}, chunks, [], Path(folder), retry, LocalAIError))
            self.assertEqual(len(result), 2)
            self.assertEqual(len(calls), 1)
            self.assertIn('L2', calls[0])

    def test_owned_model_released_even_when_extraction_fails(self):
        model = {'id': 'wealth-statements', 'contextLength': 8192}
        with tempfile.TemporaryDirectory() as directory, patch.dict('os.environ', {'WEALTH_STATEMENT_LIBRARY': directory}), patch('local_ai.discover_model', return_value=model), patch('local_ai._extract_bank', side_effect=LocalAIError('invalid')), patch('local_ai.release_model') as release:
            with self.assertRaises(LocalAIError):
                extract_bank('source', 'bank.md')
            release.assert_called_once()

    def test_load_has_one_worker_small_context_and_idle_ttl(self):
        with patch('model_runtime.Path.is_file', return_value=True), patch('model_runtime.subprocess.run') as run:
            result = load_for_statements([{'key': 'bonsai', 'type': 'llm', 'size_bytes': 4}],
                lambda path: {'models': [{'loaded_instances': [{'id': 'wealth-statements', 'config': {'context_length': 8192}}]}]})
            args = run.call_args.args[0]
            self.assertEqual(args[args.index('--parallel') + 1], '1')
            self.assertEqual(args[args.index('--ttl') + 1], '120')
            self.assertEqual(result['id'], 'wealth-statements')
        with patch('local_ai.request_json') as request:
            release_model({'id': 'another-app'}, request)
            request.assert_not_called()

    def test_generation_budget_increased_but_bounded(self):
        with patch('local_ai.request_json', return_value={'choices': [{'finish_reason': 'stop', 'message': {'content': '{"transactions":[]}'}}]}) as request:
            extract_chunk({'id': 'fake', 'contextLength': 8192}, 'source')
            self.assertEqual(request.call_args.args[1]['max_tokens'], 6144)

if __name__ == '__main__':
    unittest.main()
