//! Settings domain models.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub font: String,
    pub base_currency: String,
    pub timezone: String,
    pub instance_id: String,
    pub onboarding_completed: bool,
    pub auto_update_check_enabled: bool,
    pub menu_bar_visible: bool,
    pub sync_enabled: bool,
    pub inflation_rate: f64,
    pub financial_year_start_month: u32,
    pub tax_regime: String,
    pub enable_tax_deduction_tracking: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font: "font-mono".to_string(),
            base_currency: "INR".to_string(),
            timezone: "".to_string(),
            instance_id: "".to_string(),
            onboarding_completed: true,
            auto_update_check_enabled: true,
            menu_bar_visible: true,
            sync_enabled: true,
            inflation_rate: 0.06,
            financial_year_start_month: 4,
            tax_regime: "NEW".to_string(),
            enable_tax_deduction_tracking: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub theme: Option<String>,
    pub font: Option<String>,
    pub base_currency: Option<String>,
    pub timezone: Option<String>,
    pub onboarding_completed: Option<bool>,
    pub auto_update_check_enabled: Option<bool>,
    pub menu_bar_visible: Option<bool>,
    pub sync_enabled: Option<bool>,
    pub inflation_rate: Option<f64>,
    pub financial_year_start_month: Option<u32>,
    pub tax_regime: Option<String>,
    pub enable_tax_deduction_tracking: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sort {
    pub id: String,
    pub desc: bool,
}

/// Domain model for app setting key-value pair
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSetting {
    pub setting_key: String,
    pub setting_value: String,
}
