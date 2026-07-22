# Findings & Decisions

## Requirements
- Support statement file uploads via a native frontend React addon dashboard.
- Backend Rust handler at `POST /api/v1/activities/import/statement` that calls the python ingestor, requests LM Studio dynamically without model hardcoding, computes balances, and returns the parsed drafts.
- Display drafts in an Option A interactive grid allowing editing, row deletion, and manual insertions before DB save.

## Research Findings
- LM Studio base URLs and active model lists can be resolved dynamically from `/v1/models` and used for chat prompt completions.
- Wealthfolio's addon sdk provides native client utilities like `checkImport` and `import` to directly integrate with the app's existing ingestion and duplicate-checking framework.

## Technical Decisions
| Decision | Rationale |
|----------|-----------|
| Subprocess ingest.py | Leverages tested local bank parser logic without rewriting parsing scripts in Rust. |
| Relative API endpoint `/api/v1/...` | Works natively on both Tauri desktop webviews and web-browser deployments. |
| In-memory JSON Boundary Slicing | Eliminates heavy external regex libraries and prevents parsing compilation errors. |

## Issues Encountered
| Issue | Resolution |
|-------|------------|
| Unresolved module `re` | Replaced regex library with native string searching and slicing in Rust. |
| Missing `asset_id` and `isin` fields | Appended missing properties to the mapped `ActivityImport` rust struct instances. |
| Missing `packageManager` field | Added `"packageManager": "pnpm@11.10.0"` in root `package.json` to allow Turborepo workspace execution. |

## Resources
- [statement-importer manifest.json](file:///home/astro/wealth-system/wealthfolio/addons/statement-importer/manifest.json)
- [statement-importer addon.tsx](file:///home/astro/wealth-system/wealthfolio/addons/statement-importer/src/addon.tsx)
- [activities.rs backend handler](file:///home/astro/wealth-system/wealthfolio/apps/server/src/api/activities.rs#L370-L715)
