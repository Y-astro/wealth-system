# Phase 2 Task Briefs — Indian Instruments (Weeks 4–6)

> **Goal**: Users can add and track FDs, PPF, EPF, NPS, insurance, loans, and cash via preset-driven forms with computed valuations.

## Global Constraints

- Evolve in place — no fork; extend existing codebase progressively
- Indian instruments use `AssetKind::Other` (or `Liability`) with `attributes` JSON metadata — NO new schema types
- Compile-safe Rust — `cargo check` must pass after every task
- TypeScript types exported via `ts-rs` where applicable
- No breaking changes to existing API contracts
- Frontend uses Tailwind CSS v4, Shadcn/Radix UI, react-hook-form + zod, TanStack Query v5
- Metadata flows as `Record<string, string>` in `CreateAlternativeAssetRequest.metadata`
- `InstrumentPreset` enum (15 variants) and metadata validation already exist from Phase 1
- Existing alternative asset quick-add modal has a 2-step flow: Kind selection → Form fields
- Alternative asset detail page reads metadata via `getDetailRows()` and `getSubtypeLabel()` pattern

## Task Dependency Graph

```
T1 (FD/RD calc) ──┐
T2 (PPF/EPF/NPS) ─┤
T3 (Insurance)  ───┼── T5 (Frontend preset forms) ── T6 (Frontend detail views)
T4 (Loan amort) ───┤                                         │
                   │                                          │
                   └── T7 (Cash preset) ─────────────────────┘
                                                              │
                                                        T8 (Integration tests)
```

Tasks 1–4 are backend-only and independent. Task 5 depends on 1–4 (needs the calculation APIs). Task 6 depends on 5. Task 7 is trivial and can go any time. Task 8 depends on all.

---

## Task 1: FD/RD Creation Helpers and Compound Interest Calculator

**Stream**: A1 | **Effort**: 3 days | **Files**: Backend only

### Requirements

Create a fixed-income calculation service in `crates/core/src/assets/fixed_income_calculator.rs` that computes accrued value for FDs and RDs.

#### FD Compound Interest Formula
```
V(t) = P × (1 + r/n)^(n × t_years)
```
Where:
- P = principal (from metadata `principal`)
- r = annual interest rate as decimal (from metadata `interest_rate_pct` / 100)
- n = compounding frequency per year (from metadata `compounding_frequency`: `monthly`=12, `quarterly`=4, `half_yearly`=2, `yearly`=1)
- t_years = elapsed time in years from `start_date` to target date

If target date > maturity_date, cap at maturity_date.

#### RD Monthly Installment Sum Formula
```
V(t) = Σ (for each installment i from 1 to months_elapsed) [
  installment_amount × (1 + r/n)^(n × remaining_years_i)
]
```
Where remaining_years_i = (target_date - installment_i_date) / 365.25.

#### API
```rust
pub fn calculate_fd_value(
    principal: Decimal,
    annual_rate_pct: Decimal,
    compounding_frequency: &str,  // "monthly"|"quarterly"|"half_yearly"|"yearly"
    start_date: NaiveDate,
    maturity_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal, CalculationError>

pub fn calculate_rd_value(
    monthly_installment: Decimal,
    annual_rate_pct: Decimal,
    compounding_frequency: &str,
    start_date: NaiveDate,
    maturity_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal, CalculationError>

pub fn calculate_accrued_interest_fd(
    principal: Decimal,
    annual_rate_pct: Decimal,
    compounding_frequency: &str,
    start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal, CalculationError>
```

#### Integration into Net Worth
In `net_worth_service.rs`, when processing an alternative asset with `instrument_preset = "fixed_deposit"` or `"recurring_deposit"`, compute accrued value dynamically instead of relying solely on the last manual quote. The computed value should be used if no manual quote exists for the target date (fallback computation).

Add a new function:
```rust
fn compute_preset_value(asset: &Asset, metadata: &Value, target_date: NaiveDate) -> Option<Decimal>
```

#### Tests (minimum 8)
1. FD basic: ₹1,00,000 at 7% quarterly for 1 year → expected ~₹7,186 interest
2. FD at maturity: value capped at maturity date
3. FD before start: returns principal
4. RD basic: ₹10,000/month at 7% quarterly for 12 months
5. RD partial: 6 months into 12-month RD
6. Invalid compounding frequency → error
7. Zero/negative rate → error
8. Integration: compute_preset_value with FD metadata returns correct value

#### Files to Create/Modify
- **Create**: `crates/core/src/assets/fixed_income_calculator.rs`
- **Modify**: `crates/core/src/assets/mod.rs` (add `pub mod fixed_income_calculator;`)
- **Modify**: `crates/core/src/portfolio/net_worth/net_worth_service.rs` (add `compute_preset_value` fallback)

#### Commit Message
`feat(assets): add FD/RD compound interest calculator and net worth integration`

---

## Task 2: PPF/EPF/NPS Creation Helpers and Validation

**Stream**: A2 | **Effort**: 3 days | **Files**: Backend only

### Requirements

Extend the fixed-income calculator with PPF, EPF, and NPS valuation helpers.

#### PPF
- Interest compounded annually at government-declared rate
- Default rate: 7.1% (configurable via metadata `interest_rate_pct`)
- Annual contribution limit: ₹1,50,000 (validate in metadata `yearly_limit`)
- Lock-in: 15 years from opening
- Partial withdrawal eligible after year 7
- Value = sum of yearly contributions compounded at declared rate

```rust
pub fn calculate_ppf_value(
    contributions: &[(NaiveDate, Decimal)],  // (date, amount) pairs
    annual_rate_pct: Decimal,
    target_date: NaiveDate,
) -> Result<Decimal, CalculationError>
```

#### EPF
- Manual balance entry model (user enters balance from EPFO passbook)
- No dynamic calculation needed — value comes from latest manual quote
- Validate metadata: `uan_number` required
- Contribution tracking: store yearly employee + employer contributions in metadata

#### NPS
- Two tiers: Tier I (illiquid until 60), Tier II (liquid)
- NAV-based valuation if available, else manual
- Validate metadata: `pran_number`, `tier` (must be "I" or "II")
- `compute_preset_value` for NPS: if `nav` and `units_held` in metadata, return `nav × units_held`

```rust
pub fn calculate_nps_value(
    nav: Decimal,
    units_held: Decimal,
) -> Decimal
```

#### Tests (minimum 6)
1. PPF: single year contribution at 7.1% → expected value
2. PPF: multi-year contributions with compounding
3. PPF: contribution exceeding ₹1,50,000 limit → warning (not error — soft limit)
4. NPS: nav × units_held calculation
5. NPS: tier validation ("I" and "II" valid, "III" invalid)
6. Integration: compute_preset_value with PPF metadata

#### Files to Create/Modify
- **Modify**: `crates/core/src/assets/fixed_income_calculator.rs` (add PPF, NPS functions)
- **Modify**: `crates/core/src/portfolio/net_worth/net_worth_service.rs` (extend `compute_preset_value` for PPF/NPS)

#### Commit Message
`feat(assets): add PPF/EPF/NPS valuation helpers and net worth integration`

---

## Task 3: Insurance Policy Metadata and Valuation Logic

**Stream**: A4 | **Effort**: 2 days | **Files**: Backend only

### Requirements

Implement insurance policy valuation logic. Only endowment/whole-life and ULIP policies contribute to net worth; term and health insurance contribute ₹0.

#### Valuation Rules
| Insurance Type | Net Worth Contribution |
|---|---|
| `InsuranceTerm` | ₹0 (pure protection) |
| `InsuranceHealth` | ₹0 (pure protection) |
| `InsuranceEndowment` | `surrender_value` from metadata (manual entry) |
| `InsuranceUlip` | `fund_value` from metadata, OR `nav × units_held` if both present |

#### Premium Tracking
- Store `premium_annual` and `premium_due_date` in metadata
- Add a function to check premium due status:
```rust
pub fn get_premium_status(metadata: &Value, today: NaiveDate) -> PremiumStatus
// PremiumStatus: Current, DueWithin30Days, Overdue, Unknown
```

#### Integration
In `compute_preset_value`:
- `InsuranceTerm` / `InsuranceHealth` → return `Some(Decimal::ZERO)`
- `InsuranceEndowment` → return `surrender_value` from metadata if present
- `InsuranceUlip` → return `fund_value` or `nav × units_held` from metadata

#### Tests (minimum 5)
1. Term insurance → value = 0
2. Health insurance → value = 0
3. Endowment with surrender_value → correct value
4. ULIP with nav × units_held → correct value
5. Premium status: due within 30 days

#### Files to Create/Modify
- **Create**: `crates/core/src/assets/insurance_service.rs`
- **Modify**: `crates/core/src/assets/mod.rs` (add module)
- **Modify**: `crates/core/src/portfolio/net_worth/net_worth_service.rs` (extend `compute_preset_value`)

#### Commit Message
`feat(assets): add insurance valuation logic and premium status tracking`

---

## Task 4: Loan Amortization Calculator

**Stream**: C1 | **Effort**: 3 days | **Files**: Backend only

### Requirements

Create a loan amortization service that computes EMI schedules, outstanding balance, and repayment progress.

#### EMI Formula
```
EMI = P × r × (1+r)^n / ((1+r)^n - 1)
```
Where P = principal, r = monthly rate (annual_rate/12/100), n = tenure in months.

#### API
```rust
pub struct AmortizationEntry {
    pub month: u32,
    pub emi: Decimal,
    pub principal_component: Decimal,
    pub interest_component: Decimal,
    pub outstanding_balance: Decimal,
}

pub fn calculate_emi(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
) -> Result<Decimal, CalculationError>

pub fn generate_amortization_schedule(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    start_date: NaiveDate,
) -> Result<Vec<AmortizationEntry>, CalculationError>

pub fn calculate_outstanding_balance(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal, CalculationError>
```

#### Integration
In `compute_preset_value` for loan presets (`HomeLoan`, `CarLoan`, `PersonalLoan`):
- Compute outstanding balance at target_date using amortization formula
- Return as negative value (liability)

For `CreditCard`:
- Value = negative of `current_outstanding` from metadata (manual entry only)

#### Tests (minimum 8)
1. EMI calculation: ₹50,00,000 at 8.5% for 20 years → expected ₹43,391/month (±₹10)
2. Amortization schedule: first month's principal + interest = EMI
3. Amortization: last month outstanding balance ≈ 0
4. Outstanding balance at month 0 = principal
5. Outstanding balance at tenure end ≈ 0
6. Outstanding balance at month 120 of 240-month loan
7. Credit card: returns negative of current_outstanding
8. Zero interest rate → equal principal payments

#### Files to Create/Modify
- **Create**: `crates/core/src/assets/loan_calculator.rs`
- **Modify**: `crates/core/src/assets/mod.rs` (add module)
- **Modify**: `crates/core/src/portfolio/net_worth/net_worth_service.rs` (extend `compute_preset_value`)

#### Commit Message
`feat(assets): add loan amortization calculator and outstanding balance computation`

---

## Task 5: Frontend — Instrument Preset Selection in Asset Forms

**Stream**: A, B, C | **Effort**: 3 days | **Files**: Frontend only

### Requirements

Extend the `AlternativeAssetQuickAddModal` to support instrument presets. When the user selects `AssetKind::Other`, show a second dropdown asking them to pick an `InstrumentPreset`. Based on the selected preset, render a preset-specific form with the required fields from Phase 1 constants.

#### UX Flow
1. User opens quick-add modal → selects asset kind (existing Step 1)
2. If kind is `Other`, show a new **instrument preset selector** with options:
   - Fixed Deposit, Recurring Deposit, PPF, EPF, NPS, Cash in Hand, Sovereign Gold Bond
   - "Other (no preset)" — default, shows generic form
3. If kind is `Liability`, show preset selector:
   - Home Loan, Car Loan, Personal Loan, Credit Card
   - "Other Liability" — default
4. Based on selected preset, render preset-specific form fields (Step 2)

#### Preset Form Fields

| Preset | Fields |
|---|---|
| Fixed Deposit | Bank Name*, Principal*, Interest Rate %*, Compounding (quarterly/monthly/half_yearly/yearly)*, Start Date*, Maturity Date*, Tax Saver (checkbox) |
| Recurring Deposit | Bank Name*, Monthly Installment*, Interest Rate %*, Compounding*, Start Date*, Maturity Date* |
| PPF | Account Number*, Bank/Post Office*, Interest Rate % (default 7.1) |
| EPF | UAN Number*, Current Balance* |
| NPS | PRAN Number*, Tier (I/II)*, Scheme (E/C/G/A), Fund Manager, NAV, Units Held |
| Cash in Hand | Location, Currency* (default INR) |
| SGB | Series Name, Issue Price, Interest Rate % (default 2.5), Maturity Date |
| Home Loan | Lender*, Principal*, Interest Rate %*, Tenure (months)*, EMI*, Disbursement Date*, Linked Asset (property selector) |
| Car Loan | Same as Home Loan minus Linked Asset |
| Personal Loan | Same as Car Loan |
| Credit Card | Card Issuer*, Last 4 Digits, Credit Limit, Current Outstanding*, Due Date |

`*` = required field

#### Implementation Details
- Add `InstrumentPreset` type to `src/lib/types.ts` matching the Rust enum's snake_case serde values
- Add preset options arrays to `alternative-asset-quick-add-schema.ts`
- Extend `AlternativeAssetQuickAddModal` with a preset selector step between kind selection and form
- Each preset renders its own field group component
- Form data serializes preset-specific fields into `metadata: Record<string, string>`
- Include `instrument_preset: "<preset_value>"` in metadata automatically

#### Files to Create/Modify
- **Create**: `apps/frontend/src/pages/asset/alternative-assets/components/preset-form-fields.tsx` (all preset field groups)
- **Modify**: `apps/frontend/src/pages/asset/alternative-assets/components/alternative-asset-quick-add-modal.tsx` (add preset step)
- **Modify**: `apps/frontend/src/pages/asset/alternative-assets/components/alternative-asset-quick-add-schema.ts` (add preset validation)
- **Modify**: `apps/frontend/src/lib/types.ts` (add InstrumentPreset type)

#### Commit Message
`feat(frontend): add instrument preset selection and preset-specific forms for Indian instruments`

---

## Task 6: Frontend — Instrument-Specific Detail Views

**Stream**: A, B | **Effort**: 3 days | **Files**: Frontend only

### Requirements

Extend `alternative-asset-content.tsx` to show preset-specific detail cards when viewing an asset that has an `instrument_preset` in its metadata.

#### Detail Card Sections by Preset

| Preset | Detail Sections |
|---|---|
| Fixed Deposit | Maturity countdown (days remaining), Accrued interest, Current value (computed), Interest schedule, Tax-saver badge |
| Recurring Deposit | Months remaining, Total deposited, Accrued value, Monthly installment info |
| PPF | Lock-in status, Partial withdrawal eligibility, Contribution history, Interest rate, Current balance |
| EPF | UAN display, Current balance, Last updated |
| NPS | PRAN, Tier badge (with liquidity note), Scheme preference, NAV/units if available |
| Insurance | Policy number, Insurer, Sum assured/insured, Premium amount + due date, Premium status badge (Current/Due Soon/Overdue), Type-specific: surrender value or fund value for endowment/ULIP |
| Loans | Outstanding balance (computed), EMI breakdown (principal vs interest pie), Repayment progress bar (% paid), Next EMI date, Linked property (if home loan) |
| Credit Card | Outstanding, Available limit, Utilization %, Payment due date |
| Cash | Location, Simple value display |

#### Implementation Pattern
- Add a `PresetDetailCard` component that switches on `instrument_preset` value from metadata
- Each preset renders a styled card with the relevant fields extracted from `holding.metadata`
- Use existing UI components: `Badge`, `Card`, `Separator`, `Progress` bar
- Computed values (FD accrued interest, loan outstanding) should call backend API endpoints

#### Files to Create/Modify
- **Create**: `apps/frontend/src/pages/asset/alternative-assets/components/preset-detail-card.tsx`
- **Modify**: `apps/frontend/src/pages/asset/alternative-asset-content.tsx` (integrate PresetDetailCard)

#### Commit Message
`feat(frontend): add instrument-specific detail views for Indian presets`

---

## Task 7: Cash-in-Hand Preset (Backend + Frontend)

**Stream**: B5 | **Effort**: 0.5 day | **Files**: Backend + Frontend

### Requirements

Minimal task — ensure cash-in-hand works end-to-end.

#### Backend
- In `compute_preset_value` for `CashInHand`: return the latest manual quote value (no computation needed)
- Cash is always classified as: `Liquid`, `Safe` risk
- Already handled by Phase 1 classification service — verify it works

#### Frontend
- Cash preset form: just Name, Currency (default INR), Current Value, Location (optional)
- Detail view: simple value card with location if provided
- Should already work via Task 5 and Task 6 — this task verifies it

#### Tests
1. Backend: `classify_liquidity(CashInHand)` → Liquid
2. Backend: `classify_risk(CashInHand)` → Safe
3. Frontend: can create cash-in-hand asset through preset form

#### Commit Message
`feat(assets): verify cash-in-hand preset end-to-end`

---

## Task 8: Integration Tests — Full Preset Lifecycle

**Stream**: All | **Effort**: 2 days | **Files**: Backend tests

### Requirements

Write integration tests verifying the full lifecycle for each preset: create → validate → compute value → classify → display in net worth.

#### Test Scenarios
1. Create FD with valid metadata → accrued value computed correctly in net worth
2. Create RD → monthly installment sum computed
3. Create PPF → annual compounding computed
4. Create EPF → manual balance used as-is
5. Create NPS Tier I → classified as illiquid
6. Create NPS Tier II → classified as liquid
7. Create term insurance → contributes ₹0 to net worth
8. Create ULIP → fund value used
9. Create home loan → outstanding balance is negative liability
10. Create credit card → current outstanding is negative
11. Create cash → liquid + safe classification
12. Invalid metadata → validation error

#### Files to Create
- **Create**: `crates/core/tests/preset_lifecycle_tests.rs`

#### Commit Message
`test(assets): add preset lifecycle integration tests`
