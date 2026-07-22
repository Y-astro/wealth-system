# P1-T4: Extend `NetWorthSummary` with Liquid/Illiquid/Risk Fields

## Context

You are implementing **Phase 1, Task 4** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

The existing `NetWorthResponse` struct (in `crates/core/src/portfolio/net_worth/net_worth_model.rs`) needs to be extended with liquidity and risk classification fields.

## Your Task

Add `liquid_assets`, `illiquid_assets`, `safe_assets`, `moderate_assets`, and `aggressive_assets` fields to the `NetWorthResponse` struct, and ensure they flow through the service and API.

### Files to modify

1. **`crates/core/src/portfolio/net_worth/net_worth_model.rs`** — Add fields to `NetWorthResponse`
2. **`crates/core/src/portfolio/net_worth/net_worth_model.rs`** — Add new `LiquidityClassification` and `RiskClassification` types
3. **`crates/core/src/portfolio/net_worth/net_worth_service.rs`** — Initialize new fields to `Decimal::ZERO` in the response

### Exact requirements

#### 1. New types in `net_worth_model.rs`

Add these types to the model file:

```rust
/// Liquidity classification for an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum LiquidityClass {
    Liquid,
    Illiquid,
}

/// Risk classification for an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum RiskClass {
    Safe,
    Moderate,
    Aggressive,
}
```

#### 2. Extend `NetWorthResponse`

Add these fields to `NetWorthResponse` (all default to `Decimal::ZERO`):

```rust
/// Liquid assets total (immediately accessible without penalty)
pub liquid_assets: Decimal,
/// Illiquid assets total (locked, hard to sell quickly)
pub illiquid_assets: Decimal,

/// Safe risk bucket (FD, PPF, EPF, govt bonds, debt MF)
pub safe_assets: Decimal,
/// Moderate risk bucket (balanced MF, corporate bonds, NPS-C, rental RE)
pub moderate_assets: Decimal,
/// Aggressive risk bucket (equity MF, stocks, crypto, NPS-E, private equity)
pub aggressive_assets: Decimal,
```

#### 3. Update `NetWorthResponse::empty()`

Initialize all new fields to `Decimal::ZERO` in the `empty()` constructor.

#### 4. Update `NetWorthService` 

In `net_worth_service.rs`, find the place where `NetWorthResponse` is constructed and initialize the new fields to `Decimal::ZERO`. The actual classification logic comes in Task 5 — for now, just ensure the struct builds without errors.

Search for all places where `NetWorthResponse` is constructed:
```bash
grep -n "NetWorthResponse {" crates/core/src/portfolio/net_worth/net_worth_service.rs
```

Add the new fields with `Decimal::ZERO` to each construction site.

#### 5. API layer

Check if the net worth response is returned by any API handlers. Run:
```bash
grep -rn "net_worth\|NetWorth" apps/server/src/api/ | grep -v ".pyc"
```

If there are any API response types that wrap `NetWorthResponse`, ensure they compile. Usually the Axum handler just serializes the struct directly — `serde` will handle the new fields automatically.

### Tests

Check existing tests:
```bash
cargo test -p wealthfolio-core net_worth 2>&1 | tail -20
```

Ensure all existing net worth tests still pass.

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-core 2>&1 | tail -30
```

Fix all errors.

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/portfolio/net_worth/
git commit -m "feat(net-worth): add liquidity and risk classification fields to NetWorthResponse"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-4-report.md`

Return only:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: X/Y passing
- Concerns (if any)
