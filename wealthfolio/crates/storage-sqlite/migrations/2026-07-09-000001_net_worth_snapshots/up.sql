-- Net worth monthly snapshots for the wealth growth timeline.
-- Stores a point-in-time breakdown of wealth across categories.
-- Populated by the monthly snapshot scheduler (Phase 1, Task 6).
CREATE TABLE IF NOT EXISTS net_worth_snapshots (
    id TEXT NOT NULL PRIMARY KEY,
    snapshot_date DATE NOT NULL,
    currency TEXT NOT NULL DEFAULT 'INR',

    -- Asset totals
    total_assets NUMERIC NOT NULL DEFAULT 0,
    total_liabilities NUMERIC NOT NULL DEFAULT 0,
    net_worth NUMERIC NOT NULL DEFAULT 0,

    -- Liquidity split
    liquid_assets NUMERIC NOT NULL DEFAULT 0,
    illiquid_assets NUMERIC NOT NULL DEFAULT 0,

    -- Risk classification
    safe_assets NUMERIC NOT NULL DEFAULT 0,
    moderate_assets NUMERIC NOT NULL DEFAULT 0,
    aggressive_assets NUMERIC NOT NULL DEFAULT 0,

    -- Full component breakdown as JSON
    -- Schema: { "equities": 0, "fixed_income": 0, "real_estate": 0,
    --           "gold": 0, "insurance": 0, "cash": 0, "other": 0,
    --           "home_loans": 0, "car_loans": 0, "credit_cards": 0,
    --           "other_liabilities": 0 }
    component_breakdown_json TEXT,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(snapshot_date, currency)
);

CREATE INDEX idx_net_worth_snapshots_date ON net_worth_snapshots(snapshot_date);
