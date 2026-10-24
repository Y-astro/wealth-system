"""Credit-card accounting with fictional statement rows only."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from statement_parser import analyze, finalize_bank
from credit_cards import is_credit_card_filename

HEADER = 'Date,Description,Debit,Credit,Balance\n'
ROWS = ['01/04/2026,Fuel purchase,300,0,', '02/04/2026,Groceries,700,0,',
        '03/04/2026,Fuel Trxn Off us,0,30,', '04/04/2026,BBPS Payment received,0,970,']

class CreditCardTests(unittest.TestCase):
    def test_filename_detection_does_not_match_account(self):
        for name in ['ICICI CC Statement.csv', 'HDFC_CC_June.pdf', 'bank-cc.xls', 'ICICICCStatement.csv']:
            self.assertTrue(is_credit_card_filename(name), name)
        for name in ['Account Statement.csv', 'Axis AcctStatement.xls', 'bank.csv', 'ACCOUNT.pdf']:
            self.assertFalse(is_credit_card_filename(name), name)

    def test_fuel_credit_offsets_expense_and_payments_are_transfers(self):
        report = analyze(HEADER + '\n'.join(ROWS), 'bank CC.csv')
        self.assertEqual(report['statementType'], 'credit-card')
        fuel, payment = report['transactions'][2:]
        self.assertEqual((fuel['flowType'], fuel['category'], fuel['expenseOffset']), ('expense', 'Transport', True))
        self.assertEqual((payment['flowType'], payment['cardCreditKind']), ('transfer', 'payment'))
        self.assertEqual(report['summary']['income'], 0)
        self.assertEqual(report['summary']['grossCharges'], 1000)
        self.assertEqual(report['summary']['expenseRefunds'], 30)
        self.assertEqual(report['summary']['expenses'], 970)
        self.assertEqual(report['summary']['cardPayments'], 970)
        self.assertEqual(report['summary']['paymentDifference'], 0)
        self.assertNotIn('closingBalance', report['summary'])

    def test_short_and_excess_payments_are_not_forced_to_match(self):
        for payment, difference in [(900, 70), (1200, -230)]:
            text = HEADER + '\n'.join(ROWS[:-1] + [f'04/04/2026,BBPS Payment received,0,{payment},'])
            self.assertEqual(analyze(text, 'cc.csv')['summary']['paymentDifference'], difference)

    def test_unknown_credit_requires_review_and_never_becomes_income(self):
        text = HEADER + '\n'.join(ROWS[:-1] + ['04/04/2026,Unidentified credit,0,970,'])
        report = analyze(text, 'cc.csv')
        self.assertEqual(report['summary']['income'], 0)
        self.assertEqual(report['summary']['unclassifiedCredits'], 970)
        self.assertEqual(report['summary']['cardPayments'], 0)
        self.assertTrue(any('Unclassified card credits' in w for w in report['warnings']))

    def test_refund_category_and_negative_net_expense_are_preserved(self):
        report = analyze(HEADER + '01/04/2026,GST fee reversal,0,100,', 'cc.csv')
        self.assertEqual(report['transactions'][0]['category'], 'Fees & taxes')
        self.assertEqual(report['summary']['expenses'], -100)
        self.assertEqual(report['summary']['income'], 0)

    def test_liability_balance_equation_in_both_source_orders(self):
        rows = ['01/04/2026,Fuel purchase,1000,0,1500', '02/04/2026,Fuel refund,0,100,1400',
                '03/04/2026,BBPS Payment received,0,700,700']
        for sequence in [rows, rows[::-1]]:
            summary = analyze(HEADER + '\n'.join(sequence), 'cc.csv')['summary']
            self.assertEqual(summary['openingBalance'], 500)
            self.assertEqual(summary['closingBalance'], 700)
            self.assertEqual(summary['balanceDifference'], 0)
            self.assertEqual(summary['paymentDifference'], 200)

    def test_repeated_finalization_preserves_source_and_classification(self):
        report = analyze(HEADER + '\n'.join(ROWS), 'cc.csv')
        mappings = len(report['mappings'])
        finalize_bank(report)
        self.assertEqual(len(report['mappings']), mappings)
        self.assertEqual(report['summary']['expenses'], 970)
        self.assertEqual(report['transactions'][2]['sourceLines'], [4])

    def test_ordinary_bank_credit_handling_is_unchanged(self):
        report = analyze(HEADER + '01/04/2026,Fuel Trxn Off us,0,30,1000', 'bank.csv')
        self.assertNotIn('statementType', report)
        self.assertEqual(report['transactions'][0]['flowType'], 'income')

    def test_merchant_verification_credits_match_only_one_earlier_debit(self):
        rows = ['01/04/2026,VIDEO MERCHANT,2,0,', '02/04/2026,VIDEO MERCHANT,0,2,', '03/04/2026,VIDEO MERCHANT,0,2,']
        report = analyze(HEADER + '\n'.join(rows), 'CC.csv')
        self.assertEqual(report['transactions'][1]['cardCreditKind'], 'refund')
        self.assertEqual(report['transactions'][2]['cardCreditKind'], 'unclassified')
        source = next(m['sourceLines'] for m in report['mappings'] if m['target'] == 'transactions[1].cardCreditKind')
        self.assertEqual(set(source), {2, 3})

    def test_tax_reversal_abbreviation_offsets_tax_expense(self):
        report = analyze(HEADER + '01/04/2026,IGST charge,1.8,0,\n02/04/2026,IGST-Rev-CI@18%,0,1.8,', 'CC.csv')
        self.assertEqual(report['transactions'][1]['category'], 'Fees & taxes')
        self.assertTrue(report['transactions'][1]['expenseOffset'])
        self.assertEqual(report['summary']['expenses'], 0)

if __name__ == '__main__': unittest.main()
