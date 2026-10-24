import { describe, expect, it } from "vitest";
import { detailedCashFlow, spendingCategory } from "./bank-sankey-data";
import type { StatementTransaction } from "./statement-data";

const transaction = (
  id: string,
  amount: number,
  description: string,
  changes: Partial<StatementTransaction> = {},
): StatementTransaction => ({
  id,
  amount,
  description,
  date: "2026-04-01",
  direction: "out",
  currency: "INR",
  category: "Other expenses",
  flowType: "expense",
  unit: "cash",
  sourceLines: [1],
  ...changes,
});
const rows = [
  transaction("salary", 1000, "Payroll salary", {
    direction: "in",
    category: "Salary",
    flowType: "income",
  }),
  transaction("groceries", 200, "UPI BIGBASKET"),
  transaction("dining", 100, "UPI ZOMATO"),
  transaction("fees", 20, "Service charge"),
  transaction("transfer", 50, "Own account transfer", {
    flowType: "transfer",
    category: "Internal transfers",
  }),
];
describe("hierarchical bank cash flow", () => {
  it("conserves cash and each category split exactly", () => {
    const graph = detailedCashFlow(rows, "INR", true, "Groceries");
    expect(graph.spending).toBe(320);
    expect(graph.retained).toBe(630);
    expect(graph.transferTotal).toBe(50);
    for (let index = 0; index < graph.nodes.length; index++) {
      const node = graph.nodes[index];
      const incoming = graph.links.filter((link) => link.target === index);
      const outgoing = graph.links.filter((link) => link.source === index);
      for (const links of [incoming, outgoing]) {
        if (links.length)
          expect(Math.round(links.reduce((sum, link) => sum + link.value, 0) * 100)).toBe(
            Math.round(node.value * 100),
          );
      }
    }
  });
  it("expands Expenses into categories and a category into descriptions", () => {
    expect(detailedCashFlow(rows, "INR", false).nodes.some((node) => node.level === 3)).toBe(false);
    const graph = detailedCashFlow(rows, "INR", true, "Groceries");
    expect(graph.nodes.some((node) => node.key === "expense:Groceries")).toBe(true);
    expect(graph.nodes.some((node) => node.level === 4 && node.category === "Groceries")).toBe(
      true,
    );
  });
  it("keeps funding separate from income when debits exceed credits", () => {
    const graph = detailedCashFlow(
      rows.filter((row) => row.direction === "out"),
      "INR",
      true,
    );
    expect(graph.nodes.find((node) => node.key === "funding")?.value).toBe(370);
    expect(graph.credits).toBe(0);
    expect(graph.retained).toBe(0);
  });
  it("does not mix currencies or non-cash unit movements", () => {
    const extras = [
      transaction("usd", 999, "foreign", { currency: "USD" }),
      transaction("units", 999, "mutual fund units", { unit: "units" }),
    ];
    expect(detailedCashFlow([...rows, ...extras], "INR", true).spending).toBe(320);
  });
  it("preserves reviewed categories and does not infer a transfer merely from UPI", () => {
    expect(
      spendingCategory(
        transaction("custom", 20, "UPI ZOMATO", { category: "My reviewed category" }),
      ),
    ).toBe("My reviewed category");
    expect(spendingCategory(transaction("unknown", 20, "UPI 123456 SOME PERSON"))).toBe(
      "Uncategorized expenses",
    );
    expect(
      detailedCashFlow([transaction("unknown", 20, "UPI 123456 SOME PERSON")], "INR", true)
        .transferTotal,
    ).toBe(0);
  });
  it("retains the full total when grouping many description branches", () => {
    const many = Array.from({ length: 13 }, (_, index) =>
      transaction("row" + index, 10.01, "merchant-" + String.fromCharCode(65 + index), {
        category: "Reviewed category",
      }),
    );
    const graph = detailedCashFlow(many, "INR", true, "Reviewed category");
    const descriptions = graph.nodes.filter((node) => node.level === 4);
    expect(descriptions).toHaveLength(9);
    expect(Math.round(descriptions.reduce((sum, node) => sum + node.value, 0) * 100)).toBe(13013);
  });
});
