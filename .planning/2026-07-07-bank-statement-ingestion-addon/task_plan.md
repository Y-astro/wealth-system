# Task Plan: Bank Statement Ingestion Addon and Standalone Improvements

## Goal
Implement a native Wealthfolio GUI Addon and backend endpoint for bank statement ingestion via LLM extraction, and make the standalone Python scripts robust and resilient.

## Current Phase
Phase 4 - Complete

## Phases

### Phase 1: Standalone Python Script Improvements (Tier 2 & 3)
- [x] Implement dynamic model detection in `process_all.py` (querying LM Studio's `/v1/models`).
- [x] Make LM Studio/Wealthfolio endpoints configurable via environment variables in `process_all.py` and `import_cached.py`.
- [x] Add exponential backoff retries for LLM requests.
- [x] Integrate deduplication checks using `/api/v1/activities/check-duplicates`.
- [x] Test standalone script execution.
- **Status:** complete

### Phase 2: Backend Rust Endpoint Implementation (Tier 1 & 3)
- [x] Expose `POST /api/v1/activities/import/statement` endpoint in `apps/server/src/api/activities.rs`.
- [x] Implement multipart file upload and temporary file saving.
- [x] Implement subprocess execution to call `python3 ingest.py` from Rust.
- [x] Implement dynamic model resolution and chat prompt call to LM Studio via `reqwest` in Rust.
- [x] Implement opening balance adjustment calculations and deduplication analysis.
- [x] Return parsed activities drafts list.
- **Status:** complete

### Phase 3: Frontend Addon Creation (Tier 1)
- [x] Initialize `addons/statement-importer` directory structure, `manifest.json`, and configs.
- [x] Add sidebar item and route registration in `src/addon.tsx`.
- [x] Implement Account Selector and File Upload Dropzone.
- [x] Implement interactive review table with row deletion and duplicate warning flags.
- [x] Implement bulk confirmation importing activities.
- **Status:** complete

### Phase 4: Integration Testing & Verification
- [x] Start Wealthfolio server and verify compilation.
- [x] Test standalone python pipeline.
- [x] Test GUI Statement Importer Addon (uploading, reviewing, importing).
- [x] Verify balance and database state.
- **Status:** complete

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| Subprocess ingest.py | Reuses existing tested python parsing code, saving dev time and ensuring PyMuPDF layout accuracy. |
| Option A (Review Table) | Gives user final validation control over AI-generated transaction drafts before DB write. |

## Errors Encountered
| Error | Resolution |
|-------|------------|
| Unresolved module `re` | Removed regex library usage and implemented direct JSON string slicing |
| Missing `asset_id` and `isin` fields | Explicitly set them to `None` in `ActivityImport` initializers |
| Missing `packageManager` field | Added `"packageManager": "pnpm@11.10.0"` in root `package.json` |

