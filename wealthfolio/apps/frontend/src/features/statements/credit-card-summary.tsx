import { creditCardTotals } from "./credit-card-data";
import { formatMoney, type StatementTransaction } from "./statement-data";

export function CreditCardSummary({
  transactions,
  currency,
}: {
  transactions: StatementTransaction[];
  currency: string;
}) {
  const totals = creditCardTotals(transactions, currency);
  return (
    <section
      aria-label="Credit card payment coverage"
      className="bg-card space-y-4 rounded-xl border p-5"
    >
      <div>
        <h2 className="font-semibold">Credit card · payments vs net expenses</h2>
        <p className="text-muted-foreground mt-1 text-sm">
          Fuel credits, refunds and waivers reduce expenses. Card payments are transfers, not
          income.
        </p>
      </div>
      <dl className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {[
          ["Charges", totals.gross],
          ["Expense refunds / fuel credits", totals.refunds],
          ["Net expenses", totals.expenses],
          ["Card payments", totals.payments],
        ].map(([label, value]) => (
          <div key={String(label)}>
            <dt className="text-muted-foreground text-sm">{label}</dt>
            <dd className="mt-1 text-xl tabular-nums">{formatMoney(Number(value), currency)}</dd>
          </div>
        ))}
      </dl>
      <p role="status" className="rounded-lg border p-3 font-medium">
        {totals.otherCredits > 0
          ? `Needs review · ${formatMoney(totals.otherCredits, currency)} of unclassified card credits`
          : totals.expenses < 0
            ? `Refunds exceed charges by ${formatMoney(-totals.expenses, currency)} · net expense credit for this period.`
            : totals.matches
              ? "Payments match net expenses (within 0.01)."
              : totals.difference > 0
                ? `Payments are below net expenses by ${formatMoney(totals.difference, currency)}.`
                : `Payments exceed net expenses by ${formatMoney(-totals.difference, currency)}.`}
      </p>
      <p className="text-muted-foreground text-xs">
        For the selected currency and dates: charges − expense refunds = net expenses; net expenses
        − card payments = {formatMoney(totals.difference, currency)}. Unclassified credits are
        excluded until reviewed. Payments may settle an earlier bill; this comparison is not the
        amount due.
      </p>
    </section>
  );
}
