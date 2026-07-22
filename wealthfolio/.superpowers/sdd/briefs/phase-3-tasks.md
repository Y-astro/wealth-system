# Phase 3 — Wealth Dashboard Tasks

## Assessment of Current State

The net-worth page ALREADY EXISTS as a tab in the portfolio page (`portfolio-page.tsx`). It has:
- ✅ Hero card with total net worth, gain/loss amount and percent
- ✅ Net worth chart (area chart with time-series data)
- ✅ Interval selector (1M, 3M, 6M, 1Y, ALL)
- ✅ Balance sheet breakdown (collapsible assets/liabilities sections)
- ✅ Composition widget with pie chart and legend
- ✅ Stale valuations warning
- ✅ Quick action links

What's MISSING (per the plan):
- ❌ Liquid/Illiquid split cards with amounts and percentages
- ❌ Risk allocation cards (Safe/Moderate/Aggressive) with amounts and %
- ❌ Enhanced holdings page with unified view + filter tabs + liquidity/risk badges
- ❌ Enhanced income dashboard

## Backend APIs Available

The `NetWorthResponse` already includes these fields (added in Phase 1):
- `liquidAssets: Decimal`
- `illiquidAssets: Decimal`
- `safeAssets: Decimal`
- `moderateAssets: Decimal`
- `aggressiveAssets: Decimal`

API endpoints:
- `GET /api/net-worth?date=YYYY-MM-DD` → returns `NetWorthResponse`
- `GET /api/net-worth/history?startDate=...&endDate=...` → returns `NetWorthHistoryPoint[]`

## Global Constraints

- Use existing UI component library from `@wealthfolio/ui` (Button, Card, Icons, etc.)
- Follow existing design patterns (shadcn/ui-based components with Tailwind)
- Use `PrivacyAmount` for all money displays (supports privacy toggle)
- Use existing hooks pattern (`useNetWorth`, `useNetWorthHistory` from `use-alternative-assets.ts`)
- The net worth theme color is `hsl(38 75% 50%)` (golden orange)
- No breaking changes to existing components

---

## P3-T1: Liquid/Illiquid & Risk Allocation Cards

**Goal**: Add two card groups between the hero section and the breakdown section in `net-worth-content.tsx`.

**What to build**:

1. **Liquidity Cards Section** — Two side-by-side cards:
   - "Liquid" card: Shows `liquidAssets` value + % of total assets
   - "Illiquid" card: Shows `illiquidAssets` value + % of total assets
   - Visual: Horizontal bar showing the split ratio
   
2. **Risk Allocation Cards Section** — Three side-by-side cards:
   - "Safe" card (green accent): Shows `safeAssets` + % of total assets
   - "Moderate" card (yellow/amber accent): Shows `moderateAssets` + %
   - "Aggressive" card (red accent): Shows `aggressiveAssets` + %
   - Visual: Horizontal bar showing the three-way split

**Location in component**: Insert after the chart/IntervalSelector section and before the "Content section" div in `NetWorthContent`.

**Key files**:
- MODIFY: `apps/frontend/src/pages/net-worth/net-worth-content.tsx`

**Data source**: `netWorthData` already has `liquidAssets`, `illiquidAssets`, `safeAssets`, `moderateAssets`, `aggressiveAssets` fields.

**Design guidelines**:
- Use `bg-card` cards with subtle colored left border or top accent
- Use `PrivacyAmount` for all values
- Show percentage as plain text
- Responsive: 2 columns on mobile for liquidity, 3 columns for risk (or stack on very small screens)
- Match existing card style from the app (see income-page.tsx cards)

---

## P3-T2: Enhanced Composition & Chart Improvements

**Goal**: Improve the existing CompositionWidget pie chart and net worth timeline chart.

**Changes**:
1. Add a hover tooltip to the donut chart with currency-formatted amounts
2. Add the currency from the user's settings instead of hardcoded INR
3. Ensure the composition widget properly handles all asset categories

**Key files**:
- MODIFY: `apps/frontend/src/pages/net-worth/net-worth-content.tsx` (CompositionWidget)
- MODIFY: `apps/frontend/src/pages/net-worth/net-worth-chart.tsx` (minor refinements)

---

## P3-T3: Enhanced Holdings Page with Unified View

**Goal**: Add liquidity and risk badges to alternative holdings table. Add filter capabilities.

**What to build**:
1. Liquidity badge (Liquid/Illiquid) on each alternative asset row
2. Risk badge (Safe/Moderate/Aggressive) on each alternative asset row  
3. Stale value indicator (yellow warning icon) if manual asset hasn't been revalued in >90 days

**Key files**:
- MODIFY: `apps/frontend/src/pages/holdings/components/alternative-holdings-table.tsx`
- MODIFY: `apps/frontend/src/pages/holdings/components/alternative-holdings-list-mobile.tsx`

**Data**: Alternative holdings already have metadata with `instrument_preset`. The classification functions `classify_liquidity()` and `classify_risk()` exist in the backend. The frontend needs to derive these from the asset kind + metadata.

---

## P3-T4: Enhanced Income Dashboard

**Goal**: Extend the income dashboard to show additional income types beyond dividends and interest.

**What to add**:
- "Rental Income" category
- "FD Interest" category  
- "PPF Interest" category (exempt indicator)
- Tax-relevant income highlighting

**Key files**:
- MODIFY: `apps/frontend/src/pages/income/income-page.tsx`

**Note**: This depends on the backend income service also tracking these income types, which is a Phase 2/3 boundary item. For now, add the UI categories and show them when data exists.
