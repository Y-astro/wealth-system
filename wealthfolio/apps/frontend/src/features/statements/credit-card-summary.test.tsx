// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, describe, expect, it } from "vitest";
import { BankSankey } from "./bank-sankey";
import { CreditCardSummary } from "./credit-card-summary";
import type { StatementTransaction } from "./statement-data";

const row = (changes: Partial<StatementTransaction> = {}): StatementTransaction => ({
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
describe("card payment coverage", () => {
  it("uses card funding labels and omits the income legend", () => {
    const container = document.createElement("div");
    const chartRoot = createRoot(container);
    act(() =>
      chartRoot.render(
        <BankSankey
          creditCard
          currency="INR"
          transactions={[
            row(),
            row({
              direction: "in",
              amount: 500,
              flowType: "transfer",
              category: "Card repayments",
              cardCreditKind: "payment",
            }),
          ]}
        />,
      ),
    );
    expect(container.textContent).toContain("Card funding");
    expect(container.textContent).not.toContain("Income");
    act(() => chartRoot.unmount());
  });

  let host: HTMLDivElement, root: Root;
  beforeEach(() => {
    host = document.createElement("div");
    root = createRoot(host);
  });
  afterEach(() => {
    act(() => root.unmount());
  });
  function show(payment: number, other = 0) {
    act(() =>
      root.render(
        <CreditCardSummary
          currency="INR"
          transactions={[
            row(),
            row({ direction: "in", amount: 50, expenseOffset: true, cardCreditKind: "refund" }),
            row({
              direction: "in",
              amount: payment,
              flowType: "transfer",
              cardCreditKind: "payment",
            }),
            ...(other
              ? [
                  row({
                    direction: "in",
                    amount: other,
                    flowType: "transfer",
                    cardCreditKind: "unclassified",
                  }),
                ]
              : []),
          ]}
        />,
      ),
    );
    return host.querySelector('[role="status"]')!.textContent;
  }
  it("shows a match after netting fuel credits and explains the comparison", () => {
    expect(show(450)).toContain("Payments match net expenses");
    expect(host.textContent).toContain("not the amount due");
    expect(host.textContent).toContain("transfers, not income");
  });
  it("shows uncovered expenses", () => {
    expect(show(400)).toContain("below net expenses");
  });
  it("shows excess payments", () => {
    expect(show(500)).toContain("exceed net expenses");
  });
  it("requires review of unidentified credits before claiming a match", () => {
    expect(show(450, 5)).toContain("Needs review");
  });
  it("shows a prior-period expense credit instead of inventing income", () => {
    act(() =>
      root.render(
        <CreditCardSummary
          currency="INR"
          transactions={[
            row({ direction: "in", amount: 50, expenseOffset: true, cardCreditKind: "refund" }),
          ]}
        />,
      ),
    );
    expect(host.querySelector('[role="status"]')?.textContent).toContain("Refunds exceed charges");
  });
});
