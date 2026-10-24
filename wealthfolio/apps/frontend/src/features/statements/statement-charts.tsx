import { useState } from "react";
import { StatementChartTooltip } from "./statement-chart-tooltip";
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  formatMoney,
  groupValues,
  type StatementHolding,
  type StatementReport,
} from "./statement-data";

const COLORS = ["#2563eb", "#0d9488", "#d97706", "#7c3aed", "#e11d48", "#0891b2"];
const axis = { fontSize: 11, fill: "var(--muted-foreground)" };

export function PortfolioCharts({
  report,
  holdings,
}: {
  report: StatementReport;
  holdings: StatementHolding[];
}) {
  const [showAll, setShowAll] = useState(false);
  const allocation = groupValues(
    holdings,
    (h) => h.assetClass,
    (h) => h.value,
  ).filter((v) => v.value > 0);
  const positions = groupValues(
    holdings,
    (h) => h.isin || h.name,
    (h) => h.value,
  )
    .filter((v) => v.value > 0)
    .slice(0, showAll ? undefined : 12);
  const positionLabels = new Map(holdings.map((h) => [h.isin || h.name, h.symbol || h.name]));
  return (
    <div className="grid min-w-0 gap-5 xl:grid-cols-2">
      <section className="bg-card min-w-0 rounded-xl border p-5 xl:col-span-2">
        <h2 className="font-semibold">Portfolio value over time</h2>
        <p className="text-muted-foreground mb-4 text-sm">
          Whole portfolio · statement month-end values. Changes include deposits and withdrawals.
        </p>
        {report.history.length > 1 ? (
          <div
            className="h-64"
            role="img"
            aria-label={`Portfolio values for ${report.history.length} statement months. Exact values in the history table below.`}
          >
            <ResponsiveContainer width="100%" height="100%" minWidth={0}>
              <AreaChart data={report.history} margin={{ left: 20, right: 15, top: 10, bottom: 5 }}>
                <defs>
                  <linearGradient id="statement-value-fill" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor={COLORS[0]} stopOpacity={0.3} />
                    <stop offset="100%" stopColor={COLORS[0]} stopOpacity={0.02} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="var(--border)" />
                <XAxis
                  dataKey="date"
                  tickFormatter={(date: string) => date.slice(0, 7)}
                  tick={axis}
                  minTickGap={35}
                />
                <YAxis
                  tickFormatter={(v: number) => formatMoney(v, report.currency, true)}
                  tick={axis}
                  width={82}
                  domain={["auto", "auto"]}
                />
                <Tooltip
                  content={(props) => (
                    <StatementChartTooltip {...props} currency={report.currency} />
                  )}
                />
                <Area
                  type="linear"
                  dataKey="value"
                  stroke={COLORS[0]}
                  strokeWidth={2}
                  fill="url(#statement-value-fill)"
                  isAnimationActive={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        ) : (
          <p className="text-muted-foreground py-12 text-center">
            No historical series in this statement. Upload a statement with monthly values to see
            this chart.
          </p>
        )}
        {report.history.length > 0 && (
          <details className="mt-3 text-sm">
            <summary className="cursor-pointer">Exact monthly values</summary>
            <div className="mt-2 max-h-64 overflow-auto">
              <table className="w-full text-left">
                <caption className="sr-only">Whole portfolio monthly values</caption>
                <thead>
                  <tr>
                    <th scope="col">Month</th>
                    <th scope="col">Value</th>
                    <th scope="col">Change</th>
                    <th scope="col">Change %</th>
                  </tr>
                </thead>
                <tbody>
                  {report.history.map((h) => (
                    <tr key={h.date} className="border-t">
                      <td className="py-2">{h.date.slice(0, 7)}</td>
                      <td>{formatMoney(h.value, report.currency)}</td>
                      <td>{formatMoney(h.change, report.currency)}</td>
                      <td>{h.changePercent == null ? "—" : `${h.changePercent}%`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </details>
        )}
      </section>
      <section className="bg-card min-w-0 rounded-xl border p-5">
        <h2 className="font-semibold">Asset allocation</h2>
        <p className="text-muted-foreground text-sm">
          Based on the account and holding filters below
        </p>
        {allocation.length > 0 ? (
          <>
            <div
              className="h-64"
              role="img"
              aria-label="Asset allocation. Exact values listed below."
            >
              <ResponsiveContainer width="100%" height="100%" minWidth={0}>
                <PieChart>
                  <Pie
                    data={allocation}
                    dataKey="value"
                    nameKey="name"
                    innerRadius="55%"
                    outerRadius="85%"
                    paddingAngle={2}
                    isAnimationActive={false}
                  >
                    {allocation.map((a, i) => (
                      <Cell key={a.name} fill={COLORS[i % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    content={(props) => (
                      <StatementChartTooltip
                        {...props}
                        currency={report.currency}
                        total={allocation.reduce((sum, row) => sum + row.value, 0)}
                      />
                    )}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
            <ul className="space-y-2 text-sm">
              {allocation.map((a, i) => (
                <li key={a.name} className="flex justify-between gap-3">
                  <span>
                    <span
                      className="mr-2 inline-block h-2 w-2 rounded-full"
                      style={{ background: COLORS[i % COLORS.length] }}
                    />
                    {a.name}
                  </span>
                  <span className="tabular-nums">{formatMoney(a.value, report.currency)}</span>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <p className="py-16 text-center">No holdings match these filters.</p>
        )}
      </section>
      <section className="bg-card min-w-0 rounded-xl border p-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="font-semibold">Position values</h2>
          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={showAll}
              onChange={(e) => setShowAll(e.target.checked)}
            />
            Show every position
          </label>
        </div>
        <p className="text-muted-foreground mb-4 text-sm">
          {showAll ? "All positions" : "Top 12"} · the same ISIN is combined across selected
          accounts
        </p>
        <div
          style={{ height: Math.max(384, positions.length * 32) }}
          role="img"
          aria-label="Largest holdings by statement value. All holdings and values are in the table below."
        >
          <ResponsiveContainer width="100%" height="100%" minWidth={0}>
            <BarChart data={positions} layout="vertical" margin={{ left: 5, right: 10 }}>
              <XAxis
                type="number"
                tick={axis}
                tickFormatter={(v: number) => formatMoney(v, report.currency, true)}
              />
              <YAxis
                type="category"
                dataKey="name"
                width={115}
                tick={axis}
                tickFormatter={(v: string) => {
                  const label = positionLabels.get(v) || v;
                  return label.length > 17 ? label.slice(0, 15) + "…" : label;
                }}
              />
              <Tooltip
                content={(props) => (
                  <StatementChartTooltip
                    {...props}
                    currency={report.currency}
                    label={positionLabels.get(String(props.label)) || props.label}
                  />
                )}
              />
              <Bar
                dataKey="value"
                name="Statement value"
                fill={COLORS[0]}
                radius={[0, 4, 4, 0]}
                isAnimationActive={false}
              />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </section>
    </div>
  );
}

export { BankSankey } from "./bank-sankey";
