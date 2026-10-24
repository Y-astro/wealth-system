// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { BankSankey } from "./bank-sankey";
import type { StatementTransaction } from "./statement-data";

let root: Root | undefined;
let host: HTMLDivElement;
const rows: StatementTransaction[] = [
  {
    id: "salary",
    date: "2026-04-01",
    description: "Payroll salary",
    amount: 1000,
    direction: "in",
    currency: "INR",
    category: "Salary",
    flowType: "income",
    unit: "cash",
    sourceLines: [1],
  },
  {
    id: "food",
    date: "2026-04-02",
    description: "BIGBASKET groceries",
    amount: 200,
    direction: "out",
    currency: "INR",
    category: "Other expenses",
    flowType: "expense",
    unit: "cash",
    sourceLines: [2],
  },
];
async function mount() {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<BankSankey transactions={rows} currency="INR" />));
}
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  root = undefined;
});
describe("interactive Sankey", () => {
  it("shows a populated amount and transaction count on hover", async () => {
    await mount();
    const node = host.querySelector('g[aria-label^="Expenses:"]')!;
    await act(async () => node.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })));
    const tooltip = host.querySelector('[data-testid="cash-flow-tooltip"]')!;
    expect(tooltip.textContent).toContain("Expenses");
    expect(tooltip.textContent).toContain("200");
    expect(tooltip.textContent).toContain("1 transaction");
  });
  it("clicks through Expenses, category, and statement transaction evidence", async () => {
    await mount();
    await act(async () =>
      host
        .querySelector('g[aria-label^="Expenses:"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    expect(host.textContent).toContain("EXPENSE CATEGORIES");
    await act(async () =>
      host
        .querySelector('g[aria-label^="Groceries:"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    expect(host.textContent).toContain("STATEMENT DESCRIPTIONS");
    expect(
      host.querySelector('[aria-label="Cash flow transaction breakdown"]')?.textContent,
    ).toContain("BIGBASKET groceries");
  });
  it("supports keyboard expansion", async () => {
    await mount();
    await act(async () =>
      host
        .querySelector('g[aria-label^="Expenses:"]')!
        .dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })),
    );
    expect(host.textContent).toContain("EXPENSE CATEGORIES");
  });
});
