# Wealth System Transformation — SDD Progress Ledger

**Plan**: `/home/astro/Documents/wealth_system_transformation_plan.md`
**Repo**: `/home/astro/wealth-system/wealthfolio`
**Branch started from**: `8ada80f` (HEAD at session start)
**Session started**: 2026-07-09

## Global Constraints

- Evolve in place — no fork; extend existing codebase progressively
- Indian instruments use `AssetKind::Other` with `attributes` JSON metadata — NO new top-level schema types for instruments
- Compile-safe Rust — all changes must compile (`cargo check`)
- TypeScript types exported via `ts-rs` where applicable
- No breaking changes to existing API contracts
- Phase gating: Phase N must be stable before Phase N+1 begins
- Statement importer addon (Phase 1 bank statement ingestion) already complete — do not re-do

## Phase 1 — Foundation (Weeks 1–3) Tasks

| # | Task | Status | Commits | Notes |
|---|------|--------|---------|-------|
| P1-T1 | Define `InstrumentPreset` enum and metadata schema constants | ✅ DONE (reviewed) | 48a4230 | 46/46 tests; Approved |
| P1-T2 | Implement metadata validation per preset | ✅ DONE (reviewed) | 7c9de8f | 11/11 tests; Approved |
| P1-T3 | Create `net_worth_snapshots` DB migration | ✅ DONE (reviewed) | 7358217 | Approved |
| P1-T4 | Extend `NetWorthSummary` with liquid/illiquid/risk fields | ✅ DONE (reviewed) | 9b8cc65 | Approved |
| P1-T5 | Asset liquidity and risk classification logic | ✅ DONE (reviewed) | 3ffab44 | Approved |
| P1-T6 | Monthly net worth snapshot scheduler | ✅ DONE (reviewed) | 478eaf1 | Approved |
| P1-T7 | Create `crates/core/src/tax/` module skeleton | ✅ DONE (reviewed) | 24215d5 | Approved |
| P1-T8 | Create `crates/core/src/expenses/` module skeleton | ✅ DONE (reviewed) | 08aabe7 | Approved |

## Phase 2 — Indian Instruments (Weeks 4–6) Tasks

| # | Task | Status | Commits | Notes |
|---|------|--------|---------|-------|
| P2-T1 | FD/RD compound interest calculator + net worth integration | ✅ DONE (reviewed) | 7cdabb0 | 8/8 tests; Approved. Minor: lift Decimal::from_f64(365.25) to const |
| P2-T2 | PPF/EPF/NPS valuation helpers + net worth integration | ✅ DONE (reviewed) | a08b90d | 6/6 tests; Approved |
| P2-T3 | Insurance valuation logic + premium status | ✅ DONE (reviewed) | b2875f3 | 7/7 tests; Approved |
| P2-T4 | Loan amortization calculator + outstanding balance | ✅ DONE (reviewed) | 3451f85 | 4/4 tests; Approved |
| P2-T5 | Frontend: instrument preset selection in asset forms | ✅ DONE (reviewed) | d79e153 | Indian instrument preset selection & forms added |
| P2-T6 | Frontend: instrument-specific detail views | ✅ DONE (reviewed) | 6721819 | PresetDetailCard component integrated into asset details sheet & profile |
| P2-T7 | Cash-in-hand preset end-to-end verification | ✅ DONE (reviewed) | 3428c2d | Classification & NetWorthService unit tests added |
| P2-T8 | Integration tests: full preset lifecycle | ✅ DONE (reviewed) | 9310118 | 15/15 integration tests for all presets passing |

## Phase 3 — Wealth Dashboard (Weeks 7–9) Tasks

| # | Task | Status | Commits | Notes |
|---|------|--------|---------|-------|
| P3-T1 | Liquidity/risk allocation cards on net worth dashboard | ✅ DONE (reviewed) | d8e1cca | LiquidityCards + RiskAllocationCards components added |
| P3-T2 | Fix composition widget currency + enhance donut chart | ✅ DONE (reviewed) | 8a394ec | Dynamic currency prop added, INR hardcoding removed |
| P3-T3 | Enhanced Holdings page (liquidity/risk badges) | ✅ DONE (reviewed) | 3f7aff9 | Classification badges + stale valuation warning icon added |
| P3-T4 | Enhanced Income dashboard | ✅ DONE (reviewed) | c72774a | Added Indian income types awareness + info notes |

## Phase 4 — Universal Ingestion (Weeks 10–13) Tasks

| # | Task | Status | Commits | Notes |
|---|------|--------|---------|-------|
| P4-T1 | AI prompt enhancements & template learning engine | ✅ DONE (reviewed) | e0442da | Prompt rules, Gemini fallback, TemplateEngine & DB migration added; Approved |
| P4-T2 | Opening balance auto-injection logic | ✅ DONE (reviewed) | 42fcecc | Auto-injects opening balance activity with date fallback & net-change computation; Approved |
| P4-T3 | CAMS / KFintech CAS PDF statement parser | ✅ DONE (reviewed) | c9bf855 | CamsCasParser, auto-asset resolution & creation for missing MF schemes; Approved |
| P4-T4 | Indian broker CSV parsers (Zerodha, Groww, AngelOne) | ✅ DONE (reviewed) | 08cfe27 | BrokerCsvParser, auto-asset creation & idempotency key tagging; Approved |

## Recovery Notes

After any context loss, check `git log --oneline` and compare to commit hashes above.
Task briefs: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/phase-4-tasks.md`

