# Statement visualizations

User authorizes implementing NSDL markdown ingestion, explicit PDF-to-Markdown mappings, stock graphics, bank Sankey, and currently loaded LM Studio model support.

1. Inspect existing application and sample — complete.
2. Implement normalized statement contract, deterministic NSDL/bank parsing, source mappings, reconciliation, local model fallback and CLI — complete.
3. Expose authenticated server endpoint; integrate existing statement importer and batch workflow — complete.
4. Build routed statement dashboard, charts, source inspection, bank Sankey, filters and local report persistence — complete.
5. Validate sample, adversarial fixtures, endpoint, frontend builds and browser; independent review and usage docs — complete.

Design: one canonical versioned report with metadata, summary, accounts, allocations, history, holdings, transactions, mappings and warnings. Parse known tabular formats deterministically; request bounded JSON only for unknown bank layouts from the currently loaded local LLM. No generated/executed chart code. Preserve source indices; distinguish demat quantities from bank cash; expose unresolved reconciliation. Bank charts separate income, expenses, transfers and balance funding; currencies never combined.

Verification limitation: full TypeScript check has only the pre-existing health issue-detail-sheet category-descriptor error; preserved unrelated user changes. Both frontend targets bundle, focused tests and browser checks pass.
