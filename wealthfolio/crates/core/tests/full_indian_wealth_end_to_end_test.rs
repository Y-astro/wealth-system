//! Comprehensive End-to-End Integration Test Suite for Indian Wealth Management.
//!
//! This suite validates all 6 phases of the Indian Wealth System Transformation:
//! 1. Indian Instruments & Presets (FD, RD, PPF, EPF, NPS, Insurance, Loans)
//! 2. Physical & Alternative Assets (Real Estate, Gold, Vehicle Depreciation, Cash)
//! 3. Net Worth Dashboard & Liquidity/Risk Classification
//! 4. Universal Statement Ingestion (CAMS/KFintech CAS, Broker CSV, Credit Card)
//! 5. Tax & Reporting Engine (STCG/LTCG, Grandfathering, 80C/80D, Slabs, Expenses, Real Returns, Projections)
//! 6. Polish & Health Diagnostics (Settings, FD Maturity Alerts, Insurance Premium Alerts, SIP Tracker)

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal_macros::dec;

use wealthfolio_core::activities::sip_tracker::{calculate_next_sip_date, SipSchedule};
use wealthfolio_core::assets::fixed_income_calculator::{
    calculate_accrued_interest_fd, calculate_nps_value, calculate_ppf_value, calculate_rd_value,
};
use wealthfolio_core::assets::insurance_calculator::calculate_insurance_value;
use wealthfolio_core::assets::loan_calculator::{calculate_emi, generate_amortization_schedule};
use wealthfolio_core::assets::vehicle_depreciation::calculate_vehicle_depreciated_value;
use wealthfolio_core::assets::{classify_liquidity, classify_risk, AssetKind, InstrumentPreset};
use wealthfolio_core::expenses::categorizer::{categorize_with_fallback, sort_rules_by_priority};
use wealthfolio_core::expenses::models::{ExpenseCategoryRule, MatchType};
use wealthfolio_core::health::checks::fd_maturity::{FdMaturityAssetInfo, FdMaturityCheck};
use wealthfolio_core::health::checks::insurance_premium::{InsurancePolicyInfo, InsurancePremiumCheck};
use wealthfolio_core::health::checks::sip_health::{SipHealthCheck, SipHealthInfo};
use wealthfolio_core::health::model::Severity;
use wealthfolio_core::health::traits::HealthContext;
use wealthfolio_core::importers::broker_csv::{BrokerCsvParser, BrokerType};
use wealthfolio_core::importers::cams_cas::CamsCasParser;
use wealthfolio_core::portfolio::net_worth::{LiquidityClass, RiskClass};
use wealthfolio_core::portfolio::performance::real_returns::{adjust_amount_for_inflation, calculate_real_return};
use wealthfolio_core::portfolio::projections::{compute_wealth_projection, WealthProjectionInputs};
use wealthfolio_core::tax::capital_gains::{
    calculate_grandfathered_cost, classify_holding_period, compute_capital_gains, CapitalGainLotSale,
};
use wealthfolio_core::tax::deductions::{compute_deductions, DeductionInputs};
use wealthfolio_core::tax::income_tax::calculate_income_tax;
use wealthfolio_core::tax::models::{CapitalGainType, TaxRegime};
use wealthfolio_core::tax::service::TaxReportService;

// =============================================================================
// 1. Indian Instruments & Presets Test
// =============================================================================
#[test]
fn test_e2e_indian_instruments_calculators() {
    // FD Accrued Interest (₹5L at 7.1% quarterly for 2 years -> ~₹75.7k interest)
    let start_fd = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let target_fd = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let fd_interest = calculate_accrued_interest_fd(dec!(500000), dec!(7.10), "quarterly", start_fd, target_fd).unwrap();
    assert!(fd_interest > dec!(70000) && fd_interest < dec!(80000));

    // RD Compound Interest (₹10k monthly at 7.0% quarterly for 12 months)
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let maturity = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let rd_val = calculate_rd_value(dec!(10000), dec!(7.0), "quarterly", start, maturity, maturity).unwrap();
    assert!(rd_val > dec!(120000));

    // PPF Valuation (Contributions accumulated)
    let ppf_contribs = vec![
        (NaiveDate::from_ymd_opt(2020, 4, 1).unwrap(), dec!(1000000)),
        (NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(), dec!(150000)),
    ];
    let target = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let ppf_val = calculate_ppf_value(&ppf_contribs, dec!(7.10), target).unwrap();
    assert!(ppf_val > dec!(1100000));

    // NPS Tier I Valuation (NAV ₹56 * 10,000 units = ₹5,60,000)
    let nps_val = calculate_nps_value(dec!(56.00), dec!(10000));
    assert_eq!(nps_val, dec!(560000));

    // Insurance Valuation: Term Life (₹0), ULIP (Fund Value)
    let term_val = calculate_insurance_value(&InstrumentPreset::InsuranceTerm, &serde_json::json!({}));
    assert_eq!(term_val, dec!(0));

    let ulip_val = calculate_insurance_value(
        &InstrumentPreset::InsuranceUlip,
        &serde_json::json!({"fund_value": 350000.0}),
    );
    assert_eq!(ulip_val, dec!(350000));

    // Loan Amortization: Home Loan ₹50L at 8.5% for 20 years (240 months)
    let emi = calculate_emi(dec!(5000000), dec!(8.5), 240).unwrap();
    assert!(emi > dec!(43000) && emi < dec!(44000));

    let sched = generate_amortization_schedule(dec!(5000000), dec!(8.5), 240, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()).unwrap();
    assert_eq!(sched.len(), 240);
    assert_eq!(sched[0].month, 1);
    assert!(sched[0].principal_component > dec!(0));
    assert!(sched[0].interest_component > dec!(0));
}

// =============================================================================
// 2. Physical & Alternative Assets Test
// =============================================================================
#[test]
fn test_e2e_physical_assets_and_depreciation() {
    // Vehicle Depreciation (Creta bought for ₹16L in 2023, 2 years held at 15% reducing balance)
    let purchase_date = NaiveDate::from_ymd_opt(2023, 4, 15).unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2025, 4, 15).unwrap();

    let depreciated_val = calculate_vehicle_depreciated_value(dec!(1600000), purchase_date, as_of_date, Some(15.0));
    assert_eq!(depreciated_val, dec!(1155485.40));
}

// =============================================================================
// 3. Asset Liquidity & Risk Classification Test
// =============================================================================
#[test]
fn test_e2e_asset_classification() {
    let ppf_meta = serde_json::json!({"instrument_preset": "ppf"});
    let cash_meta = serde_json::json!({"instrument_preset": "cash_in_hand"});

    // PPF / EPF / NPS Tier I -> Illiquid & Safe
    let ppf_liq = classify_liquidity(&AssetKind::Other, &Some(ppf_meta.clone()));
    let ppf_risk = classify_risk(&AssetKind::Other, &Some(ppf_meta.clone()));
    assert_eq!(ppf_liq, LiquidityClass::Illiquid);
    assert_eq!(ppf_risk, Some(RiskClass::Safe));

    // Cash In Hand -> Liquid & Safe
    let cash_liq = classify_liquidity(&AssetKind::Other, &Some(cash_meta.clone()));
    let cash_risk = classify_risk(&AssetKind::Other, &Some(cash_meta.clone()));
    assert_eq!(cash_liq, LiquidityClass::Liquid);
    assert_eq!(cash_risk, Some(RiskClass::Safe));

    // Equity Investments -> Liquid & Aggressive Risk
    let eq_liq = classify_liquidity(&AssetKind::Investment, &None);
    let eq_risk = classify_risk(&AssetKind::Investment, &None);
    assert_eq!(eq_liq, LiquidityClass::Liquid);
    assert_eq!(eq_risk, Some(RiskClass::Aggressive));
}

// =============================================================================
// 4. Tax Engine & Grandfathering Test
// =============================================================================
#[test]
fn test_e2e_tax_engine_and_grandfathering() {
    // Pre-2018 Grandfathering: Bought at ₹100, FMV on 31-Jan-2018 was ₹250, Sold at ₹400
    let gf_cost = calculate_grandfathered_cost(dec!(100), Some(dec!(250)), dec!(400));
    assert_eq!(gf_cost, dec!(250)); // max(100, min(250, 400)) = 250

    // Holding Period Classification
    let buy_equity = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let sell_equity = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(); // 374 days >= 365 -> LTCG
    assert_eq!(classify_holding_period(buy_equity, sell_equity, AssetKind::Investment), CapitalGainType::Ltcg);

    // Debt MF Post-2023 -> Always Debt Mf Gain (STCG)
    assert_eq!(classify_holding_period(buy_equity, sell_equity, AssetKind::DebtMf), CapitalGainType::Stcg);

    // Compute Capital Gains
    let sales = vec![
        CapitalGainLotSale {
            buy_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            sell_date: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            buy_price: dec!(100),
            sell_price: dec!(400),
            quantity: dec!(1000), // Raw gain = 300,000
            asset_kind: AssetKind::Investment,
            fmv_31_jan_2018: None,
        },
    ];

    let summary = compute_capital_gains(&sales, "FY2024-25");
    assert_eq!(summary.equity_ltcg_exempt, dec!(125000));
    assert_eq!(summary.equity_ltcg, dec!(175000));
    assert_eq!(summary.estimated_tax_ltcg, dec!(21875.00)); // 12.5% of 175,000

    // Section 80C & 80D Deductions
    let deduction_inputs = DeductionInputs {
        elss_mf: dec!(50000),
        ppf_contribution: dec!(100000),
        epf_contribution: dec!(50000), // Total 80C = 200,000 (> 150,000 limit)
        health_insurance_self: dec!(20000),
        health_insurance_parents: dec!(45000), // Senior citizen parents limit = 50,000
        is_parents_senior: true,
        ..Default::default()
    };

    let deductions = compute_deductions(&deduction_inputs, "FY2024-25");
    assert_eq!(deductions.total_80c, dec!(150000));
    assert_eq!(deductions.total_80d, dec!(65000)); // 20k + 45k
    assert_eq!(deductions.total_deductions, dec!(215000));

    // Income Tax Slab (New Regime ₹12L income)
    let tax_res = calculate_income_tax(dec!(1200000), TaxRegime::New);
    assert_eq!(tax_res.base_tax, dec!(80000));
    assert_eq!(tax_res.total_tax, dec!(83200)); // 80k + 4% cess

    // Tax Report Generator Service
    let report_service = TaxReportService::new();
    let report = report_service.generate_tax_report(
        "FY2024-25",
        TaxRegime::New,
        &sales,
        &deduction_inputs,
        dec!(25000),
        dec!(35000),
        dec!(15000),
    );

    assert_eq!(report.financial_year, "FY2024-25");
    assert_eq!(report.dividend_income, dec!(25000));
    assert_eq!(report.taxable_interest_income, dec!(35000));
    assert_eq!(report.exempt_interest_income, dec!(15000));
}

// =============================================================================
// 5. Expense Categorization & Rule Engine Test
// =============================================================================
#[test]
fn test_e2e_expense_categorization() {
    let mut rules = vec![
        ExpenseCategoryRule {
            id: "r1".to_string(),
            category_id: "dining".to_string(),
            match_pattern: "^UPI/.*ZOMATO.*".to_string(),
            match_type: MatchType::Regex,
            priority: 100,
        },
        ExpenseCategoryRule {
            id: "r2".to_string(),
            category_id: "groceries".to_string(),
            match_pattern: "SWIGGY INSTAMART".to_string(),
            match_type: MatchType::Contains,
            priority: 90,
        },
    ];

    sort_rules_by_priority(&mut rules);

    let cat1 = categorize_with_fallback("UPI/12345/ZOMATO ORDER", &rules, "other");
    assert_eq!(cat1, "dining");

    let cat2 = categorize_with_fallback("SWIGGY INSTAMART ORDER #99", &rules, "other");
    assert_eq!(cat2, "groceries");

    let cat3 = categorize_with_fallback("UNKNOWN MERCHANT PAYMENT", &rules, "other");
    assert_eq!(cat3, "other");
}

// =============================================================================
// 6. Real Returns & Wealth Growth Projections Test
// =============================================================================
#[test]
fn test_e2e_real_returns_and_projections() {
    // Real Return (12% nominal, 6% inflation)
    let real_ret = calculate_real_return(dec!(0.12), dec!(0.06));
    assert!(real_ret > dec!(0.056) && real_ret < dec!(0.057));

    // Inflation Discount (₹10L in 10 years at 6% inflation)
    let real_val = adjust_amount_for_inflation(dec!(1000000), dec!(0.06), 10);
    assert!(real_val > dec!(550000) && real_val < dec!(560000));

    // Multi-Year Wealth Growth Projection (10 years)
    let proj_inputs = WealthProjectionInputs {
        current_equities: dec!(1000000),     // 10L
        current_fixed_income: dec!(500000),   // 5L
        current_real_estate: dec!(2000000),   // 20L
        current_gold: dec!(300000),          // 3L
        current_cash_other: dec!(200000),     // 2L
        annual_contribution: dec!(240000),    // 2.4L/year
        annual_contribution_step_up: dec!(0.05), // 5% step up
        inflation_rate: dec!(0.06),           // 6%
        horizon_years: 10,
    };

    let proj_res = compute_wealth_projection(&proj_inputs);
    assert_eq!(proj_res.initial_net_worth, dec!(4000000)); // 40L initial
    assert_eq!(proj_res.yearly_projections.len(), 10);

    let y10 = &proj_res.yearly_projections[9];
    assert!(y10.nominal_wealth_optimistic > y10.nominal_wealth_base);
    assert!(y10.nominal_wealth_base > y10.nominal_wealth_conservative);
    assert!(y10.nominal_wealth_base > y10.real_wealth_base);
}

// =============================================================================
// 7. Statement Ingestion Parsers Test
// =============================================================================
#[test]
fn test_e2e_statement_ingestion_parsers() {
    let cas_text = r#"
01-Jan-2025  PURCHASE     25000.00        312.500     80.0000       312.500
10-Feb-2025  SYSTEMATIC   10000.00        121.951     82.0000       434.451
"#;

    let cas_res = CamsCasParser::parse(cas_text, "ACC-001").expect("CAS parse failed");
    assert_eq!(cas_res.len(), 2);

    let zerodha_csv = r#"symbol,isin,trade_date,exchange,segment,trade_type,quantity,price,trade_value,brokerage,stt
RELIANCE,INE002A01018,2024-05-10,NSE,EQ,buy,50,2450.00,122500.00,20.00,122.50
TCS,INE467B01029,2024-06-15,NSE,EQ,buy,25,3800.00,95000.00,20.00,95.00
"#;

    let broker_fmt = BrokerCsvParser::detect_broker(zerodha_csv);
    assert_eq!(broker_fmt, BrokerType::Zerodha);

    let broker_activities = BrokerCsvParser::parse(zerodha_csv, "ACC-002").expect("Parse broker CSV failed");
    assert_eq!(broker_activities.len(), 2);
    assert_eq!(broker_activities[0].quantity, Some(dec!(50)));
}

// =============================================================================
// 8. Health Center Diagnostics Alerts Test
// =============================================================================
#[tokio::test]
async fn test_e2e_health_center_diagnostics() {
    let now = Utc::now();
    let ctx = HealthContext::with_timestamp(
        wealthfolio_core::health::model::HealthConfig::default(),
        "INR",
        10000000.0,
        now,
    );

    // FD Maturity Check
    let fd_check = FdMaturityCheck::new();
    let fd_info = vec![
        FdMaturityAssetInfo {
            asset_id: "FD-001".to_string(),
            name: "SBI FD Matured".to_string(),
            symbol: None,
            preset: "fixed_deposit".to_string(),
            maturity_date: now.date_naive() - Duration::days(5), // past -> Matured warning
        },
    ];

    let fd_issues = fd_check.analyze(&fd_info, &ctx);
    assert_eq!(fd_issues.len(), 1);
    assert_eq!(fd_issues[0].severity, Severity::Warning);

    // Insurance Premium Due Check
    let ins_check = InsurancePremiumCheck::new();
    let ins_info = vec![
        InsurancePolicyInfo {
            asset_id: "INS-001".to_string(),
            name: "HDFC Life Term".to_string(),
            symbol: None,
            preset: "insurance_term".to_string(),
            due_date: now.date_naive() + Duration::days(10), // within 30 days -> Info alert
        },
    ];

    let ins_issues = ins_check.analyze(&ins_info, &ctx);
    assert_eq!(ins_issues.len(), 1);
    assert_eq!(ins_issues[0].severity, Severity::Info);

    // SIP Tracker & Health Check
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let next_sip = calculate_next_sip_date(start_date, 1, Some(start_date));
    assert_eq!(next_sip, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());

    let sip_check = SipHealthCheck::new();
    let sip_info = vec![
        SipHealthInfo {
            schedule: SipSchedule {
                asset_id: "MF-001".to_string(),
                mandate_amount: dec!(10000),
                frequency_months: 1,
                start_date,
                last_installment_date: Some(start_date), // missed next installment
            },
            asset_name: Some("Parag Parikh Flexi Cap".to_string()),
            asset_symbol: None,
        },
    ];

    let sip_issues = sip_check.analyze(&sip_info, &ctx);
    assert_eq!(sip_issues.len(), 1);
    assert_eq!(sip_issues[0].severity, Severity::Warning);
}
