import { describe, expect, it } from "vitest";
import {
  bankFlow,
  statementSchema,
  type StatementReport,
  type StatementTransaction,
} from "./statement-data";
import { creditCardTotals, expenseAdjustedTransactions, isCreditCard } from "./credit-card-data";
import { detailedCashFlow } from "./bank-sankey-data";

const txn = (changes: Partial<StatementTransaction> = {}): StatementTransaction => ({
  date: "2026-04-01",
  description: "Fuel",
  direction: "out",
  amount: 500,
  currency: "INR",
  category: "Transport",
  flowType: "expense",
  unit: "cash",
  sourceLines: [1],
  ...changes,
});
const refund = txn({ direction: "in", amount: 50, expenseOffset: true, cardCreditKind: "refund" });
const payment = txn({
  direction: "in",
  amount: 450,
  flowType: "transfer",
  category: "Card repayments",
  cardCreditKind: "payment",
});
const report = (transactions: StatementTransaction[]): StatementReport => ({
  schemaVersion: 1,
  id: "fictional",
  kind: "bank",
  statementType: "credit-card",
  filename: "bank CC.csv",
  currency: "INR",
  period: { from: null, to: null },
  summary: {},
  accounts: [],
  allocations: [],
  history: [],
  holdings: [],
  transactions,
  mappings: [],
  warnings: [],
  source: [{ line: 1, text: "fictional", normalized: "fictional" }],
  extraction: { method: "deterministic", model: null },
});
function conserve(graph: ReturnType<typeof detailedCashFlow>) {
  for (const [index, node] of graph.nodes.entries()) {
    const inward = graph.links
      .filter((l) => l.target === index)
      .reduce((sum, l) => sum + Math.round(l.value * 100), 0);
    const outward = graph.links
      .filter((l) => l.source === index)
      .reduce((sum, l) => sum + Math.round(l.value * 100), 0);
    if (inward) expect(inward).toBe(Math.round(node.value * 100));
    if (outward) expect(outward).toBe(Math.round(node.value * 100));
  }
}
describe("credit card accounting", () => {
  it("nets fuel credits, excludes payments from income, and checks coverage", () => {
    const rows = [txn(), refund, payment];
    expect(creditCardTotals(rows, "INR")).toMatchObject({
      gross: 500,
      refunds: 50,
      expenses: 450,
      payments: 450,
      difference: 0,
      matches: true,
    });
    expect(bankFlow(rows, "INR")).toMatchObject({ income: 0, expenses: 450, net: 0 });
    const graph = detailedCashFlow(rows, "INR", true);
    expect(graph.spending).toBe(450);
    expect(graph.nodes.some((n) => n.kind === "income")).toBe(false);
    expect(graph.nodes.find((n) => n.key === "expense:Transport")?.value).toBe(450);
    conserve(graph);
  });
  it("preserves original rows and their source amounts for review", () => {
    const rows = [txn(), refund, payment];
    const before = JSON.stringify(rows);
    expenseAdjustedTransactions(rows);
    expect(JSON.stringify(rows)).toBe(before);
  });
  it("never uses one card's refund to reduce another card's charges", () => {
    const rows = [
      { ...txn(), statementId: "one" },
      { ...refund, statementId: "two" },
    ];
    const adjusted = expenseAdjustedTransactions(rows);
    expect(adjusted.find((row) => row.direction === "out")?.amount).toBe(500);
    expect(adjusted.find((row) => row.direction === "in")?.amount).toBe(50);
    const graph = detailedCashFlow(rows, "INR", true);
    expect(graph.nodes.some((n) => n.name.startsWith("Expense credit"))).toBe(true);
    expect(graph.nodes.some((n) => n.kind === "income")).toBe(false);
    conserve(graph);
  });
  it("keeps prior-period excess refunds as expense credits, not income", () => {
    const rows = [txn({ amount: 20 }), refund];
    expect(creditCardTotals(rows, "INR").expenses).toBe(-30);
    const graph = detailedCashFlow(rows, "INR", true);
    expect(graph.nodes.some((n) => n.name === "Expense credit · Transport" && n.value === 30)).toBe(
      true,
    );
    expect(graph.nodes.some((n) => n.kind === "income")).toBe(false);
    conserve(graph);
  });
  it("flags unknown credits and does not mix currencies", () => {
    const rows = [
      txn(),
      payment,
      txn({ currency: "USD", amount: 10000 }),
      txn({ direction: "in", amount: 50, flowType: "transfer", cardCreditKind: "unclassified" }),
    ];
    expect(creditCardTotals(rows, "INR")).toMatchObject({
      difference: 50,
      otherCredits: 50,
      matches: false,
    });
  });
  it("validates explicit refund metadata without permitting ordinary bank expense credits", () => {
    expect(statementSchema.safeParse(report([txn(), refund, payment])).success).toBe(true);
    expect(
      statementSchema.safeParse({ ...report([refund]), statementType: undefined }).success,
    ).toBe(false);
    expect(statementSchema.safeParse(report([txn({ direction: "in" })])).success).toBe(false);
    expect(
      statementSchema.safeParse(report([txn({ direction: "in", flowType: "income" })])).success,
    ).toBe(false);
  });
  it("recognizes CC filenames without mistaking account statements", () => {
    expect(isCreditCard(report([]))).toBe(true);
    expect(
      isCreditCard({ ...report([]), statementType: undefined, filename: "Account Statement.csv" }),
    ).toBe(false);
  });
});
