import { useEffect, useMemo, useState } from "react";
import { useBalancePrivacy } from "@/hooks/use-balance-privacy";
import { BankSankey } from "./bank-sankey";
import { bankCurrencies, combineBankStatements } from "./combined-bank-data";
import { bankFlow, formatMoney, type StatementReport } from "./statement-data";

const GROUPS_KEY = "bank-sankey-account-groups";
const field = "bg-background min-h-10 rounded-md border px-3 py-2 text-sm";
export function AllBankSankey({ reports }: { reports: StatementReport[] }) {
  const { isBalanceHidden } = useBalancePrivacy();
  const [excludedIds, setExcludedIds] = useState<string[]>([]);
  const [selectedCurrency, setCurrency] = useState("INR");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [accountGroups, setGroups] = useState<Record<string, string>>(() => {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(GROUPS_KEY) ?? "{}");
      if (saved && typeof saved === "object" && !Array.isArray(saved))
        return Object.fromEntries(
          Object.entries(saved).filter(([, value]) => typeof value === "string"),
        ) as Record<string, string>;
    } catch {
      /* Account labels can be entered again if browser storage is unavailable. */
    }
    return {};
  });
  useEffect(() => {
    try {
      localStorage.setItem(GROUPS_KEY, JSON.stringify(accountGroups));
    } catch {
      /* Chart still works without browser storage. */
    }
  }, [accountGroups]);
  const banks = useMemo(
    () => [
      ...new Map(
        reports.filter((report) => report.kind === "bank").map((report) => [report.id, report]),
      ).values(),
    ],
    [reports],
  );
  const currencies = useMemo(() => bankCurrencies(banks), [banks]);
  const currency = currencies.includes(selectedCurrency)
    ? selectedCurrency
    : (currencies[0] ?? "INR");
  const combined = useMemo(
    () => combineBankStatements(banks, { currency, from, to, excludedIds, accountGroups }),
    [banks, currency, from, to, excludedIds, accountGroups],
  );
  const totals = useMemo(
    () => bankFlow(combined.transactions, currency),
    [combined.transactions, currency],
  );
  const invalidRange = !!from && !!to && from > to;
  const bankWarnings = banks
    .filter((report) => !excludedIds.includes(report.id))
    .reduce((sum, report) => sum + report.warnings.length, 0);
  if (isBalanceHidden)
    return (
      <section aria-label="All bank statements cash flow" className="rounded-xl border p-6">
        <h2 className="font-semibold">All bank statements</h2>
        <p className="text-muted-foreground mt-2">
          Show balances to view the combined bank Sankey.
        </p>
      </section>
    );
  return (
    <section aria-label="All bank statements cash flow" className="space-y-5">
      <div>
        <h2 className="text-xl font-semibold">All bank statements</h2>
        <p className="text-muted-foreground mt-2 text-sm">
          Combined bank income, expenses and transfers. NSDL stock values, investment statements,
          opening balances and closing balances are excluded from cash-flow amounts.
        </p>
      </div>
      {banks.length === 0 ? (
        <p className="rounded-xl border p-6">
          Upload a bank statement to build this Sankey. NSDL statements appear in their separate
          investment reports.
        </p>
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3 rounded-xl border p-4">
            <label className="flex flex-col gap-1 text-xs">
              Currency
              <select
                aria-label="Combined bank currency"
                className={field}
                value={currency}
                onChange={(event) => setCurrency(event.target.value)}
              >
                {(currencies.length ? currencies : [currency]).map((code) => (
                  <option key={code}>{code}</option>
                ))}
              </select>
            </label>
            <label className="flex flex-col gap-1 text-xs">
              From
              <input
                aria-label="Combined bank from date"
                type="date"
                className={field}
                value={from}
                onChange={(event) => setFrom(event.target.value)}
              />
            </label>
            <label className="flex flex-col gap-1 text-xs">
              To
              <input
                aria-label="Combined bank to date"
                type="date"
                className={field}
                value={to}
                onChange={(event) => setTo(event.target.value)}
              />
            </label>
            <button
              type="button"
              className={field + " hover:bg-muted"}
              onClick={() => {
                setFrom("");
                setTo("");
                setExcludedIds([]);
              }}
            >
              All files and dates
            </button>
            <p className="text-muted-foreground text-xs">Currencies are shown separately.</p>
          </div>
          <details className="rounded-xl border p-4">
            <summary className="cursor-pointer text-sm font-medium">
              Included bank files ({combined.statements} of {banks.length}) · account groups
            </summary>
            <p className="text-muted-foreground mt-3 text-xs">
              For overlapping statements from the same account, enter the same account-group label
              below. Matching date, description, direction, amount and running balance are counted
              once within that group. Leave different accounts in different groups. Repeated
              payments within a file are preserved. Group labels are saved in this browser.
            </p>
            <div className="mt-3 space-y-3">
              {banks.map((report) => (
                <div
                  key={report.id}
                  className="flex flex-wrap items-center justify-between gap-3 border-t pt-3"
                >
                  <label className="flex min-w-0 flex-1 items-center gap-3 text-sm">
                    <input
                      type="checkbox"
                      aria-label={"Include " + report.filename}
                      checked={!excludedIds.includes(report.id)}
                      onChange={(event) =>
                        setExcludedIds((previous) =>
                          event.target.checked
                            ? previous.filter((id) => id !== report.id)
                            : [...previous, report.id],
                        )
                      }
                    />
                    <span className="break-words">
                      {report.filename}
                      <span className="text-muted-foreground mt-1 block text-xs">
                        {report.period.from ?? "Unknown start"} →{" "}
                        {report.period.to ?? "Unknown end"}
                      </span>
                    </span>
                  </label>
                  <input
                    aria-label={"Account group for " + report.filename}
                    placeholder="Account group (optional)"
                    className={field + " w-64 max-w-full"}
                    value={accountGroups[report.id] ?? ""}
                    onChange={(event) =>
                      setGroups((previous) => ({ ...previous, [report.id]: event.target.value }))
                    }
                  />
                </div>
              ))}
            </div>
          </details>
          {invalidRange ? (
            <p role="alert" className="text-destructive">
              The start date must be on or before the end date.
            </p>
          ) : (
            <>
              <div className="grid gap-3 sm:grid-cols-3">
                {[
                  ["Income", totals.income],
                  ["Expenses", totals.expenses],
                  ["Net cash movement", totals.net],
                ].map(([label, value]) => (
                  <div key={label} className="rounded-xl border p-4">
                    <p className="text-muted-foreground text-xs">{label}</p>
                    <p className="mt-2 text-xl font-semibold">
                      {formatMoney(value as number, currency)}
                    </p>
                  </div>
                ))}
              </div>
              <p role="status" className="text-muted-foreground text-sm">
                {combined.transactions.length} bank cash transactions · {combined.statements}{" "}
                selected statements · {currency}
                {combined.duplicates > 0
                  ? ` · ${combined.duplicates} overlapping transaction(s) counted once`
                  : ""}
                . Transfers use their reviewed classification and stay separate from income and
                spending.
              </p>
              {bankWarnings > 0 && (
                <p className="text-sm text-amber-600 dark:text-amber-400">
                  Selected bank reports contain {bankWarnings} extraction/review note(s). Review
                  those notes in the individual reports.
                </p>
              )}
              <BankSankey
                key={JSON.stringify([currency, from, to, excludedIds, accountGroups])}
                transactions={combined.transactions}
                currency={currency}
              />
            </>
          )}
        </>
      )}
    </section>
  );
}
