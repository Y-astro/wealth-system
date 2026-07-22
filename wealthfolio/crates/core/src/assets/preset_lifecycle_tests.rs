use crate::assets::alternative_assets_service::validate_instrument_preset_metadata;
use crate::assets::classification_service::{classify_liquidity, classify_risk};
use crate::assets::{Asset, AssetKind, InstrumentPreset, QuoteMode};
use crate::portfolio::net_worth::{NetWorthService, LiquidityClass, RiskClass};
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use serde_json::json;

fn create_mock_asset(preset: InstrumentPreset, metadata: serde_json::Value, kind: AssetKind) -> Asset {
    Asset {
        id: format!("test_{:?}", preset),
        kind,
        name: Some(format!("Test {:?}", preset)),
        display_code: None,
        notes: None,
        metadata: Some(metadata),
        is_active: true,
        quote_mode: QuoteMode::Manual,
        quote_ccy: "INR".to_string(),
        instrument_type: None,
        instrument_symbol: None,
        instrument_exchange_mic: None,
        instrument_key: None,
        provider_config: None,
        exchange_name: None,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    }
}

#[test]
fn test_fixed_deposit_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "fixed_deposit", 
        "bank_name": "HDFC",
        "principal": "100000.0",
        "interest_rate_pct": "7.5",
        "start_date": "2023-01-01",
        "maturity_date": "2024-01-01",
        "compounding_frequency": "quarterly"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "fixed_deposit", "principal": "100000.0"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for fixed_deposit");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for fixed_deposit: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::FixedDeposit, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for fixed_deposit, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(103753).round_dp(0));
}

#[test]
fn test_recurring_deposit_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "recurring_deposit", 
        "bank_name": "SBI",
        "monthly_installment": "10000.0",
        "interest_rate_pct": "6.5",
        "start_date": "2023-01-01",
        "maturity_date": "2024-01-01",
        "compounding_frequency": "quarterly"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "recurring_deposit", "monthly_installment": "10000.0"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for recurring_deposit");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for recurring_deposit: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Liquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::RecurringDeposit, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for recurring_deposit, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(71134).round_dp(0));
}

#[test]
fn test_ppf_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "ppf", 
        "account_number": "123",
        "bank_or_post_office": "SBI",
        "interest_rate_pct": "7.1",
        "contributions": [
            { "date": "2023-04-01", "amount": "150000.0" }
        ]
    
    });

    let invalid_meta = json!({
        "instrument_preset": "ppf", "account_number": "123"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for ppf");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for ppf: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::Ppf, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for ppf, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(152585).round_dp(0));
}

#[test]
fn test_epf_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "epf", 
        "uan_number": "123456789012"
    
    });

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for epf: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::Epf, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert_eq!(value, None);
}

#[test]
fn test_nps_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "nps", 
        "pran_number": "123456789012",
        "tier": "I",
        "nav": "35.5",
        "units_held": "1000.0"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "nps", "pran_number": "123"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for nps");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for nps: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Moderate));

    let asset = create_mock_asset(InstrumentPreset::Nps, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for nps, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(35500.0).round_dp(0));
}

#[test]
fn test_insurance_term_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "insurance_term", 
        "policy_number": "POL123",
        "insurer": "LIC"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "insurance_term", "policy_number": "POL123"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for insurance_term");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for insurance_term: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::InsuranceTerm, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for insurance_term, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(0).round_dp(0));
}

#[test]
fn test_insurance_endowment_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "insurance_endowment", 
        "policy_number": "POL123",
        "insurer": "LIC",
        "surrender_value": "150000.0"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "insurance_endowment", "policy_number": "POL123"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for insurance_endowment");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for insurance_endowment: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::InsuranceEndowment, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for insurance_endowment, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(150000.0).round_dp(0));
}

#[test]
fn test_insurance_ulip_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "insurance_ulip", 
        "policy_number": "POL123",
        "insurer": "HDFC Life",
        "fund_value": "250000.0"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "insurance_ulip", "policy_number": "POL123"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for insurance_ulip");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for insurance_ulip: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Moderate));

    let asset = create_mock_asset(InstrumentPreset::InsuranceUlip, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for insurance_ulip, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(250000.0).round_dp(0));
}

#[test]
fn test_insurance_health_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "insurance_health", 
        "policy_number": "POL123",
        "insurer": "Star Health"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "insurance_health", "policy_number": "POL123"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for insurance_health");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for insurance_health: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::InsuranceHealth, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for insurance_health, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(0).round_dp(0));
}

#[test]
fn test_home_loan_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "home_loan", 
        "lender": "SBI",
        "principal_amount": "5000000.0",
        "interest_rate_pct": "8.5",
        "tenure_months": "240",
        "emi_amount": "43391.0",
        "disbursement_date": "2023-01-01"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "home_loan", "lender": "SBI"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for home_loan");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for home_loan: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Liability, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Liability, &Some(valid_meta.clone())), None);

    let asset = create_mock_asset(InstrumentPreset::HomeLoan, valid_meta.clone(), AssetKind::Liability);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for home_loan, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(4951298).round_dp(0));
}

#[test]
fn test_car_loan_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "car_loan", 
        "lender": "HDFC",
        "principal_amount": "1000000.0",
        "interest_rate_pct": "9.0",
        "tenure_months": "60",
        "emi_amount": "20758.0",
        "disbursement_date": "2023-01-01"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "car_loan", "lender": "HDFC"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for car_loan");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for car_loan: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Liability, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Liability, &Some(valid_meta.clone())), None);

    let asset = create_mock_asset(InstrumentPreset::CarLoan, valid_meta.clone(), AssetKind::Liability);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for car_loan, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(918943).round_dp(0));
}

#[test]
fn test_personal_loan_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "personal_loan", 
        "lender": "ICICI",
        "principal_amount": "500000.0",
        "interest_rate_pct": "12.0",
        "tenure_months": "36",
        "emi_amount": "16607.0",
        "disbursement_date": "2023-01-01"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "personal_loan", "lender": "ICICI"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for personal_loan");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for personal_loan: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Liability, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Liability, &Some(valid_meta.clone())), None);

    let asset = create_mock_asset(InstrumentPreset::PersonalLoan, valid_meta.clone(), AssetKind::Liability);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for personal_loan, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(428593).round_dp(0));
}

#[test]
fn test_credit_card_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "credit_card", 
        "card_issuer": "SBI Card",
        "credit_limit": "200000.0",
        "current_outstanding": "45000.0",
        "due_date": "2024-02-01"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "credit_card", "card_issuer": "SBI Card"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for credit_card");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for credit_card: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Liability, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Liability, &Some(valid_meta.clone())), None);

    let asset = create_mock_asset(InstrumentPreset::CreditCard, valid_meta.clone(), AssetKind::Liability);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert!(value.is_some(), "Expected Some for credit_card, got None. Asset metadata: {:?}", valid_meta);
    let v = value.unwrap();
    assert_eq!(v.round_dp(0), dec!(45000.0).round_dp(0));
}

#[test]
fn test_cash_in_hand_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "cash_in_hand"
    });

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for cash_in_hand: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Liquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::CashInHand, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert_eq!(value, None);
}

#[test]
fn test_sovereign_gold_bond_lifecycle() {
    let valid_meta = json!({
        "instrument_preset": "sovereign_gold_bond", 
        "series_name": "SGB 2023-24 Series I",
        "issue_price": "5926.0",
        "maturity_date": "2031-06-27"
    
    });

    let invalid_meta = json!({
        "instrument_preset": "sovereign_gold_bond", "series_name": "SGB 2023-24 Series I"
    });
    assert!(validate_instrument_preset_metadata(&Some(invalid_meta)).is_err(), "Expected invalid meta to fail for sovereign_gold_bond");

    assert!(validate_instrument_preset_metadata(&Some(valid_meta.clone())).is_ok(), "Failed valid meta for sovereign_gold_bond: {:?}", validate_instrument_preset_metadata(&Some(valid_meta.clone())));

    assert_eq!(classify_liquidity(&AssetKind::Other, &Some(valid_meta.clone())), LiquidityClass::Illiquid);
    assert_eq!(classify_risk(&AssetKind::Other, &Some(valid_meta.clone())), Some(RiskClass::Safe));

    let asset = create_mock_asset(InstrumentPreset::SovereignGoldBond, valid_meta.clone(), AssetKind::Other);
    let target_date = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
    let value = NetWorthService::compute_preset_value(&asset, &valid_meta, target_date);
    
    assert_eq!(value, None);
}
