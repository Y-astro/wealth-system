import type { StatementReport, StatementTransaction } from "./statement-data";

export type CombinedBankTransaction = StatementTransaction & {
  statementFilename: string;
  statementId: string;
};
type Options = {
  currency: string;
  from?: string;
  to?: string;
  excludedIds?: string[];
  accountGroups?: Record<string, string>;
};
export function bankCurrencies(reports: StatementReport[]) {
  return [
    ...new Set(
      reports
        .filter((report) => report.kind === "bank")
        .flatMap((report) =>
          report.transactions
            .filter(
              (row) =>
                row.unit === "cash" &&
                row.flowType !== "security" &&
                row.amount != null &&
                row.amount > 0 &&
                row.currency,
            )
            .map((row) => row.currency!),
        ),
    ),
  ].sort();
}

/** Bank cash only. Multiset union preserves legitimate repeated payments within each file. */
export function combineBankStatements(reports: StatementReport[], options: Options) {
  const transactions: CombinedBankTransaction[] = [];
  const reportIds = new Set<string>();
  const multiplicities = new Map<string, number>();
  let duplicates = 0,
    duplicateFiles = 0,
    statements = 0;
  for (const report of reports) {
    if (report.kind !== "bank" || options.excludedIds?.includes(report.id)) continue;
    if (reportIds.has(report.id)) {
      duplicateFiles++;
      continue;
    }
    reportIds.add(report.id);
    statements++;
    // Grouping is explicit: similar payments in different accounts must not disappear.
    const account = options.accountGroups?.[report.id]?.trim().toLowerCase();
    const group = account ? "account:" + account : "file:" + report.id;
    const occurrences = new Map<string, number>();
    report.transactions.forEach((row, index) => {
      if (
        row.unit !== "cash" ||
        row.flowType === "security" ||
        row.currency !== options.currency ||
        row.amount == null ||
        !Number.isFinite(row.amount) ||
        row.amount <= 0
      )
        return;
      if ((options.from && row.date < options.from) || (options.to && row.date > options.to))
        return;
      const signature = JSON.stringify([
        group,
        row.currency,
        row.date,
        row.direction,
        Math.round(row.amount * 100),
        row.description.trim().replace(/\s+/g, " ").toLowerCase(),
        row.balance == null ? null : Math.round(row.balance * 100),
      ]);
      const ordinal = (occurrences.get(signature) ?? 0) + 1;
      occurrences.set(signature, ordinal);
      const previous = multiplicities.get(signature) ?? 0;
      if (ordinal <= previous) {
        duplicates++;
        return;
      }
      multiplicities.set(signature, ordinal);
      transactions.push({
        ...row,
        id: report.id + ":" + index,
        statementId: report.id,
        statementFilename: report.filename,
      });
    });
  }
  return {
    transactions: transactions.sort((a, b) => a.date.localeCompare(b.date)),
    duplicates,
    duplicateFiles,
    statements,
  };
}
