// Asset constants - kept minimal after v2 refactor

// ── Indian Instrument Metadata Schemas ────────────────────────────────────────

/// Required metadata fields for Fixed Deposit assets.
pub const FD_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "bank_name",
    "principal",
    "interest_rate_pct",
    "start_date",
    "maturity_date",
];

pub const FD_OPTIONAL_FIELDS: &[&str] = &[
    "account_number",
    "compounding_frequency", // "monthly" | "quarterly" | "half_yearly" | "annually"
    "tenure_months",
    "is_tax_saver",
    "auto_renewal",
    "nomination",
];

/// Required metadata fields for Recurring Deposit assets.
pub const RD_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "bank_name",
    "monthly_installment",
    "interest_rate_pct",
    "start_date",
    "maturity_date",
];

pub const RD_OPTIONAL_FIELDS: &[&str] = &[
    "account_number",
    "tenure_months",
    "total_installments",
];

/// Required metadata fields for PPF assets.
pub const PPF_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "account_number",
    "bank_or_post_office",
];

pub const PPF_OPTIONAL_FIELDS: &[&str] = &[
    "current_interest_rate_pct",
    "opening_date",
    "maturity_date",
    "financial_year_contributions",  // JSON array of {fy, amount}
    "rate_history",                  // JSON array of {from_date, rate_pct}
];

/// Required metadata fields for EPF assets.
pub const EPF_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "uan_number",
];

pub const EPF_OPTIONAL_FIELDS: &[&str] = &[
    "employer_name",
    "employee_contribution_pct",
    "employer_contribution_pct",
    "eps_component",
    "current_balance",
    "as_of_date",
];

/// Required metadata fields for NPS assets.
pub const NPS_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "pran_number",
    "tier", // "I" | "II"
];

pub const NPS_OPTIONAL_FIELDS: &[&str] = &[
    "fund_manager",
    "scheme_preference", // "E" | "C" | "G" | "A"
    "nav",
    "units_held",
    "fund_value",
];

/// Required metadata fields for Loan/Liability presets.
pub const LOAN_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "lender",
    "principal_amount",
    "interest_rate_pct",
    "tenure_months",
    "emi_amount",
    "disbursement_date",
];

pub const LOAN_OPTIONAL_FIELDS: &[&str] = &[
    "loan_account_number",
    "emi_start_date",
    "linked_asset_id",
    "prepayment_charges_pct",
];

/// Required metadata fields for Credit Card liability.
pub const CREDIT_CARD_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "card_issuer",
    "credit_limit",
    "current_outstanding",
    "due_date",
];

pub const CREDIT_CARD_OPTIONAL_FIELDS: &[&str] = &[
    "card_last_4",
    "minimum_due",
    "interest_rate_pct",
    "reward_type",
];

/// Required metadata fields for Cash in Hand.
pub const CASH_REQUIRED_FIELDS: &[&str] = &["instrument_preset"];

pub const CASH_OPTIONAL_FIELDS: &[&str] = &["location", "denomination_notes"];

/// Required metadata fields for Sovereign Gold Bond.
pub const SGB_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "series_name",
    "issue_price",
    "maturity_date",
];

pub const SGB_OPTIONAL_FIELDS: &[&str] = &[
    "interest_rate_pct",
    "quantity_grams",
    "purchase_date",
];

/// Required metadata fields for Insurance policies (all types).
/// Term and Health insurance contribute ₹0 to net worth (pure protection).
/// Endowment/Whole Life and ULIP contribute surrender value / fund value.
pub const INSURANCE_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "policy_number",
    "insurer",
];

pub const INSURANCE_OPTIONAL_FIELDS: &[&str] = &[
    "sum_assured",
    "premium_annual",
    "premium_due_date",
    "policy_term_years",
    "rider_details",      // For Term
    "surrender_value",    // For Endowment/Whole Life
    "maturity_value",     // For Endowment
    "bonus_declared",     // For Endowment
    "fund_choices",       // For ULIP
    "nav",                // For ULIP
    "units_held",         // For ULIP
    "fund_value",         // For ULIP
    "sum_insured",        // For Health
    "family_members",     // For Health
    "renewal_date",       // For Health
];

