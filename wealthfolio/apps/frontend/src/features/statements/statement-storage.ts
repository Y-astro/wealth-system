import { isServerReport, saveServerReport, archiveServerReport } from "./statement-library";
import { statementSchema, type StatementReport } from "./statement-data";

// Legacy browser storage is retained only until each report is safely migrated to the server.
function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("wealth-statement-reports", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("reports");
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(new Error("Local statement storage is unavailable."));
  });
}

async function accessStore<T>(
  mode: IDBTransactionMode,
  action: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDatabase();
  try {
    return await new Promise<T>((resolve, reject) => {
      const transaction = db.transaction("reports", mode);
      const request = action(transaction.objectStore("reports"));
      transaction.oncomplete = () => resolve(request.result);
      transaction.onerror = () => reject(new Error("Could not update local statement storage."));
      transaction.onabort = () => reject(new Error("Local statement storage was interrupted."));
    });
  } finally {
    db.close();
  }
}

export async function readReports(): Promise<StatementReport[]> {
  const values = await accessStore("readonly", (store) => store.getAll());
  return values.flatMap((value) => {
    const parsed = statementSchema.safeParse(value);
    return parsed.success ? [parsed.data] : [];
  });
}
export async function saveReport(report: StatementReport): Promise<void> {
  if (isServerReport(report.id)) return saveServerReport(report);
  await accessStore("readwrite", (store) => store.put(report, report.id));
}
export async function deleteReport(id: string): Promise<void> {
  if (isServerReport(id)) return archiveServerReport(id);
  await accessStore("readwrite", (store) => store.delete(id));
}

export async function removeMigratedLocalReport(id: string): Promise<void> {
  await accessStore("readwrite", (store) => store.delete(id));
}
