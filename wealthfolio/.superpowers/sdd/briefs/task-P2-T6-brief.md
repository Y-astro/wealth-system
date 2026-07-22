# P2-T6: Frontend — Instrument Preset Detail Views

## Task Overview

Add preset-specific detail cards to the asset profile page so that when viewing an asset with an instrument preset (FD, PPF, Loan, etc.), users see instrument-specific information instead of generic asset details.

## Requirements

### 1. Preset Detail Card Component

Create a component that renders preset-specific detail views:

```tsx
// apps/frontend/src/pages/asset/alternative-assets/components/preset-detail-card.tsx

interface PresetDetailCardProps {
  preset: string;  // instrument_preset from metadata
  metadata: Record<string, unknown>;
  currency: string;
}
```

### 2. Detail Views Per Preset

#### Fixed Deposit Detail
- Bank Name, Account Number
- Principal Amount
- Interest Rate (with compounding frequency)
- Start Date → Maturity Date (with countdown: "X days remaining")
- Accrued Interest (current value - principal)
- Projected Maturity Value
- Tax Saver badge if applicable
- Progress bar: elapsed tenure as % of total

#### Recurring Deposit Detail
- Bank Name
- Monthly Installment
- Interest Rate
- Installments Paid / Total
- Start → Maturity with countdown
- Total Deposited vs Current Value

#### PPF Detail
- Bank/Post Office, Account Number
- Current Balance
- Interest Rate (govt-declared)
- Opening Date
- Lock-in status (15 years)
- FY contribution (if tracked)

#### EPF Detail
- UAN Number, Employer Name
- Current Balance
- Last updated date

#### NPS Detail
- PRAN Number, Tier (I/II badge)
- Fund Manager, Scheme Preference
- NAV, Units Held, Fund Value
- Liquidity badge (Tier II = Liquid, Tier I = Locked until 60)

#### Insurance Detail (all types)
- Policy Number, Insurer
- Policy Type badge (Term/Endowment/ULIP/Health)
- Sum Assured/Insured
- Premium Amount, Due Date
- Premium Status: "Upcoming in X days" / "Overdue by X days" (with color coding)
- For Endowment: Surrender Value, Maturity Value
- For ULIP: Fund Value, NAV, Units
- For Health: Family Members

#### Loan Detail
- Lender, Loan Account Number
- Loan Type badge (Home/Car/Personal)
- Principal, Interest Rate, Tenure
- EMI Amount
- Outstanding Balance (computed)
- Principal Paid vs Interest Paid breakdown
- Completion % progress bar
- EMIs Remaining
- Linked Asset (if any, with link)
- Disbursement Date → Expected completion

#### Credit Card Detail
- Card Issuer, Last 4 digits
- Credit Limit
- Current Outstanding
- Available Limit
- Utilization % (with color: green <30%, yellow 30-60%, red >60%)
- Payment Due Date with countdown

#### Cash in Hand Detail
- Location/Description
- Amount
- Simple display

### 3. Integration into Asset Profile Page

The detail card should appear on the asset profile page (`asset-profile-page.tsx`) or in the asset details sheet (`asset-details-sheet.tsx`) when the asset has an `instrument_preset` in its metadata.

Check the asset's metadata for `instrument_preset`:
```tsx
const preset = asset.metadata?.instrument_preset;
if (preset) {
  return <PresetDetailCard preset={preset} metadata={asset.metadata} currency={asset.currency} />;
}
```

### 4. Styling

- Use the same card styling as other asset detail sections
- Use shadcn/ui Badge for status indicators
- Use progress bars (CSS or shadcn) for tenure/completion
- Use red/yellow/green color coding for overdue/upcoming/healthy status
- Use the existing formatting utilities for currency and dates

## Files to Create/Modify

1. **Create**: `apps/frontend/src/pages/asset/alternative-assets/components/preset-detail-card.tsx` — Main preset detail component
2. **Modify**: `apps/frontend/src/pages/asset/alternative-assets/components/asset-details-sheet.tsx` — Integrate preset detail card
3. **Modify**: `apps/frontend/src/pages/asset/asset-profile-page.tsx` — Add preset detail rendering (if applicable)

## Constraints

- Do NOT modify backend Rust code
- Follow existing component patterns
- Use existing shadcn/ui components
- No new npm packages
- All data comes from the asset's metadata field — no new API calls needed
