//! Income tax slab calculation for India.
//!
//! TODO (Phase 5): Implement tax slab computation for both Old and New regimes.

/// New regime tax slabs (FY2024-25 onwards).
/// (income_limit_exclusive, tax_rate)
pub const NEW_REGIME_SLABS: &[(i64, f64)] = &[
    (300_000, 0.0),
    (700_000, 0.05),
    (1_000_000, 0.10),
    (1_200_000, 0.15),
    (1_500_000, 0.20),
    (i64::MAX, 0.30),
];

/// Surcharge threshold amounts.
pub const SURCHARGE_THRESHOLD_10PCT: i64 = 5_000_000;
pub const SURCHARGE_THRESHOLD_15PCT: i64 = 10_000_000;
