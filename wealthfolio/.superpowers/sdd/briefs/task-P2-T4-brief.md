# P2-T4: Loan Amortization Calculator + Outstanding Balance

## Task Overview

Create a loan calculator module that computes EMI, amortization schedules, and outstanding balance at any date. Integrate with the net worth service so loan liabilities show computed outstanding balance.

## Requirements

### 1. EMI and Amortization Functions

Create `crates/core/src/assets/loan_calculator.rs` with:

```rust
/// EMI calculation: EMI = P × r × (1+r)^n / ((1+r)^n - 1)
/// where P = principal, r = monthly rate, n = tenure in months
pub fn calculate_emi(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
) -> Result<Decimal>

/// Single amortization schedule entry
pub struct AmortizationEntry {
    pub month: u32,            // 1-indexed
    pub emi: Decimal,
    pub principal_component: Decimal,
    pub interest_component: Decimal,
    pub outstanding_after: Decimal,
    pub date: NaiveDate,       // The EMI date for this month
}

/// Generate the full amortization schedule
pub fn generate_amortization_schedule(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    emi_start_date: NaiveDate,
) -> Result<Vec<AmortizationEntry>>

/// Compute outstanding balance at a specific date
/// Uses the amortization schedule to find how much principal remains
pub fn calculate_outstanding_balance(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    disbursement_date: NaiveDate,
    emi_start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal>

/// Loan summary for display
pub struct LoanSummary {
    pub emi: Decimal,
    pub total_interest_payable: Decimal,
    pub total_amount_payable: Decimal,
    pub principal_paid_so_far: Decimal,
    pub interest_paid_so_far: Decimal,
    pub outstanding_balance: Decimal,
    pub completion_pct: Decimal,
    pub emis_remaining: u32,
}

/// Calculate loan summary as of a given date
pub fn calculate_loan_summary(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    emi_start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<LoanSummary>
```

### 2. Net Worth Integration

In `compute_preset_value` in `net_worth_service.rs`, add:

```rust
InstrumentPreset::HomeLoan
| InstrumentPreset::CarLoan
| InstrumentPreset::PersonalLoan => {
    // Liabilities: return the outstanding balance as a positive Decimal.
    // The net worth service already negates liabilities.
    let principal = /* read from metadata */;
    let rate = /* read from metadata */;
    let tenure = /* read from metadata */;
    let disbursement = /* read from metadata */;
    let emi_start = /* read from metadata, fallback to disbursement + 1 month */;
    
    loan_calculator::calculate_outstanding_balance(
        principal, rate, tenure, disbursement, emi_start, target_date
    ).ok()
}
InstrumentPreset::CreditCard => {
    // Credit cards: just read current_outstanding from metadata
    metadata.get("current_outstanding")
        .and_then(|v| /* parse as Decimal */)
}
```

### 3. Tests

Required test cases:
- EMI calculation matches known values (e.g., ₹50L at 8.5% for 240 months = ₹43,391)
- Amortization schedule has correct number of entries
- First entry has higher interest than principal
- Last entry has outstanding_after of ~0
- All entries sum to total_amount_payable
- Outstanding balance at start = principal
- Outstanding balance at maturity ≈ 0
- Outstanding balance midway is reasonable
- Loan summary completion percentage is correct
- Credit card outstanding reads from metadata

## Existing Metadata Schema

From `assets_constants.rs`, the `LOAN_REQUIRED_FIELDS`:
```
instrument_preset, lender, principal_amount, interest_rate_pct, tenure_months, emi_amount, disbursement_date
```

Optional: `loan_account_number`, `emi_start_date`, `linked_asset_id`, `prepayment_charges_pct`

And `CREDIT_CARD_REQUIRED_FIELDS`:
```
instrument_preset, card_issuer, credit_limit, current_outstanding, due_date
```

## Files to Create/Modify

1. **Create**: `crates/core/src/assets/loan_calculator.rs` — EMI, amortization, outstanding balance + tests
2. **Modify**: `crates/core/src/assets/mod.rs` — add `pub mod loan_calculator;` re-export
3. **Modify**: `crates/core/src/portfolio/net_worth/net_worth_service.rs` — add loan/credit card arms to `compute_preset_value`

## Constraints

- Do NOT modify `InstrumentPreset` enum or `assets_constants.rs`
- Match coding style of `fixed_income_calculator.rs`
- Use `rust_decimal::Decimal` throughout, `.round_dp(2)` for money
- All tests must pass: `cargo test --workspace`
- Compile cleanly: `cargo check --workspace`
