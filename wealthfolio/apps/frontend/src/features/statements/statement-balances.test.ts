import { describe, expect, it } from "vitest";
import type { StatementReport } from "./statement-data";
import { statementBalanceHistory, filterBalanceHistory } from "./statement-balances";
const report = (fields: Partial<StatementReport>) =>
  ({
    kind: "bank",
    currency: "INR",
    transactions: [],
    history: [],
    summary: {},
    period: { from: null, to: null },
    ...fields,
  }) as StatementReport;
const transaction = (date: string, balance: number | null, amount = 10, direction = "in") => ({
  date,
  balance,
  amount,
  direction,
  currency: "INR",
  unit: "cash",
});
describe("statement dashboard balance history", () => {
  it("keeps the last observed balance per day without adding overlapping observations", () => {
    const input = report({
      transactions: [
        transaction("2026-01-01", 100),
        transaction("2026-01-01", 110),
        transaction("2026-01-02", 90, 20, "out"),
      ] as StatementReport["transactions"],
    });
    expect(statementBalanceHistory(input)).toEqual([
      { date: "2026-01-01", value: 110 },
      { date: "2026-01-02", value: 90 },
    ]);
  });
  it("handles reconciled reverse chronological exports including same-day transactions", () => {
    const input = report({
      transactions: [
        transaction("2026-01-02", 130),
        transaction("2026-01-01", 120),
        transaction("2026-01-01", 110),
      ] as StatementReport["transactions"],
    });
    expect(statementBalanceHistory(input)[0].value).toBe(120);
  });
  it("does not turn cash flow into a balance when no balance is reported", () => {
    expect(
      statementBalanceHistory(
        report({
          transactions: [transaction("2026-01-01", null)] as StatementReport["transactions"],
        }),
      ),
    ).toEqual([]);
  });
  it("excludes foreign currency and security unit rows", () => {
    const rows = [
      { ...transaction("2026-01-01", 100), currency: "USD" },
      { ...transaction("2026-01-02", 100), unit: "units" },
    ];
    expect(
      statementBalanceHistory(report({ transactions: rows as StatementReport["transactions"] })),
    ).toEqual([]);
  });
  it("uses reported investment history and keeps negative balances valid", () => {
    const history = [
      { date: "2026-01-01", value: 42 },
      { date: "2026-02-01", value: 55 },
    ] as StatementReport["history"];
    expect(statementBalanceHistory(report({ kind: "investment", history }))).toEqual(history);
    expect(
      statementBalanceHistory(
        report({
          transactions: [transaction("2026-01-01", -5)] as StatementReport["transactions"],
        }),
      )[0].value,
    ).toBe(-5);
  });
  it("anchors ranges to the last observation, even for older statements", () => {
    const points = [
      { date: "2020-01-01", value: 1 },
      { date: "2020-03-01", value: 2 },
    ];
    expect(filterBalanceHistory(points, 31)).toEqual([points[1]]);
  });
});
