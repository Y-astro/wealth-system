import { describe, expect, it } from "vitest";
import type { StatementReport, StatementTransaction } from "./statement-data";
import { bankCurrencies, combineBankStatements } from "./combined-bank-data";
const row = (changes: Partial<StatementTransaction> = {}) =>
  ({
    id: "1",
    date: "2026-04-01",
    description: "Supermarket",
    amount: 100,
    direction: "out",
    currency: "INR",
    category: "Groceries",
    flowType: "expense",
    unit: "cash",
    balance: 900,
    sourceLines: [1],
    ...changes,
  }) as StatementTransaction;
const report = (
  id: string,
  transactions: StatementTransaction[],
  kind: "bank" | "investment" = "bank",
) =>
  ({
    id,
    filename: id + ".pdf",
    kind,
    transactions,
    currency: "INR",
    summary: { closingBalance: 999999 },
    holdings: [{ value: 999999 }],
    accounts: [],
    period: { from: "2026-04-01", to: "2026-04-30" },
    warnings: [],
  }) as unknown as StatementReport;
describe("combined bank-only Sankey data", () => {
  it("excludes every investment report including its cash transactions and stock values", () => {
    const reports = [
      report("bank", [row()]),
      report("nsdl", [row({ amount: 999999, currency: "USD" })], "investment"),
    ];
    const combined = combineBankStatements(reports, { currency: "INR" });
    expect(combined.transactions).toHaveLength(1);
    expect(combined.statements).toBe(1);
    expect(bankCurrencies(reports)).toEqual(["INR"]);
  });
  it("combines different banks and keeps their original filenames and unique identities", () => {
    const result = combineBankStatements([report("a", [row()]), report("b", [row()])], {
      currency: "INR",
    });
    expect(result.transactions).toHaveLength(2);
    expect(new Set(result.transactions.map((item) => item.id)).size).toBe(2);
    expect(result.transactions.map((item) => item.statementFilename)).toEqual(["a.pdf", "b.pdf"]);
  });
  it("never adds different currencies together", () => {
    const reports = [report("a", [row(), row({ currency: "USD", amount: 50 })])];
    expect(bankCurrencies(reports)).toEqual(["INR", "USD"]);
    expect(
      combineBankStatements(reports, { currency: "USD" }).transactions.map((item) => item.amount),
    ).toEqual([50]);
  });
  it("counts matching overlaps once only when files share an explicit account group", () => {
    const reports = [report("a", [row()]), report("b", [row()])];
    expect(
      combineBankStatements(reports, {
        currency: "INR",
        accountGroups: { a: "Savings", b: " savings " },
      }).duplicates,
    ).toBe(1);
    expect(
      combineBankStatements(reports, {
        currency: "INR",
        accountGroups: { a: "Account A", b: "Account B" },
      }).transactions,
    ).toHaveLength(2);
  });
  it("preserves the maximum multiplicity of legitimate repeated payments", () => {
    const reports = [report("a", [row(), row()]), report("b", [row(), row(), row()])];
    const result = combineBankStatements(reports, {
      currency: "INR",
      accountGroups: { a: "same", b: "same" },
    });
    expect(result.transactions).toHaveLength(3);
    expect(result.duplicates).toBe(2);
  });
  it("keeps same-amount payments with different running balances distinct", () => {
    expect(
      combineBankStatements([report("a", [row()]), report("b", [row({ balance: 800 })])], {
        currency: "INR",
        accountGroups: { a: "same", b: "same" },
      }).transactions,
    ).toHaveLength(2);
  });
  it("honors date and file filters", () => {
    const reports = [report("a", [row(), row({ date: "2026-05-01" })]), report("b", [row()])];
    const result = combineBankStatements(reports, {
      currency: "INR",
      from: "2026-04-01",
      to: "2026-04-30",
      excludedIds: ["b"],
    });
    expect(result.transactions).toHaveLength(1);
    expect(result.statements).toBe(1);
  });
  it("ignores duplicate report identities without changing original rows", () => {
    const source = row();
    const input = report("a", [source]);
    const result = combineBankStatements([input, input], { currency: "INR" });
    expect(result.transactions).toHaveLength(1);
    expect(result.duplicateFiles).toBe(1);
    expect(source.id).toBe("1");
    expect(source).not.toHaveProperty("statementFilename");
  });
  it("excludes units, missing amounts and invalid cash rows", () => {
    const rows = [
      row({ unit: "units", flowType: "security" }),
      row({ amount: null }),
      row({ amount: 0 }),
      row({ amount: -1 }),
      row({ amount: Infinity }),
    ];
    expect(combineBankStatements([report("a", rows)], { currency: "INR" }).transactions).toEqual(
      [],
    );
  });
});
