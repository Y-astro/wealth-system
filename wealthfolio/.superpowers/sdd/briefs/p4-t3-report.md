# Phase 4 Task 3 Report: CAMS / KFintech CAS PDF Statement Parser

## Status
DONE

## Commit Hash
Pending (will be committed as `feat(importers): add CAMS and KFintech CAS statement parser`).

## Test Summary
Unit tests were added in `crates/core/src/importers/cams_cas.rs` for:
- Detecting CAMS / KFintech CAS statement signature.
- Parsing dummy CAS text into Wealthfolio `ActivityImport` models.

## Changes Made
- Added `crates/core/src/importers/cams_cas.rs` implementing `CamsCasParser` with deterministic regex parsing of CAS statements to extract fields: Folio Number, Scheme Name, ISIN, Transaction Date, Type, Amount, Units, NAV.
- Mapped transaction types:
  - Purchase / SIP -> BUY
  - Redemption / SWP -> SELL
  - Dividend Payout -> DIVIDEND
  - Dividend Reinvestment -> DIVIDEND (with subtype DRIP)
- Updated `crates/core/src/importers/mod.rs` to register the `cams_cas` module.
- Modified `apps/server/src/api/activities.rs` inside `import_statement_endpoint`: added format detection to check if the statement has CAMS/KFintech signatures. If so, it uses `CamsCasParser` directly, bypassing the LLM call.

## Any Concerns
- The CAS regex is currently simple and assumes a tabular layout based on typical outputs. The text extracted by `ingest.py` (via `pdftotext` or layout-aware extractors) needs to somewhat match the structure we defined (`Date Type Amount Units NAV Balance`).
- Additional tuning might be required once tested against real PDFs as `ingest.py` layout preservation might vary.


## Post-Review Fixes (Auto-Asset Resolution/Creation)
- Added `resolve_and_auto_create_cas_assets` in `apps/server/src/api/activities.rs`.
- When CAMS/KFintech CAS activities are extracted, iterates through unique scheme names / ISINs.
- Searches `state.asset_service` for existing assets matching the symbol/ISIN or scheme name.
- If no matching asset is found, automatically creates a new `Investment` asset with `instrument_type` set to Equity (Mutual Fund equivalent), `name` set to scheme name, `symbol` set to ISIN or scheme symbol, and `currency` set to INR.
- Associates the `asset_id` on the resulting `ActivityImport` items.
- Added unit tests in `apps/server/src/api/activities.rs` verifying auto-creation of missing schemes.
