//! Asset classification service for taxonomy-based lookups.

use crate::taxonomies::{Category, TaxonomyServiceTrait, TaxonomyWithCategories};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Represents an asset's classifications across all taxonomies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetClassifications {
    /// Primary type (from instrument_type taxonomy) - single value
    pub asset_type: Option<Category>,
    /// Risk level (from risk_category taxonomy) - single value
    pub risk_category: Option<Category>,
    /// Asset class allocations (from asset_classes taxonomy) - weighted
    pub asset_classes: Vec<CategoryWithWeight>,
    /// Sector allocations (from industries_gics taxonomy) - weighted
    pub sectors: Vec<CategoryWithWeight>,
    /// Regional allocations (from regions taxonomy) - weighted
    pub regions: Vec<CategoryWithWeight>,
    /// Custom group assignments - weighted
    pub custom_groups: Vec<CategoryWithWeight>,
}

/// A simple reference to a category with just id and name (for top-level lookups)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryWithWeight {
    pub category: Category,
    /// The top-level ancestor category (for hierarchical taxonomies like GICS)
    /// This is used for filtering when allocations are rolled up to top-level
    pub top_level_category: CategoryRef,
    pub weight: f64, // 0-100 percentage
}

pub struct AssetClassificationService {
    taxonomy_service: Arc<dyn TaxonomyServiceTrait>,
}

impl AssetClassificationService {
    pub fn new(taxonomy_service: Arc<dyn TaxonomyServiceTrait>) -> Self {
        Self { taxonomy_service }
    }

    /// Get all classifications for an asset
    pub fn get_classifications(&self, asset_id: &str) -> Result<AssetClassifications, String> {
        let assignments = self
            .taxonomy_service
            .get_asset_assignments(asset_id)
            .map_err(|e| e.to_string())?;

        let mut classifications = AssetClassifications::default();

        for assignment in assignments {
            // Get taxonomy with categories to find the category
            let taxonomy_with_cats = self
                .taxonomy_service
                .get_taxonomy(&assignment.taxonomy_id)
                .map_err(|e| e.to_string())?;

            if let Some(twc) = taxonomy_with_cats {
                // Build category lookup maps for finding top-level ancestor
                let categories = &twc.categories;

                // Find the category in the taxonomy
                let category = categories
                    .iter()
                    .find(|c| c.id == assignment.category_id)
                    .cloned();

                if let Some(cat) = category {
                    let weight = assignment.weight as f64 / 100.0; // Convert basis points to percentage

                    // Find the top-level ancestor
                    let top_level = self.find_top_level_ancestor(&cat, categories);
                    let top_level_ref = CategoryRef {
                        id: top_level.id.clone(),
                        name: top_level.name.clone(),
                    };

                    let cat_with_weight = CategoryWithWeight {
                        category: cat.clone(),
                        top_level_category: top_level_ref,
                        weight,
                    };

                    match assignment.taxonomy_id.as_str() {
                        "instrument_type" => classifications.asset_type = Some(cat),
                        "risk_category" => classifications.risk_category = Some(cat),
                        "asset_classes" => classifications.asset_classes.push(cat_with_weight),
                        "industries_gics" => classifications.sectors.push(cat_with_weight),
                        "regions" => classifications.regions.push(cat_with_weight),
                        "custom_groups" => classifications.custom_groups.push(cat_with_weight),
                        _ => {}
                    }
                }
            }
        }

        Ok(classifications)
    }

    /// Get classifications for multiple assets efficiently (caches taxonomies)
    pub fn get_classifications_batch(
        &self,
        asset_ids: &[String],
    ) -> HashMap<String, AssetClassifications> {
        let mut result: HashMap<String, AssetClassifications> = HashMap::new();

        // Cache taxonomies to avoid repeated fetches
        let mut taxonomy_cache: HashMap<String, TaxonomyWithCategories> = HashMap::new();

        for asset_id in asset_ids {
            let assignments = match self.taxonomy_service.get_asset_assignments(asset_id) {
                Ok(a) => a,
                Err(_) => continue,
            };

            if assignments.is_empty() {
                continue;
            }

            let mut classifications = AssetClassifications::default();

            for assignment in assignments {
                // Get taxonomy from cache or fetch
                let taxonomy = if let Some(cached) = taxonomy_cache.get(&assignment.taxonomy_id) {
                    cached
                } else {
                    match self.taxonomy_service.get_taxonomy(&assignment.taxonomy_id) {
                        Ok(Some(twc)) => {
                            taxonomy_cache.insert(assignment.taxonomy_id.clone(), twc);
                            taxonomy_cache.get(&assignment.taxonomy_id).unwrap()
                        }
                        _ => continue,
                    }
                };

                let categories = &taxonomy.categories;

                // Find the category
                let category = match categories.iter().find(|c| c.id == assignment.category_id) {
                    Some(c) => c.clone(),
                    None => continue,
                };

                let weight = assignment.weight as f64 / 100.0;

                // Find top-level ancestor
                let top_level = Self::find_top_level_ancestor_static(&category, categories);
                let top_level_ref = CategoryRef {
                    id: top_level.id.clone(),
                    name: top_level.name.clone(),
                };

                let cat_with_weight = CategoryWithWeight {
                    category: category.clone(),
                    top_level_category: top_level_ref,
                    weight,
                };

                match assignment.taxonomy_id.as_str() {
                    "instrument_type" => classifications.asset_type = Some(category),
                    "risk_category" => classifications.risk_category = Some(category),
                    "asset_classes" => classifications.asset_classes.push(cat_with_weight),
                    "industries_gics" => classifications.sectors.push(cat_with_weight),
                    "regions" => classifications.regions.push(cat_with_weight),
                    _ => {
                        // Custom taxonomies
                        classifications.custom_groups.push(cat_with_weight);
                    }
                }
            }

            result.insert(asset_id.clone(), classifications);
        }

        result
    }

    /// Find the top-level ancestor of a category (the one with no parent)
    fn find_top_level_ancestor<'a>(
        &self,
        category: &'a Category,
        all_categories: &'a [Category],
    ) -> &'a Category {
        Self::find_top_level_ancestor_static(category, all_categories)
    }

    /// Static version for use in batch method
    fn find_top_level_ancestor_static<'a>(
        category: &'a Category,
        all_categories: &'a [Category],
    ) -> &'a Category {
        match &category.parent_id {
            None => category, // This is already top-level
            Some(parent_id) => {
                // Find the parent category
                if let Some(parent) = all_categories.iter().find(|c| &c.id == parent_id) {
                    Self::find_top_level_ancestor_static(parent, all_categories)
                } else {
                    // Parent not found, return current category as fallback
                    category
                }
            }
        }
    }
}

use super::alternative_assets_service::extract_instrument_preset;
use crate::assets::assets_model::{AssetKind, InstrumentPreset};
use crate::portfolio::net_worth::{LiquidityClass, RiskClass};
use serde_json::Value;

/// Classify an asset as liquid or illiquid.
///
/// `metadata` is the asset's JSON attributes field.
/// Returns `LiquidityClass::Liquid` or `LiquidityClass::Illiquid`.
pub fn classify_liquidity(kind: &AssetKind, metadata: &Option<Value>) -> LiquidityClass {
    if let Some(meta) = metadata {
        if let Some(preset) = extract_instrument_preset(meta) {
            return match preset {
                InstrumentPreset::CashInHand | InstrumentPreset::RecurringDeposit => LiquidityClass::Liquid,
                InstrumentPreset::Nps => {
                    let tier = meta.get("tier").and_then(|t| t.as_str()).unwrap_or("I");
                    if tier == "II" {
                        LiquidityClass::Liquid
                    } else {
                        LiquidityClass::Illiquid
                    }
                }
                _ => LiquidityClass::Illiquid,
            };
        }
    }

    match kind {
        AssetKind::Investment | AssetKind::PreciousMetal => LiquidityClass::Liquid,
        _ => LiquidityClass::Illiquid,
    }
}

/// Classify an asset by risk level.
///
/// Returns `None` for liabilities (not classified).
pub fn classify_risk(kind: &AssetKind, metadata: &Option<Value>) -> Option<RiskClass> {
    if *kind == AssetKind::Liability {
        return None;
    }

    if let Some(meta) = metadata {
        if let Some(preset) = extract_instrument_preset(meta) {
            return match preset {
                InstrumentPreset::FixedDeposit
                | InstrumentPreset::RecurringDeposit
                | InstrumentPreset::Ppf
                | InstrumentPreset::Epf
                | InstrumentPreset::InsuranceTerm
                | InstrumentPreset::InsuranceEndowment
                | InstrumentPreset::InsuranceHealth
                | InstrumentPreset::CashInHand
                | InstrumentPreset::SovereignGoldBond => Some(RiskClass::Safe),

                InstrumentPreset::Nps | InstrumentPreset::InsuranceUlip => Some(RiskClass::Moderate),

                InstrumentPreset::HomeLoan
                | InstrumentPreset::CarLoan
                | InstrumentPreset::PersonalLoan
                | InstrumentPreset::CreditCard => None,
            };
        }
    }

    match kind {
        AssetKind::Liability => None,
        AssetKind::Investment | AssetKind::PrivateEquity => Some(RiskClass::Aggressive),
        AssetKind::Property
        | AssetKind::PreciousMetal
        | AssetKind::Collectible
        | AssetKind::Vehicle => Some(RiskClass::Moderate),
        _ => Some(RiskClass::Aggressive),
    }
}

#[cfg(test)]
mod classification_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fd_is_illiquid() {
        let meta = Some(json!({ "instrument_preset": "fixed_deposit" }));
        assert_eq!(classify_liquidity(&AssetKind::Other, &meta), LiquidityClass::Illiquid);
    }

    #[test]
    fn test_cash_is_liquid() {
        let meta = Some(json!({ "instrument_preset": "cash_in_hand" }));
        assert_eq!(classify_liquidity(&AssetKind::Other, &meta), LiquidityClass::Liquid);
    }

    #[test]
    fn test_investment_is_aggressive() {
        assert_eq!(classify_risk(&AssetKind::Investment, &None), Some(RiskClass::Aggressive));
    }

    #[test]
    fn test_ppf_is_safe() {
        let meta = Some(json!({ "instrument_preset": "ppf" }));
        assert_eq!(classify_risk(&AssetKind::Other, &meta), Some(RiskClass::Safe));
    }

    #[test]
    fn test_liability_has_no_risk_class() {
        assert_eq!(classify_risk(&AssetKind::Liability, &None), None);
    }

    #[test]
    fn test_property_is_moderate_risk() {
        assert_eq!(classify_risk(&AssetKind::Property, &None), Some(RiskClass::Moderate));
    }

    #[test]
    fn test_property_is_illiquid() {
        assert_eq!(classify_liquidity(&AssetKind::Property, &None), LiquidityClass::Illiquid);
    }
}


#[cfg(test)]
mod additional_classification_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cash_is_safe() {
        let meta = Some(json!({ "instrument_preset": "cash_in_hand" }));
        assert_eq!(classify_risk(&AssetKind::Other, &meta), Some(RiskClass::Safe));
    }
}
