mod flow_classifier;
pub mod performance_model;
pub mod performance_service;
pub mod real_returns;

pub use flow_classifier::{
    affects_net_contribution, affects_net_contribution_for_scope, classify_flow,
    classify_flow_for_scope, is_external_flow, is_external_flow_for_scope, FlowType,
    PerformanceScope,
};
pub use performance_model::*;
pub use performance_service::*;
pub use real_returns::{adjust_amount_for_inflation, calculate_real_return, DEFAULT_INDIA_INFLATION_RATE};
