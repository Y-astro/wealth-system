import { QueryClientProvider, type QueryClient, useQuery, useMutation } from "@tanstack/react-query";
import { type AddonContext, type Account, type ActivityImport, QueryKeys } from "@wealthfolio/addon-sdk";
import {
  Button,
  Icons,
  Page,
  PageContent,
  PageHeader,
} from "@wealthfolio/ui";
import React, { useEffect, useState, useRef } from "react";

function StatementImporter({ ctx }: { ctx: AddonContext }) {
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [file, setFile] = useState<File | null>(null);
  const [dragActive, setDragActive] = useState<boolean>(false);
  const [step, setStep] = useState<"upload" | "loading" | "review" | "success">("upload");
  const [loadingPhase, setLoadingPhase] = useState<string>("Initializing...");
  
  // Review grid states
  const [drafts, setDrafts] = useState<ActivityImport[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [errorMsg, setErrorMsg] = useState<string>("");

  const fileInputRef = useRef<HTMLInputElement>(null);

  // Fetch accounts
  const { data: accounts = [], isLoading: isLoadingAccounts } = useQuery<Account[]>({
    queryKey: [QueryKeys.ACCOUNTS],
    queryFn: async () => {
      if (!ctx.api) throw new Error("API context is required");
      return await ctx.api.accounts.getAll() || [];
    },
    enabled: !!ctx.api,
  });

  // Set default account
  useEffect(() => {
    if (accounts.length > 0 && !selectedAccountId) {
      setSelectedAccountId(accounts[0].id);
    }
  }, [accounts, selectedAccountId]);

  // Handle Drag & Drop
  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true);
    } else if (e.type === "dragleave") {
      setDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      setFile(e.dataTransfer.files[0]);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setFile(e.target.files[0]);
    }
  };

  // Upload mutation
  const uploadMutation = useMutation<ActivityImport[], Error, { file: File; accountId: string }>({
    mutationFn: async ({ file, accountId }) => {
      setStep("loading");
      setLoadingPhase("Extracting text from bank statement...");

      const formData = new FormData();
      formData.append("file", file);
      formData.append("accountId", accountId);

      // We call our newly exposed Axum endpoint directly
      const response = await fetch("/api/v1/activities/import/statement", {
        method: "POST",
        body: formData,
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || "Failed to process statement file.");
      }

      setLoadingPhase("Analyzing extracted text and mapping fields...");
      const parsedDrafts = (await response.json()) as ActivityImport[];

      setLoadingPhase("Performing duplicate check validation...");
      if (ctx.api && ctx.api.activities && ctx.api.activities.checkImport) {
        try {
          const validated = await ctx.api.activities.checkImport(accountId, parsedDrafts);
          return validated || parsedDrafts;
        } catch (e) {
          ctx.api.logger.error("Check import validation failed: " + (e as Error).message);
          return parsedDrafts;
        }
      }
      return parsedDrafts;
    },
    onSuccess: (data) => {
      // Set line_number if not set to unique index identifier
      const enriched = data.map((draft, idx) => ({
        ...draft,
        line_number: draft.line_number ?? idx + 1,
      }));
      setDrafts(enriched);
      
      // Auto-select all non-duplicates
      const initialSelected = new Set<number>();
      enriched.forEach((draft, idx) => {
        if (!draft.duplicate_of_id) {
          initialSelected.add(idx);
        }
      });
      setSelectedIds(initialSelected);
      setStep("review");
    },
    onError: (err) => {
      setErrorMsg(err.message || "An error occurred while uploading file.");
      setStep("upload");
    },
  });

  const handleUploadSubmit = () => {
    if (!file) {
      setErrorMsg("Please select a bank statement file to upload.");
      return;
    }
    setErrorMsg("");
    uploadMutation.mutate({ file, accountId: selectedAccountId });
  };

  // Add manually a new draft row
  const handleAddRow = () => {
    const newIdx = drafts.length;
    const newRow: ActivityImport = {
      id: null,
      date: new Date().toISOString().split("T")[0],
      symbol: "INR",
      activity_type: "DEPOSIT",
      quantity: 1,
      unit_price: 0,
      currency: "INR",
      fee: 0,
      amount: 0,
      comment: "New Transaction",
      account_id: selectedAccountId,
      is_draft: true,
      is_valid: true,
      line_number: newIdx + 1,
      force_import: false,
    };
    setDrafts([...drafts, newRow]);
    setSelectedIds(new Set([...selectedIds, newIdx]));
  };

  // Remove row draft
  const handleDeleteRow = (index: number) => {
    const updated = drafts.filter((_, idx) => idx !== index);
    setDrafts(updated);

    const updatedSelected = new Set<number>();
    selectedIds.forEach((id) => {
      if (id < index) updatedSelected.add(id);
      if (id > index) updatedSelected.add(id - 1);
    });
    setSelectedIds(updatedSelected);
  };

  // Handle cell edit changes
  const handleCellEdit = (index: number, key: keyof ActivityImport, val: any) => {
    const updated = [...drafts];
    updated[index] = {
      ...updated[index],
      [key]: val,
    };
    
    // Recalculate amount if unit_price changes
    if (key === "unit_price") {
      updated[index].amount = val;
    }
    
    setDrafts(updated);
  };

  // Select / Deselect individual rows
  const handleSelectToggle = (index: number) => {
    const next = new Set(selectedIds);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    setSelectedIds(next);
  };

  // Select / Deselect all rows
  const handleSelectAllToggle = () => {
    if (selectedIds.size === drafts.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(drafts.map((_, idx) => idx)));
    }
  };

  // Final confirmation to save activities in DB
  const importMutation = useMutation<any, Error, ActivityImport[]>({
    mutationFn: async (payload) => {
      if (!ctx.api || !ctx.api.activities || !ctx.api.activities.import) {
        throw new Error("Import service API is unavailable");
      }
      return await ctx.api.activities.import(payload);
    },
    onSuccess: () => {
      setStep("success");
    },
    onError: (err) => {
      setErrorMsg(err.message || "Failed to import selected activities.");
    },
  });

  const handleFinalImport = () => {
    const selectedDrafts = drafts
      .filter((_, idx) => selectedIds.has(idx))
      .map((d) => ({
        ...d,
        is_draft: false, // Convert to actual transaction on finalize
      }));

    if (selectedDrafts.length === 0) {
      setErrorMsg("Please select at least one transaction to import.");
      return;
    }

    setErrorMsg("");
    importMutation.mutate(selectedDrafts);
  };

  // Calculate opening balance & activity statistics
  const netChange = drafts.reduce((acc, row) => {
    const amt = Number(row.amount || row.unit_price || 0);
    if (row.activity_type === "DEPOSIT") {
      return acc + amt;
    } else {
      return acc - amt;
    }
  }, 0);

  const totalDeposits = drafts.filter((r) => r.activity_type === "DEPOSIT").length;
  const totalWithdrawals = drafts.filter((r) => r.activity_type === "WITHDRAWAL").length;

  if (step === "loading") {
    return (
      <Page>
        <PageHeader>
          <h1 className="text-lg font-semibold sm:text-xl">Processing Statement</h1>
        </PageHeader>
        <PageContent>
          <div className="flex min-h-[45vh] flex-col items-center justify-center py-10">
            <div className="relative flex items-center justify-center mb-6">
              <div className="h-16 w-16 animate-spin rounded-full border-4 border-slate-200 border-t-blue-600"></div>
              <Icons.Loader className="absolute text-blue-600 h-6 w-6 animate-pulse" />
            </div>
            <h3 className="mb-2 text-base font-semibold text-slate-800 dark:text-slate-100">
              {loadingPhase}
            </h3>
            <p className="text-muted-foreground text-sm max-w-sm text-center">
              Please keep this page open. Large statement files can take up to a minute to ingest and process.
            </p>
          </div>
        </PageContent>
      </Page>
    );
  }

  if (step === "success") {
    return (
      <Page>
        <PageHeader>
          <h1 className="text-lg font-semibold sm:text-xl">Statement Imported Successfully</h1>
        </PageHeader>
        <PageContent>
          <div className="flex min-h-[45vh] flex-col items-center justify-center text-center py-10">
            <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-emerald-100 dark:bg-emerald-950/50">
              <Icons.Check className="h-8 w-8 text-emerald-600 dark:text-emerald-400" />
            </div>
            <h2 className="mb-2 text-xl font-bold text-slate-900 dark:text-slate-50">
              Import Complete!
            </h2>
            <p className="text-muted-foreground text-sm max-w-md mb-8">
              Successfully imported selected transactions into your Wealthfolio account. Duplicate transactions were verified and omitted.
            </p>
            <div className="flex gap-4">
              <Button variant="outline" onClick={() => {
                setFile(null);
                setDrafts([]);
                setSelectedIds(new Set());
                setStep("upload");
              }}>
                Import Another Statement
              </Button>
              <Button onClick={() => ctx.api.navigation.navigate("/activities")}>
                View Transactions
              </Button>
            </div>
          </div>
        </PageContent>
      </Page>
    );
  }

  if (step === "review") {
    return (
      <Page>
        <PageHeader>
          <div className="flex flex-col gap-2">
            <h1 className="text-lg font-semibold sm:text-xl">Review Statement Drafts</h1>
            <p className="text-muted-foreground text-sm">
              Review, edit or add transactions before committing them to the database.
            </p>
          </div>
        </PageHeader>
        <PageContent>
          <div className="flex flex-col gap-6">
            {errorMsg && (
              <div className="rounded-lg border border-red-200 bg-red-50 p-4 dark:border-red-900/50 dark:bg-red-950/20">
                <p className="text-sm text-red-600 dark:text-red-400">{errorMsg}</p>
              </div>
            )}

            {/* Statistics Summary Cards */}
            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
                <p className="text-muted-foreground text-xs font-medium uppercase tracking-wider">Total Transactions</p>
                <h3 className="mt-2 text-2xl font-bold">{drafts.length}</h3>
                <p className="text-muted-foreground text-xs mt-1">{totalDeposits} Deposits, {totalWithdrawals} Withdrawals</p>
              </div>
              <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
                <p className="text-muted-foreground text-xs font-medium uppercase tracking-wider">Net Statement Change</p>
                <h3 className={`mt-2 text-2xl font-bold ${netChange >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400"}`}>
                  {netChange >= 0 ? "+" : ""}{netChange.toFixed(2)}
                </h3>
                <p className="text-muted-foreground text-xs mt-1">Calculated transaction sum</p>
              </div>
              <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
                <p className="text-muted-foreground text-xs font-medium uppercase tracking-wider">Selected for Import</p>
                <h3 className="mt-2 text-2xl font-bold text-blue-600 dark:text-blue-400">{selectedIds.size}</h3>
                <p className="text-muted-foreground text-xs mt-1">Omitted duplicates are deselected by default</p>
              </div>
            </div>

            {/* Table Ingestion Area */}
            <div className="rounded-xl border border-slate-200 bg-white overflow-hidden shadow-sm dark:border-slate-800 dark:bg-slate-900">
              <div className="overflow-x-auto">
                <table className="w-full border-collapse text-left text-sm">
                  <thead className="bg-slate-55 bg-slate-50 border-b border-slate-200 dark:bg-slate-800/40 dark:border-slate-800">
                    <tr>
                      <th className="py-3 px-4 w-12 text-center">
                        <input
                          type="checkbox"
                          checked={selectedIds.size === drafts.length && drafts.length > 0}
                          onChange={handleSelectAllToggle}
                          className="rounded border-slate-350 accent-blue-600"
                        />
                      </th>
                      <th className="py-3 px-4 font-semibold text-slate-700 dark:text-slate-200 w-36">Date</th>
                      <th className="py-3 px-4 font-semibold text-slate-700 dark:text-slate-200 w-32">Type</th>
                      <th className="py-3 px-4 font-semibold text-slate-700 dark:text-slate-200 w-36">Amount</th>
                      <th className="py-3 px-4 font-semibold text-slate-700 dark:text-slate-200">Comment / Narration</th>
                      <th className="py-3 px-4 font-semibold text-slate-700 dark:text-slate-200 w-32">Status</th>
                      <th className="py-3 px-4 font-semibold text-slate-700 dark:text-slate-200 w-16 text-center">Delete</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-200 dark:divide-slate-800">
                    {drafts.map((row, idx) => (
                      <tr
                        key={idx}
                        className={`hover:bg-slate-50/50 dark:hover:bg-slate-800/20 ${
                          row.duplicate_of_id ? "bg-amber-50/20 dark:bg-amber-950/5" : ""
                        }`}
                      >
                        <td className="py-3 px-4 text-center">
                          <input
                            type="checkbox"
                            checked={selectedIds.has(idx)}
                            onChange={() => handleSelectToggle(idx)}
                            className="rounded border-slate-350 accent-blue-600"
                          />
                        </td>
                        <td className="py-2 px-3">
                          <input
                            type="date"
                            value={row.date}
                            onChange={(e) => handleCellEdit(idx, "date", e.target.value)}
                            className="w-full rounded border-slate-200 bg-transparent py-1 px-2 text-sm focus:border-blue-500 focus:outline-none dark:border-slate-700"
                          />
                        </td>
                        <td className="py-2 px-3">
                          <select
                            value={row.activity_type}
                            onChange={(e) => handleCellEdit(idx, "activity_type", e.target.value)}
                            className="w-full rounded border-slate-200 bg-transparent py-1 px-2 text-sm focus:border-blue-500 focus:outline-none dark:border-slate-700"
                          >
                            <option value="DEPOSIT">Deposit</option>
                            <option value="WITHDRAWAL">Withdrawal</option>
                          </select>
                        </td>
                        <td className="py-2 px-3">
                          <input
                            type="number"
                            step="0.01"
                            value={row.amount || row.unit_price || ""}
                            onChange={(e) => handleCellEdit(idx, "unit_price", Number(e.target.value))}
                            className="w-full rounded border-slate-200 bg-transparent py-1 px-2 text-sm text-right focus:border-blue-500 focus:outline-none dark:border-slate-700 font-mono"
                          />
                        </td>
                        <td className="py-2 px-3">
                          <input
                            type="text"
                            value={row.comment || ""}
                            onChange={(e) => handleCellEdit(idx, "comment", e.target.value)}
                            className="w-full rounded border-slate-200 bg-transparent py-1 px-2 text-sm focus:border-blue-500 focus:outline-none dark:border-slate-700"
                          />
                        </td>
                        <td className="py-2 px-3">
                          {row.duplicate_of_id ? (
                            <span className="inline-flex items-center gap-1 rounded bg-amber-100 py-0.5 px-2 text-xs font-semibold text-amber-800 dark:bg-amber-950/40 dark:text-amber-400">
                              <Icons.AlertCircle className="h-3 w-3" /> Duplicate
                            </span>
                          ) : (
                            <span className="inline-flex items-center gap-1 rounded bg-emerald-100 py-0.5 px-2 text-xs font-semibold text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-400">
                              New Row
                            </span>
                          )}
                        </td>
                        <td className="py-2 px-3 text-center">
                          <button
                            onClick={() => handleDeleteRow(idx)}
                            className="text-slate-400 hover:text-red-600 dark:hover:text-red-400"
                          >
                            <Icons.Trash className="h-4 w-4" />
                          </button>
                        </td>
                      </tr>
                    ))}
                    {drafts.length === 0 && (
                      <tr>
                        <td colSpan={7} className="py-10 text-center text-slate-500 dark:text-slate-400">
                          No transaction drafts. Click Add Row to manually add transactions.
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </div>

            <div className="flex flex-col gap-3 sm:flex-row sm:justify-between items-center mt-2">
              <div className="flex gap-3">
                <Button variant="outline" onClick={() => {
                  setDrafts([]);
                  setSelectedIds(new Set());
                  setStep("upload");
                }}>
                  Back to Upload
                </Button>
                <Button variant="outline" onClick={handleAddRow}>
                  <Icons.Plus className="mr-2 h-4 w-4" /> Add Row
                </Button>
              </div>

              <div className="flex gap-3">
                <Button onClick={handleFinalImport} disabled={importMutation.isPending}>
                  {importMutation.isPending ? "Importing..." : `Import Selected (${selectedIds.size})`}
                </Button>
              </div>
            </div>
          </div>
        </PageContent>
      </Page>
    );
  }

  return (
    <Page>
      <PageHeader>
        <div className="flex flex-col gap-2">
          <h1 className="text-lg font-semibold sm:text-xl font-heading">Statement Importer</h1>
          <p className="text-muted-foreground text-sm">
            Ingest and extract bank transactions from bank statement files (PDF, XLSX, CSV) automatically using local AI.
          </p>
        </div>
      </PageHeader>
      <PageContent>
        <div className="mx-auto max-w-xl py-6">
          <div className="flex flex-col gap-6 rounded-xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
            {errorMsg && (
              <div className="rounded-lg border border-red-200 bg-red-50 p-4 dark:border-red-900/50 dark:bg-red-950/20">
                <p className="text-sm text-red-600 dark:text-red-400">{errorMsg}</p>
              </div>
            )}

            {/* Account Selector Section */}
            <div className="flex flex-col gap-2">
              <label className="text-sm font-semibold text-slate-800 dark:text-slate-200">
                1. Select Target Account
              </label>
              {isLoadingAccounts ? (
                <div className="h-10 animate-pulse rounded border border-slate-200 bg-slate-100 dark:border-slate-800 dark:bg-slate-950" />
              ) : (
                <select
                  value={selectedAccountId}
                  onChange={(e) => setSelectedAccountId(e.target.value)}
                  className="rounded-lg border border-slate-200 bg-slate-50 py-2.5 px-3.5 text-sm font-medium focus:border-blue-500 focus:outline-none dark:border-slate-800 dark:bg-slate-950"
                >
                  {accounts.map((acc) => (
                    <option key={acc.id} value={acc.id}>
                      {acc.name} ({acc.currency})
                    </option>
                  ))}
                </select>
              )}
              <span className="text-muted-foreground text-xs">
                AI will match statement activities and base currency to this account.
              </span>
            </div>

            {/* Drag and Drop Zone */}
            <div className="flex flex-col gap-2">
              <label className="text-sm font-semibold text-slate-800 dark:text-slate-200">
                2. Upload Bank Statement File
              </label>
              <div
                onDragEnter={handleDrag}
                onDragOver={handleDrag}
                onDragLeave={handleDrag}
                onDrop={handleDrop}
                onClick={() => fileInputRef.current?.click()}
                className={`relative flex min-h-[180px] cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed transition-all ${
                  dragActive
                    ? "border-blue-500 bg-blue-50/20 dark:bg-blue-950/10"
                    : "border-slate-300 hover:border-slate-400 bg-slate-50/30 dark:border-slate-700/60 dark:bg-slate-950/5"
                }`}
              >
                <input
                  type="file"
                  ref={fileInputRef}
                  onChange={handleFileChange}
                  accept=".pdf,.csv,.xls,.xlsx"
                  className="hidden"
                />
                <div className="flex flex-col items-center justify-center p-6 text-center">
                  <Icons.File className="text-slate-400 mb-3 h-10 w-10" />
                  {file ? (
                    <div>
                      <p className="text-sm font-semibold text-slate-800 dark:text-slate-100">{file.name}</p>
                      <p className="text-muted-foreground text-xs mt-1 font-mono">{(file.size / 1024).toFixed(1)} KB</p>
                    </div>
                  ) : (
                    <div>
                      <p className="text-sm font-medium text-slate-700 dark:text-slate-200">
                        Drag and drop statement here, or <span className="text-blue-500 font-semibold">browse</span>
                      </p>
                      <p className="text-muted-foreground text-xs mt-1.5">
                        Supports PDF, Excel (.xls, .xlsx) and CSV bank statement files
                      </p>
                    </div>
                  )}
                </div>
              </div>
            </div>

            <Button onClick={handleUploadSubmit} disabled={!file || uploadMutation.isPending} className="w-full">
              Analyze Statement with AI
            </Button>
          </div>
        </div>
      </PageContent>
    </Page>
  );
}

export default function enable(ctx: AddonContext) {
  ctx.api.logger.info("📄 Bank Statement Importer addon is being enabled!");

  const addedItems: Array<{ remove: () => void }> = [];

  try {
    // Add sidebar item
    const sidebarItem = ctx.sidebar.addItem({
      id: "statement-importer",
      label: "Statement Importer",
      icon: <Icons.FileText className="h-5 w-5" />,
      route: "/addon/statement-importer",
      order: 150,
    });
    addedItems.push(sidebarItem);

    // Wrapper with QueryClientProvider
    const StatementImporterWrapper = () => {
      const sharedQueryClient = ctx.api.query.getClient() as QueryClient;
      return (
        <QueryClientProvider client={sharedQueryClient}>
          <StatementImporter ctx={ctx} />
        </QueryClientProvider>
      );
    };

    // Register router page
    ctx.router.add({
      path: "/addon/statement-importer",
      component: React.lazy(() =>
        Promise.resolve({
          default: StatementImporterWrapper,
        }),
      ),
    });

    ctx.api.logger.info("Bank Statement Importer addon enabled successfully!");
  } catch (error) {
    ctx.api.logger.error("Failed to initialize addon: " + (error as Error).message);
    throw error;
  }

  ctx.onDisable(() => {
    ctx.api.logger.info("🛑 Bank Statement Importer addon is being disabled");
    addedItems.forEach((item) => {
      try {
        item.remove();
      } catch (error) {
        ctx.api.logger.error("Error removing sidebar item: " + (error as Error).message);
      }
    });
  });
}
