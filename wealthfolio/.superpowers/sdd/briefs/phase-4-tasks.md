# Phase 4 Task Briefs — Universal Ingestion (Stream D)

## P4-T1: Universal Statement Importer AI Prompt Enhancements & Template Learning Engine

### Requirements
1. **AI Prompt Enhancement**:
   - Update statement import prompt in `apps/server/src/api/activities.rs`.
   - Incorporate rules from `wealthmanager/process_all.py`:
     - Explicit instructions for extracting `closingBalance`, `closingBalanceDate`, `openingBalance`, `openingBalanceDate`.
     - Standardized ISO `YYYY-MM-DD` date parsing rules.
     - `DEPOSIT` vs `WITHDRAWAL` distinction.
     - Never use Balance column as transaction amount.
   - Support remote Gemini API fallback when local LM Studio is unavailable or returns an error.
2. **Template Learning Engine**:
   - Add DB migration `add_import_templates_v2` for `import_templates` table in `crates/storage-sqlite` adding `source_type` and `ai_template_json` columns.
   - Create template extraction and matching engine in `crates/core/src/importers/template_engine.rs` (or server API):
     - Extract structural features (column headers, text layout markers, bank header patterns).
     - Save template on successful AI parse.
     - Match uploaded file against saved templates to bypass AI call when matched.

### Files to Touch
- `apps/server/src/api/activities.rs`
- `crates/storage-sqlite/migrations/` (new migration)
- `crates/storage-sqlite/src/schema.rs` & storage models
- `crates/core/src/importers/` (new module if needed)

---

## P4-T2: Opening Balance Auto-Injection Logic

### Requirements
1. **Opening Balance Computation**:
   - In `apps/server/src/api/activities.rs` (and core importer service), inspect extracted `closingBalance` and transactions.
   - If `openingBalance` is provided, use it. Otherwise, compute:
     `computed_opening_balance = closing_balance - (sum(DEPOSIT) - sum(WITHDRAWAL))`.
2. **Auto-Inject Opening Balance Activity**:
   - Generate an `ADJUSTMENT` activity (or `DEPOSIT` with comment `Opening Balance auto-injected from statement`) for the account.
   - Set transaction date to `openingBalanceDate` (or day before first activity).
   - Ensure the account's resulting cash balance equals `closingBalance` after importing statement activities.

### Files to Touch
- `apps/server/src/api/activities.rs`
- `crates/core/src/activities/` or `crates/core/src/importers/`

---

## P4-T3: CAMS / KFintech CAS PDF Statement Parser

### Requirements
1. **Deterministic MF CAS Parser**:
   - Create deterministic parser for CAMS Consolidated Account Statement (CAS) PDF and KFintech statements.
   - Parse fields: Folio Number, Scheme Name, AMFI Code/ISIN, Date, Transaction Type (Purchase, SIP, Redemption, SWP, STP, Dividend Payout, Dividend Reinvestment), Amount, Units, NAV, Balance.
2. **Activity & Asset Mapping**:
   - Map Purchase / SIP -> `BUY` (with `SIP` subtype where applicable).
   - Map Redemption / SWP -> `SELL`.
   - Map Dividend Payout -> `DIVIDEND`.
   - Map Dividend Reinvestment -> `DIVIDEND` + `BUY` (or `DRIP` subtype).
3. **Auto-Asset Creation**:
   - Search asset database by AMFI code / ISIN / Scheme Name.
   - Auto-create Mutual Fund investment asset if not found.

### Files to Touch
- `crates/core/src/importers/cams_cas.rs` (new module)
- `crates/core/src/importers/mod.rs`
- `apps/server/src/api/activities.rs`

---

## P4-T4: Indian Broker CSV Parsers (Zerodha, Groww, AngelOne)

### Requirements
1. **Deterministic Broker CSV Parsers**:
   - Zerodha Kite tradebook / contract note CSV parser:
     `symbol, isin, trade_date, exchange, segment, trade_type, quantity, price, trade_value, brokerage, stt, charges`
   - Groww stock & mutual fund transaction CSV parser.
   - AngelOne / Upstox CSV parser.
2. **Asset Resolution & Deduplication**:
   - Resolve symbol/ISIN to existing asset or create investment asset.
   - Generate source-specific idempotency keys (e.g. `zerodha:<trade_id>` or `groww:<tx_id>`) to prevent duplicate import of trades.

### Files to Touch
- `crates/core/src/importers/broker_csv.rs` (new module)
- `crates/core/src/importers/mod.rs`
- `apps/server/src/api/activities.rs`

---

## P4-T5: Credit Card Parser & Addon UI Handoff

### Requirements
1. **Credit Card Statement Processing**:
   - Parse credit card statement PDFs/CSVs.
   - Extract Date, Narration, Amount, Debit/Credit flag, Expense category hint.
2. **Addon & Frontend UI Integration**:
   - Update `addons/statement-importer/src/addon.tsx` and main app statement import modal.
   - Add dropdown to select Statement Source Type: Bank Statement, MF CAS (CAMS/KFintech), Brokerage CSV (Zerodha/Groww), Credit Card.
   - Render preview with account selection, opening/closing balance verification badge, and expense category selection.

### Files to Touch
- `addons/statement-importer/src/addon.tsx`
- `apps/frontend/src/` (statement importer components)
