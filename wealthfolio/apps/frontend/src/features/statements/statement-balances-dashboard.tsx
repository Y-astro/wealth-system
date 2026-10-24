import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { Button } from "@wealthfolio/ui";
import { useBalancePrivacy } from "@/hooks/use-balance-privacy";
import { readLibrary } from "./statement-library";
import { formatMoney } from "./statement-data";
import {
  filterBalanceHistory,
  statementBalanceHistory,
  type BalancePoint,
} from "./statement-balances";

const SELECTION_KEY = "dashboard-statement-id";
const ranges = [
  { label: "1M", days: 31 },
  { label: "3M", days: 92 },
  { label: "1Y", days: 366 },
  { label: "All", days: null },
];

function BalanceTimeline({
  points,
  currency,
  hidden,
  bank,
}: {
  points: BalancePoint[];
  currency: string;
  hidden: boolean;
  bank: boolean;
}) {
  const [activeDate, setActiveDate] = useState<string | null>(null);
  if (!points.length)
    return (
      <p className="text-muted-foreground py-16 text-center">
        This statement has no dated balances. Its extracted transactions are available in
        Statements.
      </p>
    );
  const active = points.find((point) => point.date === activeDate) ?? points[points.length - 1];
  const values = points.map((point) => point.value);
  const low = Math.min(...values),
    high = Math.max(...values);
  const spread = high - low || Math.max(Math.abs(high) * 0.1, 1);
  const min = low - spread * 0.12,
    max = high + spread * 0.12;
  const first = Date.parse(points[0].date + "T00:00:00Z"),
    last = Date.parse(points[points.length - 1].date + "T00:00:00Z");
  const x = (point: BalancePoint) =>
    last === first
      ? 480
      : 30 + ((Date.parse(point.date + "T00:00:00Z") - first) / (last - first)) * 900;
  const y = (point: BalancePoint) => 240 - ((point.value - min) / (max - min)) * 200;
  const path = points
    .map((point, index) =>
      index === 0
        ? `M${x(point)},${y(point)}`
        : bank
          ? `H${x(point)}V${y(point)}`
          : `L${x(point)},${y(point)}`,
    )
    .join(" ");
  return (
    <div>
      <div className="mb-2 flex flex-wrap items-baseline justify-between gap-2" aria-live="polite">
        <span className="text-muted-foreground text-sm">
          {active.date} · {bank ? "Account balance" : "Reported portfolio value"}
        </span>
        <span className="font-mono text-lg">
          {hidden ? "••••••" : formatMoney(active.value, currency)}
        </span>
      </div>
      {hidden ? (
        <div className="text-muted-foreground flex h-64 items-center justify-center">
          Balance history is hidden
        </div>
      ) : (
        <svg
          viewBox="0 0 960 280"
          className="h-72 w-full"
          role="img"
          aria-label="Statement balance over time"
        >
          {[40, 140, 240].map((line) => (
            <line
              key={line}
              x1="30"
              x2="930"
              y1={line}
              y2={line}
              stroke="currentColor"
              opacity="0.12"
            />
          ))}
          <path d={path} fill="none" stroke="#14b8a6" strokeWidth="3" strokeLinejoin="round" />
          {points.map((point) => (
            <circle
              key={point.date}
              cx={x(point)}
              cy={y(point)}
              r={point.date === active.date ? 6 : 4}
              fill="#14b8a6"
              tabIndex={0}
              role="button"
              aria-label={`${point.date}: ${formatMoney(point.value, currency)}`}
              onMouseEnter={() => setActiveDate(point.date)}
              onFocus={() => setActiveDate(point.date)}
              onClick={() => setActiveDate(point.date)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  setActiveDate(point.date);
                }
              }}
            >
              <title>
                {point.date}: {formatMoney(point.value, currency)}
              </title>
            </circle>
          ))}
          <text x="30" y="274" fill="currentColor" fontSize="12">
            {points[0].date}
          </text>
          <text x="930" y="274" textAnchor="end" fill="currentColor" fontSize="12">
            {points[points.length - 1].date}
          </text>
        </svg>
      )}
      <p className="text-muted-foreground text-xs">
        {bank
          ? "Dated closing balances from the statement. Steps connect observed dates; gaps may contain unreported activity."
          : "Historical valuations reported in the statement. Balance changes include deposits, withdrawals and market movement."}
      </p>
    </div>
  );
}

export function StatementBalancesDashboard() {
  const { isBalanceHidden } = useBalancePrivacy();
  const [selectedId, setSelectedId] = useState(() => {
    try {
      return localStorage.getItem(SELECTION_KEY) ?? "";
    } catch {
      return "";
    }
  });
  const [days, setDays] = useState<number | null>(null);
  const library = useQuery({
    queryKey: ["dashboard-statement-library"],
    queryFn: readLibrary,
    refetchInterval: (query) =>
      query.state.data?.some((item) => item.status === "queued" || item.status === "processing")
        ? 3000
        : 30000,
    refetchOnWindowFocus: true,
  });
  const ready = library.data?.filter((item) => item.status === "ready" && item.report) ?? [];
  const selected = ready.find((item) => item.id === selectedId) ?? ready[0];
  const report = selected?.report;
  useEffect(() => {
    if (selected) {
      setSelectedId(selected.id);
      try {
        localStorage.setItem(SELECTION_KEY, selected.id);
      } catch {
        /* Storage may be unavailable in private browsing. */
      }
    }
  }, [selected?.id]);
  const history = useMemo(() => (report ? statementBalanceHistory(report) : []), [report]);
  const points = useMemo(() => filterBalanceHistory(history, days), [history, days]);
  const latest = history[history.length - 1];
  const pending =
    library.data?.filter((item) => item.status === "queued" || item.status === "processing")
      .length ?? 0;
  const failed = library.data?.filter((item) => item.status === "failed").length ?? 0;
  const money = (value: number | null | undefined) =>
    isBalanceHidden ? "••••••" : formatMoney(value, report?.currency);
  return (
    <section aria-label="Saved statement dashboard" className="space-y-6 px-4 py-6 md:px-8">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">Your statement balances</h1>
          <p className="text-muted-foreground mt-1 text-sm">
            Saved bank and investment statements, with their balance history.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" asChild>
            <Link to="/statements?view=all-banks">All bank cash flows</Link>
          </Button>
          <Button asChild>
            <Link to="/statements">Upload statements</Link>
          </Button>
        </div>
      </div>
      {library.isPending && <p role="status">Loading saved statements…</p>}
      {library.isError && (
        <div role="alert" className="border-destructive rounded-xl border p-4">
          <p>{library.error.message}</p>
          <Button variant="outline" className="mt-2" onClick={() => library.refetch()}>
            Retry loading
          </Button>
        </div>
      )}
      {(pending > 0 || failed > 0) && (
        <p role="status" className="text-muted-foreground text-sm">
          {pending > 0 &&
            `${pending} statement(s) processing. This dashboard updates automatically. `}
          {failed > 0 && (
            <Link className="underline" to="/statements">
              {failed} statement(s) need attention
            </Link>
          )}
        </p>
      )}
      {!library.isPending && !library.isError && ready.length === 0 && (
        <div className="rounded-xl border p-10 text-center">
          <h2 className="text-lg font-medium">Bring your balances into view</h2>
          <p className="text-muted-foreground mt-2">
            Upload a statement to see its saved balance history here. Previously uploaded files are
            restored automatically.
          </p>
        </div>
      )}
      {ready.length > 0 && (
        <>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3" aria-label="Saved statements">
            {ready.map((item) => {
              const r = item.report!;
              const timeline = statementBalanceHistory(r);
              const last = timeline[timeline.length - 1];
              return (
                <button
                  key={item.id}
                  type="button"
                  aria-pressed={selected?.id === item.id}
                  onClick={() => {
                    setSelectedId(item.id);
                    setDays(null);
                  }}
                  className={`rounded-xl border p-4 text-left transition-colors ${selected?.id === item.id ? "border-teal-500 bg-teal-500/10" : "hover:bg-muted/50"}`}
                >
                  <span className="text-muted-foreground text-xs uppercase tracking-wide">
                    {r.statementType === "credit-card" ? "Credit card · balance owed" : r.kind === "bank" ? "Bank account" : "Investments"} · {r.currency}
                  </span>
                  <span className="mt-2 block break-words text-sm font-medium">
                    {item.filename}
                  </span>
                  <span className="mt-3 block text-xl font-semibold">
                    {isBalanceHidden ? "••••••" : formatMoney(last?.value, r.currency)}
                  </span>
                  <span className="text-muted-foreground mt-1 block text-xs">
                    {last ? `As of ${last.date}` : "No dated balance available"}
                  </span>
                </button>
              );
            })}
          </div>
          {report && (
            <div className="rounded-xl border p-4 md:p-6">
              <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
                <div>
                  <h2 className="text-lg font-semibold">Balance over time</h2>
                  <p className="text-muted-foreground mt-1 break-words text-sm">
                    {report.filename}
                  </p>
                </div>
                <div className="flex gap-1" aria-label="Balance history range">
                  {ranges.map((range) => (
                    <Button
                      key={range.label}
                      size="sm"
                      variant={days === range.days ? "secondary" : "ghost"}
                      aria-pressed={days === range.days}
                      onClick={() => setDays(range.days)}
                    >
                      {range.label}
                    </Button>
                  ))}
                </div>
              </div>
              <div className="mb-6 grid gap-4 sm:grid-cols-3">
                <div>
                  <p className="text-muted-foreground text-xs">Latest recorded balance</p>
                  <p className="mt-1 font-mono text-xl">{money(latest?.value)}</p>
                </div>
                <div>
                  <p className="text-muted-foreground text-xs">
                    {report.kind === "bank" ? "Opening balance" : "First reported value"}
                  </p>
                  <p className="mt-1 font-mono text-xl">
                    {money(
                      report.kind === "bank" ? report.summary.openingBalance : history[0]?.value,
                    )}
                  </p>
                </div>
                <div>
                  <p className="text-muted-foreground text-xs">Period shown</p>
                  <p className="mt-1 text-sm">
                    {points[0]?.date ?? "—"} → {points[points.length - 1]?.date ?? "—"}
                  </p>
                </div>
              </div>
              <BalanceTimeline
                points={points}
                currency={report.currency}
                hidden={isBalanceHidden}
                bank={report.kind === "bank"}
              />
              {report.warnings.length > 0 && (
                <p className="mt-4 text-sm text-amber-600 dark:text-amber-400">
                  {report.warnings.length} extraction/reconciliation note(s). Review them in
                  Statements.
                </p>
              )}
              <details className="mt-5">
                <summary className="cursor-pointer text-sm">
                  View dated balances ({points.length})
                </summary>
                <div className="mt-3 max-h-64 overflow-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b text-left">
                        <th className="p-2">Date</th>
                        <th className="p-2 text-right">Balance ({report.currency})</th>
                      </tr>
                    </thead>
                    <tbody>
                      {[...points].reverse().map((point) => (
                        <tr key={point.date} className="border-b">
                          <td className="p-2">{point.date}</td>
                          <td className="p-2 text-right font-mono">{money(point.value)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </details>
            </div>
          )}
          <p className="text-muted-foreground text-xs">
            Each card is a saved statement. Overlapping periods and investment snapshots are shown
            separately to avoid counting the same assets twice. Historical balances are current only
            to their stated dates.
          </p>
        </>
      )}
    </section>
  );
}
