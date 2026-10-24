import { isCreditCard, creditCardTotals } from "@/features/statements/credit-card-data";
import { CreditCardSummary } from "@/features/statements/credit-card-summary";
import { AllBankSankey } from "@/features/statements/all-bank-sankey";
import {
  retryStatement,
  readLibrary,
  uploadStatement,
  saveServerReport,
  isServerReport,
  type StatementUpload,
} from "@/features/statements/statement-library";
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { Button, Page, PageContent, PageHeader } from "@wealthfolio/ui";
import { useBalancePrivacy } from "@/hooks/use-balance-privacy";
import { notifyUnauthorized } from "@/lib/auth-token";
import {
  bankFlow,
  formatMoney,
  type StatementReport,
  type StatementTransaction,
} from "@/features/statements/statement-data";
import {
  deleteReport,
  readReports,
  saveReport,
  removeMigratedLocalReport,
} from "@/features/statements/statement-storage";

const PortfolioCharts = lazy(() =>
  import("@/features/statements/statement-charts").then((m) => ({ default: m.PortfolioCharts })),
);
const BankSankey = lazy(() =>
  import("@/features/statements/bank-sankey").then((m) => ({ default: m.BankSankey })),
);
const ALL_BANKS = "all-bank-statements";
const fieldClass =
  "bg-background border-input min-h-10 rounded-md border px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-ring";

async function responseJson(response: Response): Promise<unknown> {
  if (response.status === 401) notifyUnauthorized();
  if (!response.ok) {
    const result = (await response.json().catch(() => null)) as { message?: string } | null;
    throw new Error(result?.message || `Statement request failed (${response.status}).`);
  }
  return response.json();
}

function downloadReport(report: StatementReport) {
  const blob = new Blob([JSON.stringify(report, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = report.filename.replace(/\.[^.]+$/, "") + ".report.json";
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function Metric({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <div className="bg-card rounded-xl border p-5">
      <p className="text-muted-foreground text-sm">{label}</p>
      <p className="mt-2 break-words text-2xl font-semibold tabular-nums tracking-tight">{value}</p>
      {detail && <p className="text-muted-foreground mt-2 text-xs">{detail}</p>}
    </div>
  );
}

export default function StatementsPage() {
  const [reports, setReports] = useState<StatementReport[]>([]);
  const [activeId, setActiveId] = useState(() => {
    try {
      return new URLSearchParams(window.location.search).get("view") === "all-banks"
        ? ALL_BANKS
        : sessionStorage.getItem("wealth-statement-active") || "";
    } catch {
      return "";
    }
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [storageNote, setStorageNote] = useState("");
  const [uploads, setUploads] = useState<StatementUpload[]>([]);
  const [retryingId, setRetryingId] = useState<string | null>(null);
  const [libraryRevision, setLibraryRevision] = useState(0);
  const [libraryError, setLibraryError] = useState("");
  const [uploadNote, setUploadNote] = useState("");
  const [model, setModel] = useState("");
  const [modelBusy, setModelBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const controller = useRef<AbortController | null>(null);
  const active = reports.find((report) => report.id === activeId);
  const bankReports = reports.filter(
    (report) =>
      report.kind === "bank" &&
      (!isServerReport(report.id) ||
        uploads.some((upload) => upload.id === report.id && upload.status === "ready")),
  );

  useEffect(() => {
    try {
      sessionStorage.setItem("wealth-statement-active", activeId);
    } catch {
      /* Session metadata is optional. */
    }
  }, [activeId]);

  useEffect(() => {
    let mounted = true;
    readReports()
      .then(async (saved) => {
        if (mounted) {
          const results = await Promise.allSettled(
            saved.map(async (report) => {
              await saveServerReport(report);
              await removeMigratedLocalReport(report.id);
            }),
          );
          if (!mounted) return;
          if (results.some((result) => result.status === "rejected")) {
            setStorageNote(
              "Some older reports are available only in this browser. Keep it open and retry when the server library is available.",
            );
          }
          setLibraryRevision((revision) => revision + 1);
          setReports((current) => [
            ...current,
            ...saved.filter((r) => !current.some((c) => c.id === r.id)),
          ]);
          setActiveId((id) => id || saved[0]?.id || "");
        }
      })
      .catch(() => {
        if (mounted)
          setStorageNote(
            "Earlier browser reports could not be read. Server-saved statements remain available.",
          );
      });
    return () => {
      mounted = false;
      controller.current?.abort();
    };
  }, []);

  useEffect(() => {
    let mounted = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function refresh() {
      try {
        const saved = await readLibrary();
        if (!mounted) return;
        setUploads(saved);
        setLibraryError("");
        const ready = saved.flatMap((item) => (item.report ? [item.report] : []));
        setReports((current) => [
          ...ready,
          ...current.filter((report) => !isServerReport(report.id)),
        ]);
        setActiveId((id) => id || ready[0]?.id || "");
        if (saved.some((item) => item.status === "queued" || item.status === "processing")) {
          timer = setTimeout(() => void refresh(), 3000);
        }
      } catch (error) {
        if (mounted) {
          setLibraryError(error instanceof Error ? error.message : "Cannot load saved statements.");
          timer = setTimeout(() => void refresh(), 10000);
        }
      }
    }
    void refresh();
    return () => {
      mounted = false;
      if (timer) clearTimeout(timer);
    };
  }, [libraryRevision]);

  async function retrySaved(id: string) {
    setRetryingId(id);
    setLibraryError("");
    try {
      const updated = await retryStatement(id);
      setUploads((current) => current.map((item) => (item.id === id ? updated : item)));
      setLibraryRevision((revision) => revision + 1);
    } catch (error) {
      setLibraryError(error instanceof Error ? error.message : "Could not retry this saved file.");
    } finally {
      setRetryingId(null);
    }
  }

  async function openFiles(files: File[]) {
    if (!files.length) return;
    setError("");
    setBusy(true);
    const request = new AbortController();
    controller.current = request;
    const errors: string[] = [];
    try {
      for (const file of files) {
        if (request.signal.aborted) break;
        setUploadNote("Uploading " + file.name + "…");
        try {
          const item = await uploadStatement(file, request.signal);
          setUploads((current) => [item, ...current.filter((entry) => entry.id !== item.id)]);
          setActiveId(item.id);
          setActiveId(item.id);
          if (item.report) {
            const report = item.report;
            setReports((current) => [report, ...current.filter((entry) => entry.id !== report.id)]);
            setActiveId(report.id);
          }
          setUploadNote(
            item.cached
              ? file.name + " was already loaded. Opened the saved report."
              : file.name + " is saved. Conversion and analysis will continue automatically.",
          );
          setLibraryRevision((revision) => revision + 1);
        } catch (error) {
          if (!request.signal.aborted)
            errors.push(
              file.name + ": " + (error instanceof Error ? error.message : "Upload failed."),
            );
        }
      }
      setError(errors.join(" "));
    } finally {
      setBusy(false);
      controller.current = null;
    }
  }

  async function updateReport(report: StatementReport) {
    setReports((previous) => previous.map((r) => (r.id === report.id ? report : r)));
    try {
      await saveReport(report);
    } catch {
      setStorageNote(
        "Your review changes could not be saved. Please retry or download the report.",
      );
    }
  }
  async function removeActive() {
    if (!active) return;
    try {
      await deleteReport(active.id);
      setUploads((current) => current.filter((entry) => entry.id !== active.id));
      setReports((previous) => previous.filter((r) => r.id !== active.id));
      setActiveId(reports.find((r) => r.id !== active.id)?.id || "");
    } catch {
      setStorageNote("Could not remove this report from the saved library.");
    }
  }
  async function checkModel() {
    setModelBusy(true);
    try {
      const data = (await responseJson(
        await fetch("/api/v1/statements/model", {
          credentials: "same-origin",
          signal: AbortSignal.timeout(25_000),
        }),
      )) as { id: string };
      setModel(`Loaded: ${data.id}`);
    } catch (err) {
      setModel(err instanceof Error ? err.message : "LM Studio unavailable.");
    } finally {
      setModelBusy(false);
    }
  }

  return (
    <Page>
      <PageHeader heading="Statements" />
      <PageContent>
        <div className="mx-auto w-full max-w-7xl space-y-6 pb-10">
          <section className="bg-card rounded-xl border p-5 sm:p-6">
            <div className="flex flex-wrap items-start justify-between gap-5">
              <div className="max-w-2xl">
                <h1 className="text-xl font-semibold">From statement to a clearer picture</h1>
                <p className="text-muted-foreground mt-2 text-sm leading-6">
                  Add an NSDL consolidated statement to explore your holdings, or a bank statement
                  to follow income and spending. Every value links back to the original source.
                </p>
              </div>
              <Button disabled={busy} onClick={() => input.current?.click()}>
                {busy ? "Uploading…" : "Upload statements"}
              </Button>
            </div>
            <section
              className="bg-muted/20 mt-5 rounded-xl border border-dashed p-5"
              aria-label="Statement upload library"
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                if (!busy) void openFiles(Array.from(event.dataTransfer.files));
              }}
            >
              <p className="font-medium">Drop your bank or NSDL statements here</p>
              <p className="text-muted-foreground mt-1 text-sm">
                Upload bank or NSDL statements as PDF, Excel, or CSV. PDFs convert locally to saved
                Markdown with MarkItDown. We reuse saved files, extract the statement data, and
                update your dashboard automatically. Multiple files are supported.
              </p>
              <Button className="mt-3" disabled={busy} onClick={() => input.current?.click()}>
                {busy ? "Uploading…" : "Choose files"}
              </Button>
              {uploadNote && (
                <p className="mt-3 text-sm" role="status">
                  {uploadNote}
                </p>
              )}
              {libraryError && (
                <p className="text-destructive mt-3 text-sm" role="alert">
                  {libraryError}{" "}
                  <button
                    className="underline"
                    onClick={() => setLibraryRevision((value) => value + 1)}
                  >
                    Retry
                  </button>
                </p>
              )}
              {uploads.length > 0 && (
                <ul className="mt-4 divide-y" aria-label="Saved statement files">
                  {uploads.map((item) => (
                    <li
                      key={item.id}
                      className="flex flex-wrap items-center justify-between gap-3 py-3 text-sm"
                    >
                      <div className="min-w-0">
                        <p className="break-all font-medium">{item.filename}</p>
                        <p
                          className={
                            item.status === "failed" ? "text-destructive" : "text-muted-foreground"
                          }
                          role="status"
                        >
                          {item.status === "ready"
                            ? "Saved · ready to view"
                            : item.status === "queued"
                              ? "Queued for local processing"
                              : item.status === "processing"
                                ? "Converting and analyzing locally…"
                                : item.error || "Could not process this statement."}
                        </p>
                      </div>
                      {item.report && (
                        <Button variant="outline" onClick={() => setActiveId(item.id)}>
                          View report
                        </Button>
                      )}
                      {item.status === "failed" && (
                        <button
                          className="rounded-md border px-3 py-2 text-sm"
                          disabled={retryingId === item.id}
                          onClick={() => void retrySaved(item.id)}
                        >
                          {retryingId === item.id ? "Retrying…" : "Retry saved file"}
                        </button>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </section>
            <input
              ref={input}
              type="file"
              accept=".pdf,.xlsx,.xls,.md,.markdown,.txt,.csv,.json"
              multiple
              className="sr-only"
              aria-label="Choose a statement file"
              disabled={busy}
              onChange={(event) => {
                const files = Array.from(event.currentTarget.files || []);
                void openFiles(files);
                event.currentTarget.value = "";
              }}
            />
            <div className="text-muted-foreground mt-5 flex flex-wrap items-center gap-x-5 gap-y-3 border-t pt-4 text-xs">
              <span>PDF · Excel · CSV · text · report JSON · up to 50 MB per file</span>
              <span>Local AI handles unfamiliar bank layouts automatically</span>
              <button
                type="button"
                className="text-primary min-h-8 underline underline-offset-4 disabled:opacity-50"
                disabled={modelBusy}
                onClick={() => void checkModel()}
              >
                {modelBusy ? "Checking…" : "Check loaded model"}
              </button>
            </div>
            <p className="text-muted-foreground mt-2 text-xs">
              Original files and reports are saved on your server. Repeat uploads reuse saved
              results. Processing continues if you leave this page. PDF and Excel are converted
              locally.
            </p>
            {model && (
              <p className="mt-3 text-sm" role="status">
                {model}
              </p>
            )}
            {busy && (
              <p className="text-muted-foreground mt-3 text-sm" role="status">
                Saving files to your server. You can return later while local analysis processes
                unfamiliar layouts.
              </p>
            )}
            {error && (
              <p className="text-destructive mt-3 text-sm" role="alert">
                {error}
              </p>
            )}
            {storageNote && (
              <p className="mt-3 text-sm" role="status">
                {storageNote}
              </p>
            )}
          </section>
          {reports.length > 0 && (
            <div className="flex flex-wrap items-end gap-3">
              <label className="flex min-w-0 flex-1 flex-col gap-1 text-xs">
                Statement view
                <select
                  className={fieldClass}
                  value={activeId}
                  onChange={(e) => setActiveId(e.target.value)}
                >
                  <option value={ALL_BANKS}>All bank statements ({bankReports.length})</option>
                  {reports.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.filename} · {r.period.to || "undated"}
                    </option>
                  ))}
                </select>
              </label>
              <Button
                variant="outline"
                disabled={!active}
                onClick={() => active && downloadReport(active)}
              >
                Download mapped JSON
              </Button>
              <Button variant="ghost" disabled={!active} onClick={() => void removeActive()}>
                Remove report
              </Button>
            </div>
          )}
          {activeId === ALL_BANKS ? (
            <AllBankSankey reports={bankReports} />
          ) : active ? (
            <ReportView key={active.id} report={active} onChange={(r) => void updateReport(r)} />
          ) : (
            <div className="text-muted-foreground rounded-xl border border-dashed px-6 py-16 text-center">
              <h2 className="text-foreground font-semibold">Your statements, explained visually</h2>
              <p className="mx-auto mt-2 max-w-lg text-sm leading-6">
                Choose a file to see portfolio history, allocations and individual stock values, or
                a bank income and expense Sankey. No transactions are posted to your portfolio
                during analysis.
              </p>
            </div>
          )}
        </div>
      </PageContent>
    </Page>
  );
}

function ReportView({
  report,
  onChange,
}: {
  report: StatementReport;
  onChange: (report: StatementReport) => void;
}) {
  const { isBalanceHidden } = useBalancePrivacy();
  const [view, setView] = useState<"overview" | "mapping">("overview");
  const [focusLines, setFocusLines] = useState<number[]>([]);
  if (isBalanceHidden)
    return (
      <p className="text-muted-foreground rounded-xl border p-6">
        Show balances to view statement charts, values and source data.
      </p>
    );
  function inspect(lines: number[]) {
    setFocusLines(lines);
    setView("mapping");
  }
  return (
    <>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="font-medium">
            {report.kind === "investment"
              ? "Investment statement"
              : report.statementType === "credit-card"
                ? "Credit card statement"
                : "Bank statement"}{" "}
            · {report.period.from || "Unknown start"} to {report.period.to || "Unknown end"}
          </p>
          <p className="text-muted-foreground mt-1 text-xs">
            {report.extraction.method === "deterministic"
              ? "Mapped locally without AI"
              : `Extracted with ${report.extraction.model}`}{" "}
            · Snapshot from this statement
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant={view === "overview" ? "secondary" : "ghost"}
            aria-pressed={view === "overview"}
            onClick={() => setView("overview")}
          >
            Charts & data
          </Button>
          <Button
            variant={view === "mapping" ? "secondary" : "ghost"}
            aria-pressed={view === "mapping"}
            onClick={() => setView("mapping")}
          >
            Source mapping
          </Button>
        </div>
      </div>
      {report.warnings.length > 0 && (
        <div className="bg-muted/40 rounded-lg border p-4 text-sm">
          <h2 className="font-medium">Review notes</h2>
          <ul className="text-muted-foreground mt-2 list-disc space-y-1 pl-5">
            {report.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      )}
      {view === "mapping" ? (
        <SourceMapping report={report} focusLines={focusLines} />
      ) : report.kind === "investment" ? (
        <InvestmentView report={report} inspect={inspect} />
      ) : (
        <BankView report={report} inspect={inspect} onChange={onChange} />
      )}
    </>
  );
}

function InvestmentView({
  report,
  inspect,
}: {
  report: StatementReport;
  inspect: (lines: number[]) => void;
}) {
  const [account, setAccount] = useState("all");
  const [search, setSearch] = useState("");
  const holdings = useMemo(
    () =>
      report.holdings.filter(
        (h) =>
          (account === "all" || h.accountId === account) &&
          `${h.name} ${h.symbol} ${h.isin}`.toLowerCase().includes(search.toLowerCase()),
      ),
    [report.holdings, account, search],
  );
  const value = holdings.reduce((total, h) => total + Math.round(h.value * 100), 0) / 100;
  const accounts = report.accounts.filter((a) => !a.id.startsWith("summary-"));
  return (
    <>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Metric
          label="Reported portfolio value"
          value={formatMoney(report.summary.reportedValue, report.currency)}
          detail="Whole portfolio at statement close"
        />
        <Metric
          label="Mapped holdings value"
          value={formatMoney(report.summary.parsedValue, report.currency)}
          detail={`${report.holdings.length} holding rows · difference ${formatMoney(report.summary.difference, report.currency)}`}
        />
        <Metric
          label="Distinct securities / schemes"
          value={String(report.summary.securityCount ?? "—")}
          detail="The same ISIN may occur in several accounts"
        />
        <Metric
          label="Known unrealized gain"
          value={formatMoney(report.summary.knownUnrealizedGain, report.currency)}
          detail="Only holdings with reported cost data; not whole-portfolio profit"
        />
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1 text-xs">
          Account
          <select
            className={fieldClass}
            value={account}
            onChange={(e) => setAccount(e.target.value)}
          >
            <option value="all">All detailed accounts</option>
            {accounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex min-w-52 flex-1 flex-col gap-1 text-xs">
          Find a holding
          <input
            type="search"
            className={fieldClass}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Company, symbol or ISIN"
          />
        </label>
        <p className="text-muted-foreground py-2 text-sm">
          Showing {holdings.length} rows · {formatMoney(value, report.currency)}
        </p>
      </div>
      <Suspense fallback={<p role="status">Loading portfolio charts…</p>}>
        <PortfolioCharts report={report} holdings={holdings} />
      </Suspense>
      <section className="bg-card overflow-hidden rounded-xl border">
        <h2 className="p-5 font-semibold">Holdings · descending statement value</h2>
        <div className="max-h-[600px] overflow-auto">
          <table className="w-full min-w-[950px] text-left text-sm">
            <caption className="sr-only">
              Holdings matching selected account and search filters
            </caption>
            <thead className="bg-muted sticky top-0">
              <tr>
                {[
                  "Security / scheme",
                  "Account",
                  "Units / shares",
                  "Price / NAV",
                  "Value",
                  "Known cost",
                  "Known gain",
                  "Source",
                ].map((label) => (
                  <th scope="col" key={label} className="px-4 py-3 font-medium">
                    {label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {[...holdings]
                .sort((a, b) => b.value - a.value)
                .map((h) => (
                  <tr key={h.id} className="border-t">
                    <td className="max-w-72 px-4 py-3">
                      <p className="font-medium">{h.name}</p>
                      <p className="text-muted-foreground text-xs">
                        {h.symbol || h.isin || h.assetClass}
                      </p>
                      {h.pledgedQuantity != null && h.pledgedQuantity > 0 && (
                        <p className="text-muted-foreground text-xs">
                          {h.pledgedQuantity.toLocaleString("en-IN")} pledged (included in units)
                        </p>
                      )}
                    </td>
                    <td className="max-w-40 px-4 py-3 text-xs">{h.accountName}</td>
                    <td className="px-4 py-3 tabular-nums">
                      {h.quantity.toLocaleString("en-IN", { maximumFractionDigits: 4 })}
                    </td>
                    <td className="px-4 py-3 tabular-nums">{formatMoney(h.price, h.currency)}</td>
                    <td className="px-4 py-3 font-medium tabular-nums">
                      {formatMoney(h.value, h.currency)}
                    </td>
                    <td className="px-4 py-3 tabular-nums">{formatMoney(h.cost, h.currency)}</td>
                    <td className="px-4 py-3 tabular-nums">
                      {formatMoney(h.unrealizedGain, h.currency)}
                    </td>
                    <td className="px-4 py-3">
                      <button
                        type="button"
                        className="text-primary min-h-9 underline"
                        onClick={() => inspect(h.sourceLines)}
                      >
                        L{h.sourceLines[0]}
                      </button>
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      </section>
      <section className="bg-card rounded-xl border p-5">
        <h2 className="font-semibold">Account reconciliation</h2>
        <div className="mt-3 overflow-auto">
          <table className="w-full min-w-[600px] text-left text-sm">
            <caption className="sr-only">
              Detailed holding totals versus reported account totals
            </caption>
            <thead>
              <tr>
                <th scope="col">Account</th>
                <th scope="col">Mapped value</th>
                <th scope="col">Reported value</th>
                <th scope="col">Difference</th>
              </tr>
            </thead>
            <tbody>
              {accounts.map((a) => (
                <tr key={a.id} className="border-t">
                  <td className="py-3 pr-3">
                    {a.name}
                    <span className="text-muted-foreground block text-xs">{a.id}</span>
                  </td>
                  <td>{formatMoney(a.value, report.currency)}</td>
                  <td>{formatMoney(a.reportedValue, report.currency)}</td>
                  <td>
                    {a.reportedValue == null
                      ? "—"
                      : formatMoney(a.value - a.reportedValue, report.currency)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
      {report.transactions.length > 0 && (
        <details className="rounded-xl border p-5">
          <summary className="cursor-pointer font-medium">
            Depository movements & pension fees · {report.transactions.length} rows
          </summary>
          <p className="text-muted-foreground my-3 text-sm">
            Depository debits/credits are security units. They do not provide trade cash amounts or
            realized profit.
          </p>
          <div className="max-h-80 overflow-auto">
            <table className="w-full min-w-[600px] text-left text-sm">
              <thead>
                <tr>
                  <th scope="col">Date</th>
                  <th scope="col">ISIN / category</th>
                  <th scope="col">Direction</th>
                  <th scope="col">Quantity</th>
                  <th scope="col">Cash amount</th>
                  <th scope="col">Source</th>
                </tr>
              </thead>
              <tbody>
                {report.transactions.map((t, i) => (
                  <tr key={i} className="border-t">
                    <td className="py-2">{t.date}</td>
                    <td>{t.isin || t.category}</td>
                    <td>{t.direction}</td>
                    <td>{t.quantity}</td>
                    <td>
                      {t.unit === "cash"
                        ? formatMoney(t.amount, t.currency || report.currency)
                        : "Units only"}
                    </td>
                    <td>
                      <button
                        className="text-primary min-h-9 underline"
                        onClick={() => inspect(t.sourceLines)}
                      >
                        L{t.sourceLines[0]}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </details>
      )}
    </>
  );
}

function BankView({
  report,
  inspect,
  onChange,
}: {
  report: StatementReport;
  inspect: (lines: number[]) => void;
  onChange: (report: StatementReport) => void;
}) {
  const currencies = [
    ...new Set(report.transactions.map((t) => t.currency).filter((c): c is string => c != null)),
  ];
  const card = isCreditCard(report);
  const [currency, setCurrency] = useState(currencies[0] || report.currency);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [page, setPage] = useState(0);
  const transactions = useMemo(
    () =>
      report.transactions.filter(
        (t) => t.currency === currency && (!from || t.date >= from) && (!to || t.date <= to),
      ),
    [report.transactions, currency, from, to],
  );
  const flow = useMemo(() => bankFlow(transactions, currency), [transactions, currency]);
  function patch(target: StatementTransaction, changes: Partial<StatementTransaction>) {
    const updated = {
      ...report,
      transactions: report.transactions.map((t) => (t === target ? { ...t, ...changes } : t)),
    };
    // Recompute summary aggregates after a user reviews classifications.
    const totals = bankFlow(updated.transactions, report.currency);
    if (currencies.length === 1) {
      updated.summary = { ...updated.summary, income: totals.income, expenses: totals.expenses };
    }
    if (card && currencies.length === 1) {
      const cardTotals = creditCardTotals(updated.transactions, report.currency);
      updated.summary = {
        ...updated.summary,
        income: 0,
        expenses: cardTotals.expenses,
        grossCharges: cardTotals.gross,
        expenseRefunds: cardTotals.refunds,
        cardPayments: cardTotals.payments,
        unclassifiedCredits: cardTotals.otherCredits,
        paymentDifference: cardTotals.difference,
      };
    }
    onChange(updated);
  }
  return (
    <>
      <div className="flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1 text-xs">
          Currency
          <select
            className={fieldClass}
            value={currency}
            onChange={(e) => {
              setCurrency(e.target.value);
              setPage(0);
            }}
          >
            {currencies.map((code) => (
              <option key={code}>{code}</option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs">
          From
          <input
            className={fieldClass}
            type="date"
            value={from}
            onChange={(e) => {
              setFrom(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs">
          Through
          <input
            className={fieldClass}
            type="date"
            value={to}
            min={from || undefined}
            onChange={(e) => {
              setTo(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <Button
          variant="ghost"
          onClick={() => {
            setFrom("");
            setTo("");
            setPage(0);
          }}
        >
          All dates
        </Button>
        <span className="text-muted-foreground py-2 text-sm">{flow.count} transactions</span>
      </div>
      {from && to && from > to && (
        <p role="alert" className="text-destructive text-sm">
          The start date must be on or before the end date.
        </p>
      )}
      {card && <CreditCardSummary transactions={transactions} currency={currency} />}
      {!card && (
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          <Metric
            label="Income"
            value={formatMoney(flow.income, currency)}
            detail="Excludes incoming transfers and balance funding"
          />
          <Metric
            label="Expenses"
            value={formatMoney(flow.expenses, currency)}
            detail="Excludes outgoing transfers"
          />
          <Metric
            label="Net cash flow"
            value={formatMoney(flow.net, currency)}
            detail="All credits minus all debits, including transfers"
          />
          <Metric
            label="Statement balance difference"
            value={
              currencies.length === 1
                ? formatMoney(report.summary.balanceDifference, currency)
                : "Per-currency review"
            }
            detail="Whole statement: opening + credits − debits − closing"
          />
        </div>
      )}
      <Suspense fallback={<p role="status">Loading cash flow chart…</p>}>
        <BankSankey transactions={transactions} currency={currency} creditCard={card} />
      </Suspense>
      <section className="bg-card overflow-hidden rounded-xl border">
        <div className="p-5">
          <h2 className="font-semibold">Review transaction categories</h2>
          <p className="text-muted-foreground mt-1 text-sm">
            Adjust a category or mark an internal transfer. The Sankey updates immediately.
          </p>
        </div>
        <div className="overflow-auto">
          <table className="w-full min-w-[850px] text-left text-sm">
            <caption className="sr-only">
              Bank transactions with editable categories and cash flow classification
            </caption>
            <thead className="bg-muted">
              <tr>
                {["Date", "Description", "Amount", "Flow", "Category", "Source"].map((h) => (
                  <th scope="col" key={h} className="px-4 py-3">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {transactions.slice(page * 50, page * 50 + 50).map((t) => (
                <tr key={t.id || t.sourceLines.join("-")} className="border-t">
                  <td className="px-4 py-3">{t.date}</td>
                  <td className="max-w-80 px-4 py-3">{t.description}</td>
                  <td className="whitespace-nowrap px-4 py-3 tabular-nums">
                    {t.direction === "in" ? "+" : "−"}
                    {formatMoney(t.amount, currency)}
                  </td>
                  <td className="px-4 py-3">
                    {card && t.direction === "in" ? (
                      <select
                        className={fieldClass}
                        aria-label={`Credit classification for ${t.description}`}
                        value={t.cardCreditKind || "unclassified"}
                        onChange={(e) => {
                          const kind = e.target.value as "payment" | "refund" | "unclassified";
                          patch(t, {
                            cardCreditKind: kind,
                            expenseOffset: kind === "refund",
                            flowType: kind === "refund" ? "expense" : "transfer",
                            category:
                              kind === "payment"
                                ? "Card repayments"
                                : kind === "refund"
                                  ? "Other expenses"
                                  : "Other card credits",
                          });
                        }}
                      >
                        <option value="payment">Card payment</option>
                        <option value="refund">Expense refund / fuel credit</option>
                        <option value="unclassified">Other credit · review</option>
                      </select>
                    ) : (
                      <select
                        className={fieldClass}
                        aria-label={`Flow for ${t.description}`}
                        value={t.flowType}
                        onChange={(e) =>
                          patch(t, {
                            flowType: e.target.value as StatementTransaction["flowType"],
                            ...(card
                              ? {
                                  cardCreditKind:
                                    e.target.value === "transfer" && t.cardCreditKind === "payment"
                                      ? ("payment" as const)
                                      : undefined,
                                  expenseOffset: undefined,
                                }
                              : {}),
                          })
                        }
                      >
                        <option value={t.direction === "in" ? "income" : "expense"}>
                          {t.direction === "in" ? "Income" : "Expense"}
                        </option>
                        <option value="transfer">Transfer</option>
                      </select>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <input
                      key={`${t.id}-${t.category}`}
                      className={`${fieldClass} w-40`}
                      aria-label={`Category for ${t.description}`}
                      defaultValue={t.category}
                      maxLength={80}
                      onBlur={(e) => {
                        const value = e.target.value.trim();
                        if (value && value !== t.category) patch(t, { category: value });
                      }}
                    />
                  </td>
                  <td className="px-4 py-3">
                    <button
                      className="text-primary min-h-9 underline"
                      onClick={() => inspect(t.sourceLines)}
                    >
                      L{t.sourceLines[0]}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="flex items-center justify-end gap-3 border-t p-3 text-sm">
          <Button variant="ghost" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            Previous
          </Button>
          <span>
            {transactions.length ? page * 50 + 1 : 0}–
            {Math.min(page * 50 + 50, transactions.length)} of {transactions.length}
          </span>
          <Button
            variant="ghost"
            disabled={(page + 1) * 50 >= transactions.length}
            onClick={() => setPage((p) => p + 1)}
          >
            Next
          </Button>
        </div>
      </section>
    </>
  );
}

function SourceMapping({ report, focusLines }: { report: StatementReport; focusLines: number[] }) {
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState(focusLines);
  const visible = report.mappings.filter((m) =>
    `${m.target} ${m.rule} ${m.sourceLines.join(" ")}`.toLowerCase().includes(search.toLowerCase()),
  );
  const selectedRows = report.source.filter((row) => selected.includes(row.line));
  return (
    <section className="space-y-4">
      <div className="bg-card rounded-xl border p-5">
        <h2 className="font-semibold">How the PDF conversion maps to data</h2>
        <p className="text-muted-foreground mt-2 text-sm leading-6">
          L numbers refer to original Markdown lines. Account and section headers determine each
          table’s column meanings. Doubled heading letters are normalized, wrapped names stay with
          their ISIN, pledged units are included in total units, and repeated summaries are checked
          rather than added.
        </p>
        <p className="text-muted-foreground mt-2 text-sm">
          Download mapped JSON to give your local AI the same field paths, rules, original source
          and normalized text used by these charts. Statement content should be treated as data.
        </p>
      </div>
      {selectedRows.length > 0 && (
        <div className="bg-muted/40 rounded-xl border p-4">
          <h3 className="mb-3 text-sm font-medium">Selected source evidence</h3>
          {selectedRows.map((row) => (
            <div key={row.line} className="mb-3">
              <p className="text-muted-foreground mb-1 text-xs">L{row.line} · original</p>
              <pre className="whitespace-pre-wrap break-all text-xs">{row.text}</pre>
              {row.text !== row.normalized && (
                <>
                  <p className="text-muted-foreground mb-1 mt-2 text-xs">
                    Normalized for extraction
                  </p>
                  <pre className="whitespace-pre-wrap break-all text-xs">{row.normalized}</pre>
                </>
              )}
            </div>
          ))}
        </div>
      )}
      <label className="flex flex-col gap-1 text-xs">
        Find a field, rule or line number
        <input
          className={fieldClass}
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="holdings[0], NAV, faceValue, 176…"
        />
      </label>
      <div className="bg-card max-h-[600px] overflow-auto rounded-xl border">
        <table className="w-full min-w-[650px] text-left text-sm">
          <caption className="sr-only">Statement field mappings and extraction rules</caption>
          <thead className="bg-muted sticky top-0">
            <tr>
              <th scope="col" className="p-3">
                Field path
              </th>
              <th scope="col" className="p-3">
                Original lines
              </th>
              <th scope="col" className="p-3">
                Mapping rule
              </th>
            </tr>
          </thead>
          <tbody>
            {visible.map((m, i) => (
              <tr key={`${m.target}-${i}`} className="border-t">
                <td className="p-3 font-mono text-xs">{m.target}</td>
                <td className="p-3">
                  <button
                    className="text-primary min-h-9 underline"
                    onClick={() => setSelected(m.sourceLines)}
                  >
                    {m.sourceLines.map((line) => `L${line}`).join(", ")}
                  </button>
                </td>
                <td className="max-w-xl p-3 text-xs leading-5">{m.rule}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
