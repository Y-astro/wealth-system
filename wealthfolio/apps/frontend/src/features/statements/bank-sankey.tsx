import { useId, useMemo, useState } from "react";
import {
  detailedCashFlow,
  spendingCategory,
  type CashNode,
  type FlowKind,
} from "./bank-sankey-data";
import { formatMoney, type StatementTransaction } from "./statement-data";

const colors: Record<FlowKind, string> = {
  income: "#14b8a6",
  expense: "#f59e0b",
  transfer: "#a78bfa",
  balance: "#22c55e",
  cash: "#3b82f6",
};
const buttonClass =
  "rounded-md border px-3 py-2 text-sm hover:bg-muted focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary";
type Hover = { label: string; amount: number; count: number; percent: number; basis: string };
type PlacedNode = CashNode & { x: number; y: number; height: number };

export function BankSankey({
  transactions,
  currency,
  creditCard = false,
}: {
  transactions: StatementTransaction[];
  currency: string;
  creditCard?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const [category, setCategory] = useState<string | null>(null);
  const [detail, setDetail] = useState<"expenses" | "transfers" | null>(null);
  const [hover, setHover] = useState<Hover | null>(null);
  const [query, setQuery] = useState("");
  const [showAll, setShowAll] = useState(false);
  const graph = useMemo(() => {
    const result = detailedCashFlow(transactions, currency, expanded, category);
    if (creditCard)
      for (const node of result.nodes) {
        if (node.key === "cash") node.name = "Card funding";
        if (node.key === "funding") node.name = "Charges above payments / credits";
        if (node.key === "retained") node.name = "Payments / credits above charges";
        if (node.name === "Transfers in · Card repayments") node.name = "Card payments";
      }
    return result;
  }, [transactions, currency, expanded, category, creditCard]);
  const diagramId = useId().replace(/:/g, "");
  const maxLevel = Math.max(2, ...graph.nodes.map((node) => node.level));
  const columns = Array.from({ length: maxLevel + 1 }, (_, level) =>
    graph.nodes.filter((node) => node.level === level),
  );
  const maxCount = Math.max(1, ...columns.map((column) => column.length));
  const width = maxLevel === 4 ? 1360 : maxLevel === 3 ? 1120 : 960;
  const height = Math.max(420, maxCount * 48 + 80);
  const gap = 26;
  const scale = Math.min(
    ...columns
      .filter((column) => column.length)
      .map(
        (column) =>
          Math.max(1, height - 80 - gap * (column.length - 1) - 6 * column.length) /
          Math.max(
            1,
            column.reduce((sum, node) => sum + node.value, 0),
          ),
      ),
  );
  const placed = new Map<string, PlacedNode>();
  for (let level = 0; level < columns.length; level++) {
    const column = columns[level];
    const total =
      column.reduce((sum, node) => sum + 6 + node.value * scale, 0) +
      gap * Math.max(0, column.length - 1);
    let y = 40 + (height - 80 - total) / 2;
    for (const node of column) {
      const nodeHeight = 6 + node.value * scale;
      placed.set(node.key, {
        ...node,
        x: 24 + (level * (width - 70)) / maxLevel,
        y,
        height: nodeHeight,
      });
      y += nodeHeight + gap;
    }
  }
  const sourceOffset = new Map<number, number>(),
    targetOffset = new Map<number, number>();
  const paths = graph.links.map((link, index) => {
    const source = placed.get(graph.nodes[link.source].key)!;
    const target = placed.get(graph.nodes[link.target].key)!;
    const thickness = link.value * scale;
    const sourceY = source.y + 3 + (sourceOffset.get(link.source) || 0) + thickness / 2;
    const targetY = target.y + 3 + (targetOffset.get(link.target) || 0) + thickness / 2;
    sourceOffset.set(link.source, (sourceOffset.get(link.source) || 0) + thickness);
    targetOffset.set(link.target, (targetOffset.get(link.target) || 0) + thickness);
    const x1 = source.x + 14,
      x2 = target.x;
    const curve = (x2 - x1) * 0.5;
    const selected =
      !category || link.category === category || source.key === "cash" || target.key === "cash";
    const denominator = source.key.startsWith("expense:")
      ? source.value
      : link.category
        ? graph.spending
        : graph.available;
    const info = {
      label: source.name + " → " + target.name,
      amount: link.value,
      count: link.count,
      percent: denominator ? (link.value / denominator) * 100 : 0,
      basis: source.key.startsWith("expense:")
        ? "of this category"
        : link.category
          ? "of expenses"
          : creditCard
            ? "of card funding"
            : "of available cash",
    };
    return {
      ...link,
      index,
      source,
      target,
      thickness,
      selected,
      info,
      path: `M ${x1},${sourceY} C ${x1 + curve},${sourceY} ${x2 - curve},${targetY} ${x2},${targetY}`,
    };
  });
  const detailItems = (
    detail === "transfers"
      ? graph.cash.filter((item) => item.direction === "out" && item.flowType === "transfer")
      : graph.expenses
  ).filter(
    (item) =>
      (!category || spendingCategory(item) === category) &&
      (!query || item.description.toLowerCase().includes(query.toLowerCase())),
  );
  const visible = showAll ? detailItems : detailItems.slice(0, 30);
  const format = (value: number) => formatMoney(value, currency);
  const reset = () => {
    setExpanded(false);
    setCategory(null);
    setDetail(null);
    setQuery("");
    setHover(null);
  };

  function activate(node: CashNode) {
    setQuery("");
    setShowAll(false);
    if (node.key === "expenses") {
      setExpanded(true);
      setCategory(null);
      setDetail("expenses");
    } else if (node.key.startsWith("expense:")) {
      setExpanded(true);
      setCategory(node.category || null);
      setDetail("expenses");
    } else if (node.key === "transfers") {
      setCategory(null);
      setDetail("transfers");
    } else if (node.key.startsWith("description:")) {
      setDetail("expenses");
    }
  }
  function nodeInfo(node: CashNode): Hover {
    const parent = node.key.startsWith("description:")
      ? graph.nodes.find((entry) => entry.key === "expense:" + node.category)?.value ||
        graph.spending
      : node.category
        ? graph.spending
        : graph.available;
    return {
      label: node.name,
      amount: node.value,
      count: node.count,
      percent: parent ? (node.value / parent) * 100 : 0,
      basis: node.key.startsWith("description:")
        ? "of this category"
        : node.category
          ? "of expenses"
          : creditCard
            ? "of card funding"
            : "of available cash",
    };
  }

  if (!graph.links.length)
    return (
      <p className="text-muted-foreground rounded-lg border p-4 text-sm">
        No cash transactions match these filters.
      </p>
    );
  return (
    <section className="rounded-xl border p-4" aria-label="Interactive bank cash flow">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="font-semibold">
            {creditCard ? "Card charges and funding" : "Where your money flows"}
          </h3>
          <p className="text-muted-foreground mt-1 max-w-3xl text-sm">
            Click Expenses to split spending into categories. Click a category to see its statement
            descriptions and transactions. Transfers and balances stay separate from spending.
          </p>
          {graph.refundTotal > 0 && (
            <p className="text-muted-foreground mt-2 max-w-3xl text-sm">
              {format(graph.refundTotal)} of expense refunds / fuel credits reduce their expense
              categories. Excess category refunds appear as expense credits, never income. Chart
              charges are net of these adjustments; review the original amounts in the transaction
              table below.
            </p>
          )}
        </div>
        <div className="flex gap-2">
          <button
            className={buttonClass}
            aria-expanded={expanded}
            onClick={() => {
              setExpanded(!expanded);
              setCategory(null);
              setDetail(expanded ? null : "expenses");
              setQuery("");
            }}
          >
            {expanded ? "Collapse expenses" : "Expand expenses"}
          </button>
          {detail && (
            <button className={buttonClass} onClick={reset}>
              Reset view
            </button>
          )}
        </div>
      </div>
      <div className="mt-4 flex flex-wrap gap-x-5 gap-y-2 text-xs">
        {(["income", "expense", "transfer", "balance"] as const)
          .filter((kind) => !creditCard || kind !== "income")
          .map((kind) => (
            <span key={kind} className="flex items-center gap-2">
              <span
                className="h-2.5 w-2.5 rounded-full"
                style={{ backgroundColor: colors[kind] }}
              />
              {kind === "balance"
                ? creditCard
                  ? "Payment difference / expense credits"
                  : "Retained cash / funding"
                : kind === "expense"
                  ? "Expenses"
                  : kind === "transfer"
                    ? "Transfers"
                    : "Income"}
            </span>
          ))}
      </div>
      <div
        className="mt-4 min-h-[76px] rounded-lg border p-3 text-sm"
        style={{ background: "var(--card, #111827)", color: "var(--card-foreground, #f8fafc)" }}
        aria-live="polite"
        aria-atomic="true"
        data-testid="cash-flow-tooltip"
      >
        {hover ? (
          <>
            <p className="font-medium">{hover.label}</p>
            <p className="mt-1">
              <strong>{format(hover.amount)}</strong> · {hover.percent.toFixed(1)}% {hover.basis}
              {hover.count > 0
                ? ` · ${hover.count} transaction${hover.count === 1 ? "" : "s"}`
                : " · Balancing cash amount"}
            </p>
          </>
        ) : (
          <>
            <p className="font-medium">Hover or focus a flow to see its value</p>
            <p className="text-muted-foreground mt-1">
              Amounts, shares, and transaction counts appear here. Every expense branch adds up to
              its parent.
            </p>
          </>
        )}
      </div>
      <div
        className="bg-muted/10 mt-3 overflow-x-auto rounded-lg"
        aria-label="Scrollable cash flow diagram"
      >
        <svg
          viewBox={`0 0 ${width} ${height}`}
          style={{ width: "100%", minWidth: width, height }}
          role="group"
          aria-label={`Cash flow in ${currency}; expenses ${format(graph.spending)}. Click an expense node to explore.`}
        >
          <defs>
            {paths.map((path) => (
              <linearGradient key={path.index} id={`${diagramId}-${path.index}`} x1="0" x2="1">
                <stop offset="0%" stopColor={colors[path.source.kind]} />
                <stop offset="100%" stopColor={colors[path.target.kind]} />
              </linearGradient>
            ))}
          </defs>
          {columns.map((_, level) => (
            <text
              key={level}
              x={24 + (level * (width - 70)) / maxLevel}
              y={20}
              fontSize={10}
              fill="currentColor"
              opacity={0.6}
              textAnchor={level === maxLevel ? "end" : "start"}
            >
              {
                [
                  creditCard ? "PAYMENTS / CREDITS" : "MONEY IN",
                  creditCard ? "CARD FUNDING" : "AVAILABLE CASH",
                  creditCard ? "NET CHARGES" : "MONEY OUT",
                  "EXPENSE CATEGORIES",
                  "STATEMENT DESCRIPTIONS",
                ][level]
              }
            </text>
          ))}
          {paths.map((path) => (
            <path
              key={path.index}
              d={path.path}
              fill="none"
              stroke={`url(#${diagramId}-${path.index})`}
              strokeWidth={Math.max(0.6, path.thickness)}
              strokeOpacity={
                hover
                  ? hover.label === path.info.label ||
                    hover.label === path.source.name ||
                    hover.label === path.target.name
                    ? 0.8
                    : 0.14
                  : path.selected
                    ? 0.4
                    : 0.12
              }
              tabIndex={0}
              role="img"
              aria-label={`${path.info.label}: ${format(path.value)}`}
              onMouseEnter={() => setHover(path.info)}
              onMouseLeave={() => setHover(null)}
              onFocus={() => setHover(path.info)}
              onBlur={() => setHover(null)}
            >
              <title>
                {path.info.label}: {format(path.value)} · {path.count} transactions
              </title>
            </path>
          ))}
          {[...placed.values()].map((node) => {
            const interactive =
              node.key === "expenses" ||
              node.key === "transfers" ||
              node.key.startsWith("expense:");
            const right = node.level === maxLevel;
            const labelX = right ? node.x - 10 : node.x + 24;
            return (
              <g
                key={node.key}
                role={interactive ? "button" : "img"}
                tabIndex={0}
                aria-label={`${node.name}: ${format(node.value)}${interactive ? ". Show breakdown" : ""}`}
                style={{ cursor: interactive ? "pointer" : "default" }}
                onClick={() => activate(node)}
                onKeyDown={(event) => {
                  if (interactive && (event.key === "Enter" || event.key === " ")) {
                    event.preventDefault();
                    activate(node);
                  }
                }}
                onMouseEnter={() => setHover(nodeInfo(node))}
                onMouseLeave={() => setHover(null)}
                onFocus={() => setHover(nodeInfo(node))}
                onBlur={() => setHover(null)}
              >
                <title>
                  {node.name}: {format(node.value)} · {node.count} transactions
                  {interactive ? " · Click to explore" : ""}
                </title>
                <rect
                  x={node.x}
                  y={node.y}
                  width={14}
                  height={node.height}
                  rx={4}
                  fill={colors[node.kind]}
                  stroke={category === node.category && category ? "currentColor" : "none"}
                  strokeWidth={2}
                />
                <text
                  x={labelX}
                  y={node.y + node.height / 2 - 4}
                  textAnchor={right ? "end" : "start"}
                  fill="currentColor"
                  fontSize={12}
                  fontWeight={500}
                  style={{
                    paintOrder: "stroke",
                    stroke: "var(--background, #111827)",
                    strokeWidth: 3,
                    strokeLinejoin: "round",
                  }}
                >
                  {node.name.length > 33 ? node.name.slice(0, 30) + "…" : node.name}
                  {node.key === "expenses" && !expanded ? " +" : ""}
                </text>
                <text
                  x={labelX}
                  y={node.y + node.height / 2 + 13}
                  textAnchor={right ? "end" : "start"}
                  fill="currentColor"
                  fontSize={11}
                  opacity={0.85}
                  style={{
                    paintOrder: "stroke",
                    stroke: "var(--background, #111827)",
                    strokeWidth: 3,
                  }}
                >
                  {format(node.value)}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
      <p className="text-muted-foreground mt-2 text-xs">
        Amounts use {currency} only. Unknown descriptions remain uncategorized; payment modes such
        as UPI or NEFT do not by themselves prove a transfer. Scroll sideways on smaller screens.
      </p>
      {detail && (
        <div className="mt-5 rounded-lg border p-4" aria-label="Cash flow transaction breakdown">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="flex flex-wrap items-center gap-2 text-sm">
                <button className="underline" onClick={reset}>
                  All cash flows
                </button>
                <span>›</span>
                <button
                  className="underline"
                  onClick={() => {
                    setCategory(null);
                    setQuery("");
                  }}
                >
                  {detail === "transfers" ? "Transfers out" : "Expenses"}
                </button>
                {category && (
                  <>
                    <span>›</span>
                    <strong>{category}</strong>
                  </>
                )}
              </div>
              <p className="mt-2 font-medium">
                {format(detailItems.reduce((sum, item) => sum + item.amount, 0))} ·{" "}
                {detailItems.length} matching transactions
              </p>
            </div>
            <input
              aria-label="Filter cash flow transactions"
              className="bg-background min-h-10 rounded-md border px-3 text-sm"
              placeholder="Search statement descriptions"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setShowAll(false);
              }}
            />
          </div>
          {detail === "expenses" && (
            <div className="mt-3 flex flex-wrap gap-2">
              {[...new Set(graph.expenses.map(spendingCategory))].sort().map((name) => (
                <button
                  key={name}
                  className={buttonClass}
                  aria-pressed={category === name}
                  onClick={() => {
                    setExpanded(true);
                    setCategory(name);
                    setQuery("");
                    setShowAll(false);
                  }}
                >
                  {name}
                </button>
              ))}
            </div>
          )}
          <div className="mt-3 overflow-x-auto">
            <table className="w-full text-left text-sm">
              <caption className="sr-only">Transactions behind the selected cash flow</caption>
              <thead>
                <tr className="border-b">
                  <th className="py-2 pr-4">Date</th>
                  <th className="py-2 pr-4">Statement description</th>
                  <th className="py-2 pr-4">Category</th>
                  <th className="py-2 text-right">Amount</th>
                </tr>
              </thead>
              <tbody>
                {visible.map((item) => (
                  <tr key={item.id} className="border-b">
                    <td className="whitespace-nowrap py-3 pr-4">{item.date}</td>
                    <td className="min-w-56 py-3 pr-4">
                      {item.description}
                      {"statementFilename" in item &&
                        typeof item.statementFilename === "string" && (
                          <p className="text-muted-foreground mt-1 break-words text-xs">
                            {item.statementFilename}
                          </p>
                        )}
                    </td>
                    <td className="py-3 pr-4">{spendingCategory(item)}</td>
                    <td className="whitespace-nowrap py-3 text-right tabular-nums">
                      {format(item.amount)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {!visible.length && <p className="py-4 text-sm">No transactions match this search.</p>}
          </div>
          {!showAll && detailItems.length > 30 && (
            <button className={buttonClass + " mt-3"} onClick={() => setShowAll(true)}>
              Show all {detailItems.length} transactions
            </button>
          )}
          <p className="text-muted-foreground mt-3 text-xs">
            Generic categories are refined from recognizable statement descriptions. Specific
            reviewed categories are preserved. You can correct categories in the statement review
            table.
          </p>
        </div>
      )}
      <details className="mt-4 text-sm">
        <summary className="cursor-pointer">Exact flow amounts</summary>
        <div className="overflow-x-auto">
          <table className="mt-3 w-full text-left">
            <thead>
              <tr>
                <th>From</th>
                <th>To</th>
                <th className="text-right">Amount</th>
              </tr>
            </thead>
            <tbody>
              {graph.links.map((link, index) => (
                <tr className="border-t" key={index}>
                  <td className="py-2 pr-3">{graph.nodes[link.source].name}</td>
                  <td className="pr-3">{graph.nodes[link.target].name}</td>
                  <td className="text-right tabular-nums">{format(link.value)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </section>
  );
}
