import { describe, expect, it } from "vitest";
import { bankFlow, statementSchema, type StatementTransaction } from "./statement-data";

const transaction = (changes: Partial<StatementTransaction>): StatementTransaction => ({
  date: "2026-04-01",
  description: "Test",
  direction: "in",
  amount: 100,
  currency: "INR",
  category: "Salary",
  flowType: "income",
  unit: "cash",
  sourceLines: [1],
  ...changes,
});

function expectConserved(flow: ReturnType<typeof bankFlow>) {
  const inbound = flow.links
    .filter((l) => l.target === 0)
    .reduce((v, l) => v + Math.round(l.value * 100), 0);
  const outbound = flow.links
    .filter((l) => l.source === 0)
    .reduce((v, l) => v + Math.round(l.value * 100), 0);
  expect(inbound).toBe(outbound);
  expect(flow.links.every((link) => link.value > 0 && link.source !== link.target)).toBe(true);
}

describe("bank Sankey accounting", () => {
  it("balances sources and uses while separating transfers from income/expenses", () => {
    const flow = bankFlow(
      [
        transaction({ amount: 80000 }),
        transaction({ direction: "out", amount: 20000, category: "Rent", flowType: "expense" }),
        transaction({
          direction: "out",
          amount: 10000,
          category: "Own account",
          flowType: "transfer",
        }),
      ],
      "INR",
    );
    expect(flow.income).toBe(80000);
    expect(flow.expenses).toBe(20000);
    expect(flow.net).toBe(50000);
    expect(flow.nodes.some((n) => n.name === "Net cash retained")).toBe(true);
    expectConserved(flow);
  });
  it("shows balance funding for an expense-only period without inventing income", () => {
    const flow = bankFlow(
      [transaction({ direction: "out", amount: 2500, flowType: "expense" })],
      "INR",
    );
    expect(flow.income).toBe(0);
    expect(flow.nodes.some((n) => n.name === "Balance / other funding required")).toBe(true);
    expectConserved(flow);
  });
  it("does not combine currencies, zero amounts, or security units", () => {
    const flow = bankFlow(
      [
        transaction({ amount: 100 }),
        transaction({ amount: 999, currency: "USD" }),
        transaction({ amount: null, unit: "units", flowType: "security", currency: null }),
        transaction({ amount: 0 }),
      ],
      "INR",
    );
    expect(flow.income).toBe(100);
    expect(flow.count).toBe(1);
    expectConserved(flow);
  });
  it("uses cents so floating point fractions cannot unbalance the graph", () => {
    const flow = bankFlow(
      [
        transaction({ amount: 0.1 }),
        transaction({ amount: 0.2 }),
        transaction({ direction: "out", amount: 0.3, flowType: "expense" }),
      ],
      "INR",
    );
    expect(flow.net).toBe(0);
    expectConserved(flow);
  });
  it("returns an empty graph for empty filters", () => {
    expect(bankFlow([], "INR").links).toEqual([]);
  });
});

describe("downloaded statement validation", () => {
  it("rejects unsupported versions and non-finite financial data", () => {
    expect(statementSchema.safeParse({ schemaVersion: 9 }).success).toBe(false);
    expect(
      statementSchema.safeParse({ schemaVersion: 1, summary: { income: Infinity } }).success,
    ).toBe(false);
  });
});
