//! Health check implementations.
//!
//! This module contains the individual health check implementations:
//! - Price staleness check
//! - Quote sync error check
//! - FX integrity check
//! - Classification completeness check
//! - Data consistency check
//! - Account configuration check

pub mod account_configuration;
pub mod classification;
pub mod data_consistency;
pub mod fd_maturity;
pub mod fx_integrity;
pub mod insurance_premium;
pub mod price_staleness;
pub mod quote_sync;
pub mod sip_health;

// Re-export check implementations
pub use account_configuration::AccountConfigurationCheck;
pub use classification::ClassificationCheck;
pub use data_consistency::DataConsistencyCheck;
pub use fd_maturity::FdMaturityCheck;
pub use fx_integrity::FxIntegrityCheck;
pub use insurance_premium::InsurancePremiumCheck;
pub use price_staleness::PriceStalenessCheck;
pub use quote_sync::QuoteSyncCheck;
pub use sip_health::SipHealthCheck;

// Re-export data types used by checks
pub use account_configuration::UnconfiguredAccountInfo;
pub use classification::{LegacyMigrationInfo, UnclassifiedAssetInfo};
pub use data_consistency::{ConsistencyIssueInfo, ConsistencyIssueType};
pub use fd_maturity::FdMaturityAssetInfo;
pub use fx_integrity::FxPairInfo;
pub use insurance_premium::InsurancePolicyInfo;
pub use price_staleness::AssetHoldingInfo;
pub use quote_sync::QuoteSyncErrorInfo;
pub use sip_health::SipHealthInfo;

// Re-export data gathering functions
pub use classification::gather_legacy_migration_status;
pub use quote_sync::gather_quote_sync_errors;
