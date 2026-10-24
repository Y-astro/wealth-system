import type { StatementReport } from "./statement-data";

export interface BalancePoint {
  date: string;
  value: number;
}

/** Observed end-of-day balances. Never add overlapping statements or infer market returns. */
export function statementBalanceHistory(report: StatementReport): BalancePoint[] {
  const points = new Map<string, number>();
  if (report.kind === "investment") {
    for (const row of report.history) points.set(row.date, row.value);
    if (!points.size && report.period.to && report.summary.reportedValue != null)
      points.set(report.period.to, report.summary.reportedValue);
  } else {
    const cash = report.transactions.filter(
      (row) => row.unit === "cash" && row.currency === report.currency,
    );
    // Some exports list newest transactions first, including transactions on the same day.
    const dates = cash.map((row) => row.date);
    const reversed = [...cash].reverse();
    const descending =
      dates.some((date, index) => index > 0 && date < dates[index - 1]) &&
      dates.every((date, index) => index === 0 || date <= dates[index - 1]);
    const reverseReconciles = reversed.every(
      (row, index) =>
        index === 0 ||
        (row.balance != null &&
          reversed[index - 1].balance != null &&
          row.amount != null &&
          Math.abs(
            reversed[index - 1].balance! +
              (row.direction === (report.statementType === "credit-card" ? "out" : "in")
                ? row.amount
                : -row.amount) -
              row.balance,
          ) < 0.05),
    );
    const ordered = descending && reverseReconciles ? reversed : cash;
    for (const row of [...ordered].sort((a, b) => a.date.localeCompare(b.date)))
      if (row.balance != null) points.set(row.date, row.balance);
    if (report.period.to && report.summary.closingBalance != null)
      points.set(report.period.to, report.summary.closingBalance);
  }
  return [...points]
    .filter(([, value]) => Number.isFinite(value))
    .map(([date, value]) => ({ date, value }))
    .sort((a, b) => a.date.localeCompare(b.date));
}

export function filterBalanceHistory(points: BalancePoint[], days: number | null) {
  if (!days || !points.length) return points;
  const cutoff = Date.parse(points[points.length - 1].date + "T00:00:00Z") - days * 86_400_000;
  return points.filter((point) => Date.parse(point.date + "T00:00:00Z") >= cutoff);
}
