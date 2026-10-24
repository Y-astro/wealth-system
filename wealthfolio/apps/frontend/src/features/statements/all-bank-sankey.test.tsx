// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AllBankSankey } from "./all-bank-sankey";
import type { StatementReport } from "./statement-data";
const state = vi.hoisted(() => ({ hidden: false }));
vi.mock("@/hooks/use-balance-privacy", () => ({
  useBalancePrivacy: () => ({ isBalanceHidden: state.hidden }),
}));
const report = (id: string, kind = "bank", amount = 100, currency = "INR") =>
  ({
    id,
    kind,
    filename: id + ".pdf",
    currency,
    period: { from: "2026-04-01", to: "2026-04-30" },
    warnings: [],
    transactions: [
      {
        id: "same-local-id",
        date: "2026-04-01",
        description: "Groceries",
        amount,
        direction: "out",
        currency,
        category: "Groceries",
        flowType: "expense",
        unit: "cash",
        sourceLines: [1],
      },
    ],
  }) as unknown as StatementReport;
let root: Root | undefined, host: HTMLDivElement;
beforeEach(() => {
  const saved = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => saved.get(key) ?? null,
    setItem: (key: string, value: string) => saved.set(key, value),
  });
});
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  state.hidden = false;
  vi.unstubAllGlobals();
});
async function mount(
  reports = [report("Bank A"), report("Bank B"), report("NSDL", "investment", 999999)],
) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<AllBankSankey reports={reports} />));
}
describe("all-bank Sankey view", () => {
  it("excludes NSDL files and preserves bank file provenance in the drilldown", async () => {
    await mount();
    expect(host.textContent).toContain("2 bank cash transactions");
    expect(host.textContent).not.toContain("NSDL.pdf");
    await act(async () =>
      host
        .querySelector('g[aria-label^="Expenses:"]')!
        .dispatchEvent(new MouseEvent("click", { bubbles: true })),
    );
    const detail = host.querySelector('[aria-label="Cash flow transaction breakdown"]')!;
    expect(detail.textContent).toContain("Bank A.pdf");
    expect(detail.textContent).toContain("Bank B.pdf");
    expect(detail.textContent).not.toContain("999999");
  });
  it("allows a bank file to be excluded from the combined chart", async () => {
    await mount();
    const checkbox = host.querySelector(
      'input[aria-label="Include Bank B.pdf"]',
    ) as HTMLInputElement;
    await act(async () => checkbox.click());
    expect(host.textContent).toContain("1 bank cash transaction");
    expect(host.textContent).toContain("1 selected statements");
  });
  it("switches currencies without adding foreign cash to INR", async () => {
    await mount([report("INR bank"), report("USD bank", "bank", 50, "USD")]);
    const selector = host.querySelector(
      'select[aria-label="Combined bank currency"]',
    ) as HTMLSelectElement;
    await act(async () => {
      selector.value = "USD";
      selector.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(host.querySelector("svg")?.getAttribute("aria-label")).toContain("Cash flow in USD");
    expect(host.textContent).toContain("1 bank cash transaction");
  });
  it("respects the dashboard privacy setting", async () => {
    state.hidden = true;
    await mount();
    expect(host.querySelector("svg")).toBeNull();
    expect(host.textContent).toContain("Show balances");
  });
});
