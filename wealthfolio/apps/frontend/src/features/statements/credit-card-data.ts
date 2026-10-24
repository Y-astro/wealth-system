import type { StatementReport, StatementTransaction } from "./statement-data";

const cents = (value: number) => Math.round(value * 100);
export function isCreditCard(report: StatementReport) {
  return (
    report.kind === "bank" &&
    (report.statementType === "credit-card" ||
      /(?:^|[^a-z])cc(?:[^a-z]|$)|cc[ _-]*statement/i.test(report.filename))
  );
}

export function creditCardTotals(transactions: StatementTransaction[], currency: string) {
  const cash = transactions.filter(
    (row) => row.unit === "cash" && row.currency === currency && row.amount != null,
  );
  const sum = (rows: StatementTransaction[]) =>
    rows.reduce((total, row) => total + cents(row.amount!), 0);
  const gross = sum(cash.filter((row) => row.direction === "out" && row.flowType === "expense"));
  const refunds = sum(cash.filter((row) => row.direction === "in" && row.expenseOffset));
  const payments = cash
    .filter((row) => row.cardCreditKind === "payment")
    .reduce((total, row) => total + (row.direction === "in" ? 1 : -1) * cents(row.amount!), 0);
  const other = sum(cash.filter((row) => row.cardCreditKind === "unclassified"));
  return {
    gross: gross / 100,
    refunds: refunds / 100,
    expenses: (gross - refunds) / 100,
    payments: payments / 100,
    otherCredits: other / 100,
    difference: (gross - refunds - payments) / 100,
    matches: Math.abs(gross - refunds - payments) <= 1 && other === 0,
  };
}

/** Net refunds against their own statement/category for positive-width Sankey
 * branches. Preserve the original rows for review; never offset another card.
 * Prior-period/excess category credits remain explicitly labeled expense
 * credits, so they never become income or silently disappear. */
export function expenseAdjustedTransactions<T extends StatementTransaction>(rows: T[]): T[] {
  const key = (row: T) =>
    JSON.stringify([
      "statementId" in row ? row.statementId : "statement",
      row.currency,
      row.category.trim().toLowerCase(),
    ]);
  const refunds = new Map<string, number>();
  for (const row of rows) {
    if (row.direction === "in" && row.expenseOffset && row.amount != null)
      refunds.set(key(row), (refunds.get(key(row)) ?? 0) + cents(row.amount));
  }
  const adjusted: T[] = [];
  for (const row of rows) {
    if (row.direction === "in" && row.expenseOffset) continue;
    if (row.direction === "out" && row.flowType === "expense" && row.amount != null) {
      const refund = Math.min(cents(row.amount), refunds.get(key(row)) ?? 0);
      refunds.set(key(row), (refunds.get(key(row)) ?? 0) - refund);
      if (cents(row.amount) > refund)
        adjusted.push({ ...row, amount: (cents(row.amount) - refund) / 100 });
    } else adjusted.push(row);
  }
  for (const row of rows) {
    if (!row.expenseOffset || row.direction !== "in") continue;
    const remaining = refunds.get(key(row)) ?? 0;
    if (remaining > 0) {
      adjusted.push({ ...row, amount: remaining / 100 });
      refunds.set(key(row), 0);
    }
  }
  return adjusted;
}
