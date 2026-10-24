// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { StatementBalancesDashboard } from "./statement-balances-dashboard";
const state = vi.hoisted(() => ({ hidden: false, query: {} as any, options: {} as any }));
vi.mock("@/hooks/use-balance-privacy", () => ({
  useBalancePrivacy: () => ({ isBalanceHidden: state.hidden }),
}));
vi.mock("@tanstack/react-query", () => ({
  useQuery: (options: unknown) => {
    state.options = options;
    return state.query;
  },
}));
function entry(id: string, filename: string, value: number) {
  return {
    id,
    filename,
    status: "ready",
    report: {
      id,
      filename,
      kind: "bank",
      currency: "INR",
      transactions: [
        { date: "2026-01-01", balance: value, amount: 10, currency: "INR", unit: "cash" },
      ],
      history: [],
      summary: { openingBalance: value - 10 },
      period: { from: "2026-01-01", to: "2026-01-01" },
      warnings: [],
    },
  };
}
let host: HTMLDivElement, root: Root | undefined;
async function render() {
  if (!root) {
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
  }
  await act(async () =>
    root!.render(
      <MemoryRouter>
        <StatementBalancesDashboard />
      </MemoryRouter>,
    ),
  );
}
const storage = new Map<string, string>();
beforeEach(() => {
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => {
      storage.set(key, value);
    },
    clear: () => storage.clear(),
  });
});
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  storage.clear();
  vi.unstubAllGlobals();
  state.hidden = false;
});
describe("saved statement dashboard", () => {
  it("restores the selected saved statement and remembers a new selection", async () => {
    state.query = {
      data: [entry("a", "AU.xlsx", 100), entry("b", "Axis.xls", 200)],
      isPending: false,
      isError: false,
    };
    localStorage.setItem("dashboard-statement-id", "b");
    await render();
    expect(host.querySelector('[aria-pressed="true"]')?.textContent).toContain("Axis.xls");
    const card = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("AU.xlsx"),
    )!;
    await act(async () => card.click());
    expect(localStorage.getItem("dashboard-statement-id")).toBe("a");
    expect(host.querySelector('[aria-pressed="true"]')?.textContent).toContain("AU.xlsx");
  });
  it("adds newly processed reports and polls faster while processing", async () => {
    state.query = {
      data: [{ id: "a", status: "processing", report: null }],
      isPending: false,
      isError: false,
    };
    await render();
    expect(host.textContent).toContain("updates automatically");
    expect(state.options.refetchInterval({ state: { data: state.query.data } })).toBe(3000);
    state.query = { ...state.query, data: [entry("a", "Ready.xlsx", 123)] };
    await render();
    expect(host.querySelector('svg[aria-label="Statement balance over time"]')).not.toBeNull();
    expect(host.textContent).toContain("Ready.xlsx");
    expect(state.options.refetchInterval({ state: { data: state.query.data } })).toBe(30000);
  });
  it("hides amounts, chart values and the balance table when privacy is enabled", async () => {
    state.hidden = true;
    state.query = { data: [entry("a", "Private.xlsx", 987654)], isPending: false, isError: false };
    await render();
    expect(host.textContent).toContain("Balance history is hidden");
    expect(host.textContent).not.toContain("9,87,654");
    expect(host.querySelector("svg")).toBeNull();
    expect(host.querySelector("tbody")?.textContent).toContain("••••••");
  });
});
