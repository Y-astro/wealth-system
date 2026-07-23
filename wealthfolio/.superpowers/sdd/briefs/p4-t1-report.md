# P4-T1 Report: Universal Statement Importer AI Prompt Enhancements & Template Learning Engine

## Status
DONE

## Completed Work
1. **AI Prompt Enhancement**:
   - Updated statement import prompt in `apps/server/src/api/activities.rs`.
   - Incorporated rules for extracting `closingBalance`, `closingBalanceDate`, strict ISO `YYYY-MM-DD` date formatting, `DEPOSIT`/`WITHDRAWAL` activity types, and strict rule never to use Balance column as transaction amount.
   - Added remote Gemini API fallback via `reqwest` if the local LM Studio endpoint is unreachable or fails.
2. **Template Learning Engine**:
   - Created database migration `2026-07-23-000000_add_import_templates_v2` for the `import_templates` table to add `source_type` and `ai_template_json` columns.
   - Created `crates/core/src/importers/template_engine.rs` to handle extraction of structural signatures from raw statement text and matching against stored templates.
3. **Testing & Verification**:
   - Ran `cargo test --workspace` to ensure no regressions.

## Commits
- `feat(importers): Universal Statement Importer AI Prompt Enhancements & Template Learning Engine`

## Concerns / Notes
- The template engine is basic and currently uses a naive heuristic to extract column headers. It can be extended with more robust regex and machine learning based patterns for more accurate template matching.
- We need to hook up the actual template engine usage inside `activities.rs` in subsequent PRs to completely bypass the AI call based on matched template logic.
