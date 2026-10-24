import { z } from "zod";
import { expenseAdjustedTransactions } from "./credit-card-data";

const finite = z.number().finite();
const lines = z.array(z.number().int().positive()).min(1);
const currency = z.string().regex(/^[A-Z]{3}$/);
const date = z.string().regex(/^\d{4}-\d{2}-\d{2}$/);
const holdingSchema = z.object({
  id: z.string(),
  isin: z.string().nullable(),
  symbol: z.string().nullable(),
  name: z.string(),
  assetClass: z.string(),
  quantity: finite.nonnegative(),
  price: finite.nonnegative(),
  value: finite.nonnegative(),
  cost: finite.nullable(),
  unrealizedGain: finite.nullable(),
  accountId: z.string(),
  accountName: z.string(),
  currency,
  sourceLines: lines,
  pledgedQuantity: finite.nonnegative().optional(),
  faceValue: finite.optional(),
  folio: z.string().optional(),
  averageCost: finite.optional(),
});
const transactionSchema = z.object({
  id: z.string().optional(),
  date,
  description: z.string(),
  direction: z.enum(["in", "out"]),
  amount: finite.nonnegative().nullable(),
  quantity: finite.optional(),
  currency: currency.nullable(),
  category: z.string(),
  flowType: z.enum(["income", "expense", "transfer", "security"]),
  unit: z.enum(["cash", "units"]),
  sourceLines: lines,
  balance: finite.nullable().optional(),
  expenseOffset: z.boolean().optional(),
  cardCreditKind: z.enum(["payment", "refund", "unclassified"]).optional(),
  isin: z.string().optional(),
  openingQuantity: finite.nullable().optional(),
  closingQuantity: finite.optional(),
});
export const statementSchema = z
  .object({
    schemaVersion: z.literal(1),
    id: z.string(),
    kind: z.enum(["investment", "bank"]),
    statementType: z.literal("credit-card").optional(),
    filename: z.string(),
    currency,
    period: z.object({ from: date.nullable(), to: date.nullable() }),
    summary: z.record(finite.nullable()),
    accounts: z.array(
      z.object({
        id: z.string(),
        name: z.string(),
        type: z.string(),
        value: finite,
        reportedValue: finite.nullable(),
        sourceLines: lines,
        summaryAccountId: z.string().optional(),
      }),
    ),
    allocations: z.array(
      z.object({
        name: z.string(),
        code: z.string(),
        value: finite,
        percent: finite,
        sourceLines: lines,
      }),
    ),
    history: z.array(
      z.object({
        date,
        value: finite,
        change: finite.nullable(),
        changePercent: finite.nullable(),
        sourceLines: lines,
      }),
    ),
    holdings: z.array(holdingSchema),
    transactions: z.array(transactionSchema),
    mappings: z.array(z.object({ target: z.string(), sourceLines: lines, rule: z.string() })),
    warnings: z.array(z.string()),
    source: z.array(
      z.object({ line: z.number().int().positive(), text: z.string(), normalized: z.string() }),
    ),
    extraction: z.object({
      method: z.enum(["deterministic", "local-ai"]),
      model: z.string().nullable(),
    }),
  })
  .superRefine((report, ctx) => {
    const source = new Set(report.source.map((row) => row.line));
    const references = [
      ...report.mappings,
      ...report.holdings,
      ...report.transactions,
      ...report.accounts,
      ...report.history,
      ...report.allocations,
    ];
    if (
      source.size !== report.source.length ||
      references.some((row) => row.sourceLines.some((line) => !source.has(line)))
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Mapping references a missing source line.",
      });
    }
    if (
      report.transactions.some(
        (t) =>
          (t.unit === "cash" &&
            (t.amount == null || t.currency == null || t.flowType === "security")) ||
          (t.flowType === "income" && t.direction !== "in") ||
          (t.flowType === "expense" && t.direction !== "out" && !t.expenseOffset) ||
          (t.expenseOffset &&
            (t.flowType !== "expense" ||
              t.direction !== "in" ||
              report.statementType !== "credit-card")) ||
          (t.cardCreditKind === "refund" && !t.expenseOffset) ||
          (t.cardCreditKind &&
            (report.statementType !== "credit-card" ||
              (t.cardCreditKind !== "refund" && t.flowType !== "transfer"))) ||
          (report.statementType === "credit-card" && t.flowType === "income") ||
          (t.unit === "units" &&
            (t.amount != null || t.currency != null || t.flowType !== "security")),
      )
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Transaction cash, units and direction mappings are inconsistent.",
      });
    }
  });
export type StatementReport = z.infer<typeof statementSchema>;
export type StatementHolding = z.infer<typeof holdingSchema>;
export type StatementTransaction = z.infer<typeof transactionSchema>;

export function formatMoney(
  value: number | null | undefined,
  code = "INR",
  compact = false,
): string {
  if (value == null) return "—";
  return new Intl.NumberFormat(code === "INR" ? "en-IN" : "en-US", {
    style: "currency",
    currency: code,
    notation: compact ? "compact" : "standard",
    maximumFractionDigits: compact ? 1 : 2,
  }).format(value);
}

export function groupValues<T>(rows: T[], name: (row: T) => string, value: (row: T) => number) {
  const totals = new Map<string, number>();
  for (const row of rows)
    totals.set(name(row), (totals.get(name(row)) ?? 0) + Math.round(value(row) * 100));
  return [...totals]
    .map(([name, cents]) => ({ name, value: cents / 100 }))
    .sort((a, b) => b.value - a.value);
}

export function bankFlow(transactions: StatementTransaction[], code: string) {
  const cash = transactions.filter(
    (t) => t.unit === "cash" && t.currency === code && t.amount != null && t.amount > 0,
  );
  const incoming = cash.filter((t) => t.direction === "in");
  const outgoing = cash.filter((t) => t.direction === "out");
  const sum = (rows: StatementTransaction[]) =>
    rows.reduce((total, t) => total + Math.round(t.amount! * 100), 0) / 100;
  const credits = sum(incoming),
    debits = sum(outgoing);
  const income = sum(cash.filter((t) => t.flowType === "income"));
  const expenses =
    Math.round(
      (sum(cash.filter((t) => t.flowType === "expense" && t.direction === "out")) -
        sum(cash.filter((t) => t.expenseOffset && t.direction === "in"))) *
        100,
    ) / 100;
  const nodes: { name: string; side: "in" | "center" | "out" }[] = [
    { name: "Cash available", side: "center" },
  ];
  const links: { source: number; target: number; value: number }[] = [];
  const adjusted = expenseAdjustedTransactions(cash);
  for (const [rows, side] of [
    [adjusted.filter((t) => t.direction === "in"), "in"],
    [adjusted.filter((t) => t.direction === "out"), "out"],
  ] as const) {
    const groups = groupValues(
      rows,
      (t) =>
        `${t.expenseOffset ? "Expense credit" : t.flowType === "transfer" ? "Transfer" : side === "in" ? "Income" : "Expense"} · ${t.category}`,
      (t) => t.amount!,
    );
    for (const group of groups) {
      const index = nodes.push({ name: group.name, side }) - 1;
      links.push({
        source: side === "in" ? index : 0,
        target: side === "in" ? 0 : index,
        value: group.value,
      });
    }
  }
  const net = Math.round((credits - debits) * 100) / 100;
  if (net !== 0) {
    const side = net < 0 ? "in" : "out";
    const index =
      nodes.push({
        name: net < 0 ? "Balance / other funding required" : "Net cash retained",
        side,
      }) - 1;
    links.push({ source: net < 0 ? index : 0, target: net < 0 ? 0 : index, value: Math.abs(net) });
  }
  return { nodes, links, credits, debits, income, expenses, net, count: cash.length };
}
