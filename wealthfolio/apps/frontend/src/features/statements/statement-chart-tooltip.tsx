import { formatMoney } from "./statement-data";

export function StatementChartTooltip({
  active,
  payload,
  label,
  currency,
  total,
}: {
  active?: boolean;
  payload?: readonly { value?: unknown; name?: unknown; payload?: { name?: string } }[];
  label?: unknown;
  currency: string;
  total?: number;
}) {
  if (!active) return null;
  const item = payload?.find(
    (entry) => entry.value != null && Number.isFinite(Number(entry.value)),
  );
  if (!item) return null;
  const value = Number(item.value);
  const name =
    label != null && String(label).trim()
      ? String(label)
      : String(item.payload?.name ?? item.name ?? "Statement value");
  // Explicit colors keep text readable under both dashboard themes, independent
  // of Recharts' default tooltip list styles and inherited SVG text colors.
  return (
    <div
      role="status"
      data-testid="statement-chart-tooltip"
      style={{
        backgroundColor: "#111827",
        color: "#f8fafc",
        border: "1px solid #64748b",
        borderRadius: 8,
        padding: "10px 14px",
        maxWidth: 320,
        whiteSpace: "normal",
        boxShadow: "0 4px 16px #0003",
        fontSize: 13,
      }}
    >
      <div style={{ color: "#f8fafc", fontWeight: 600 }}>{name}</div>
      <div style={{ color: "#f8fafc", fontVariantNumeric: "tabular-nums", marginTop: 4 }}>
        {formatMoney(value, currency)}
      </div>
      {total != null && total > 0 && (
        <div style={{ color: "#cbd5e1", marginTop: 4 }}>
          {((value / total) * 100).toFixed(1)}% of selected holdings
        </div>
      )}
    </div>
  );
}
