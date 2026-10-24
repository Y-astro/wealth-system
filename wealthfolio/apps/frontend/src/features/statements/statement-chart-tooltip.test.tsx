// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { StatementChartTooltip } from "./statement-chart-tooltip";

describe("statement chart tooltips", () => {
  let host: HTMLDivElement;
  let root: Root;
  beforeEach(() => {
    host = document.createElement("div");
    root = createRoot(host);
  });
  afterEach(() => {
    act(() => root.unmount());
  });
  it("shows a pie slice's category, exact value and share with visible colors", () => {
    act(() =>
      root.render(
        <StatementChartTooltip
          active
          label=""
          currency="INR"
          total={1000}
          payload={[{ name: "Equity", value: 750, payload: { name: "Equity" } }]}
        />,
      ),
    );
    expect(host.textContent).toContain("Equity");
    expect(host.textContent).toContain("750");
    expect(host.textContent).toContain("75.0%");
    expect(host.querySelector<HTMLElement>("[role=status]")?.style.color).toBe(
      "rgb(248, 250, 252)",
    );
  });
  it("renders zero values and a resolved history or position label", () => {
    act(() =>
      root.render(
        <StatementChartTooltip
          active
          currency="USD"
          label="2026-04-30"
          payload={[{ value: 0, name: "value" }]}
        />,
      ),
    );
    expect(host.textContent).toContain("2026-04-30");
    expect(host.textContent).toContain("$0.00");
  });
  it("does not create empty dark rectangles for missing or invalid payloads", () => {
    for (const payload of [undefined, [], [{ value: undefined }], [{ value: "bad" }]]) {
      act(() => root.render(<StatementChartTooltip active currency="INR" payload={payload} />));
      expect(host.childElementCount).toBe(0);
    }
    act(() =>
      root.render(<StatementChartTooltip active={false} currency="INR" payload={[{ value: 5 }]} />),
    );
    expect(host.childElementCount).toBe(0);
  });
});
