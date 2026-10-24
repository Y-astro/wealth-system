import { notifyUnauthorized } from "@/lib/auth-token";
import { statementSchema, type StatementReport } from "./statement-data";

export interface StatementUpload {
  id: string;
  filename: string;
  status: "queued" | "processing" | "ready" | "failed";
  error: string | null;
  report: StatementReport | null;
  cached?: boolean;
}
const knownUploads = new Set<string>();
export function isServerReport(id: string): boolean {
  return knownUploads.has(id);
}

async function responseJson(response: Response): Promise<unknown> {
  if (response.status === 401) notifyUnauthorized();
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { message?: string } | null;
    throw new Error(body?.message || "The saved statement library is unavailable. Please retry.");
  }
  return response.json();
}
export async function readLibrary(): Promise<StatementUpload[]> {
  const data = await responseJson(
    await fetch("/api/v1/statements/library", { credentials: "same-origin" }),
  );
  if (!Array.isArray(data)) throw new Error("Invalid statement library response.");
  return data.map((item: StatementUpload) => {
    knownUploads.add(item.id);
    if (item.report) {
      const parsed = statementSchema.safeParse(item.report);
      if (!parsed.success)
        return {
          ...item,
          status: "failed" as const,
          report: null,
          error: "Saved report failed validation. Upload the original statement to try again.",
        };
      item.report = parsed.data;
    }
    return item;
  });
}
export async function uploadStatement(file: File, signal?: AbortSignal): Promise<StatementUpload> {
  if (!file.size || file.size > 50_000_000)
    throw new Error("Choose a nonempty statement up to 50 MB.");
  const form = new FormData();
  form.append("file", file);
  const item = (await responseJson(
    await fetch("/api/v1/statements/library", {
      method: "POST",
      credentials: "same-origin",
      body: form,
      signal,
    }),
  )) as StatementUpload;
  knownUploads.add(item.id);
  if (item.report) item.report = statementSchema.parse(item.report);
  return item;
}
export async function saveServerReport(report: StatementReport): Promise<void> {
  await responseJson(
    await fetch("/api/v1/statements/library/" + encodeURIComponent(report.id), {
      method: "PUT",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report),
    }),
  );
  knownUploads.add(report.id);
}
export async function archiveServerReport(id: string): Promise<void> {
  await responseJson(
    await fetch("/api/v1/statements/library/" + encodeURIComponent(id), {
      method: "DELETE",
      credentials: "same-origin",
    }),
  );
}

export async function retryStatement(id: string): Promise<StatementUpload> {
  return (await responseJson(
    await fetch("/api/v1/statements/library/" + encodeURIComponent(id) + "/retry", {
      method: "POST",
      credentials: "same-origin",
    }),
  )) as StatementUpload;
}
