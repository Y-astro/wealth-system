# P2-T5: Frontend — Instrument Preset Selection in Asset Forms

## Task Overview

Add an "Indian Instruments" category to the alternative asset quick-add modal, with instrument preset selection. When a user selects an Indian instrument preset (FD, RD, PPF, EPF, NPS, Insurance, etc.), the form should show preset-specific fields instead of the generic value form.

## Requirements

### 1. Add "Indian Instruments" Asset Category

In the quick-add modal (`apps/frontend/src/pages/asset/alternative-assets/components/alternative-asset-quick-add-modal.tsx`), add a new category card in step 1 (asset type selection):

```typescript
{
  kind: AlternativeAssetKind.OTHER,  // Maps to AssetKind::Other on backend
  label: "Indian Instruments",
  description: "FDs, PPF, NPS & more",
  icon: Icons.IndiaFlagDuotone,  // Or use an appropriate existing icon
  iconColor: "text-amber-400",
  selectedBg: "bg-amber-400/15",
  borderColor: "border-amber-400/50",
  isPresetCategory: true,  // Custom flag to trigger preset selection in step 2
}
```

### 2. Preset Selection Step

When "Indian Instruments" is selected, step 2 should show a preset selector dropdown/grid before the form fields:

Available presets (map to `InstrumentPreset` values):
| UI Label | `instrument_preset` value | Asset Kind |
|----------|--------------------------|------------|
| Fixed Deposit (FD) | `fixed_deposit` | Other |
| Recurring Deposit (RD) | `recurring_deposit` | Other |
| Public Provident Fund (PPF) | `ppf` | Other |
| Employee Provident Fund (EPF) | `epf` | Other |
| National Pension System (NPS) | `nps` | Other |
| Term Life Insurance | `insurance_term` | Other |
| Endowment/Whole Life Insurance | `insurance_endowment` | Other |
| ULIP | `insurance_ulip` | Other |
| Health Insurance | `insurance_health` | Other |
| Cash in Hand | `cash_in_hand` | Other |
| Sovereign Gold Bond (SGB) | `sovereign_gold_bond` | Investment |

And for liabilities (can also show when Liability is selected):
| UI Label | `instrument_preset` value | Asset Kind |
|----------|--------------------------|------------|
| Home Loan | `home_loan` | Liability |
| Car Loan | `car_loan` | Liability |
| Personal Loan | `personal_loan` | Liability |
| Credit Card | `credit_card` | Liability |

### 3. Preset-Specific Form Fields

When a preset is selected, show the relevant form fields based on the preset's required and optional metadata fields. The fields should be submitted as the `metadata` field in `CreateAlternativeAssetRequest`.

#### Fixed Deposit Form
- Bank Name (text, required)
- Principal Amount (money input, required) → maps to both `currentValue` AND `metadata.principal`
- Interest Rate % (number, required) → `metadata.interest_rate_pct`
- Compounding Frequency (select: Monthly/Quarterly/Half-Yearly/Yearly, default "quarterly")
- Start Date (date picker, required)
- Maturity Date (date picker, required)
- Account Number (text, optional)
- Tax Saver FD? (checkbox, optional)

#### Recurring Deposit Form
- Bank Name (text, required)
- Monthly Installment (money input, required) → `metadata.monthly_installment`
- Interest Rate % (number, required)
- Start Date (date picker, required)
- Maturity Date (date picker, required)

#### PPF Form
- Bank/Post Office (text, required)
- Account Number (text, required)
- Current Balance (money input, required) → used as `currentValue`
- Interest Rate % (number, optional, default 7.1)
- Opening Date (date picker, optional)

#### EPF Form
- UAN Number (text, required)
- Employer Name (text, optional)
- Current Balance (money input, required) → used as `currentValue`

#### NPS Form
- PRAN Number (text, required)
- Tier (select: I / II, required)
- Fund Manager (text, optional)
- Scheme Preference (select: E/C/G/A, optional)
- NAV (number, optional)
- Units Held (number, optional)
- Fund Value (money input, optional) → used as `currentValue`

#### Insurance Forms
- Policy Number (text, required)
- Insurer (text, required)
- Sum Assured/Insured (money input, optional)
- Premium Annual (money input, optional)
- Premium Due Date (date picker, optional)
- Additional fields per type:
  - Endowment: Surrender Value, Maturity Value
  - ULIP: NAV, Units Held, Fund Value
  - Health: Family Members (text, optional)

#### Loan Forms
- Lender (text, required)
- Principal Amount (money input, required) → `metadata.principal_amount`
- Interest Rate % (number, required)
- Tenure Months (number, required)
- EMI Amount (money input, required)
- Disbursement Date (date picker, required)
- EMI Start Date (date picker, optional)
- Linked Asset (select from existing assets, optional)

#### Credit Card Form
- Card Issuer (text, required)
- Credit Limit (money input, required)
- Current Outstanding (money input, required) → used as `currentValue`
- Due Date (date picker, required)
- Card Last 4 digits (text, optional)

#### Cash in Hand
- Location/Description (text, optional)
- Amount (money input, required) → used as `currentValue`

### 4. Metadata Submission

When submitting, pack the preset-specific fields into the `metadata` field of `CreateAlternativeAssetRequest`:

```typescript
const metadata: Record<string, string> = {
  instrument_preset: selectedPreset,  // e.g., "fixed_deposit"
  bank_name: formData.bankName,
  principal: formData.principal,
  interest_rate_pct: formData.interestRate,
  // ... other preset-specific fields
};

const request: CreateAlternativeAssetRequest = {
  kind: presetToKind(selectedPreset), // "other" for most, "liability" for loans
  name: formData.name,
  currency: formData.currency,
  currentValue: formData.currentValue,
  valueDate: new Date().toISOString().split("T")[0],
  metadata,
};
```

### 5. Auto-Generated Names

When a preset is selected, suggest a default name:
- FD: "{Bank Name} FD"
- RD: "{Bank Name} RD"  
- PPF: "PPF - {Bank/Post Office}"
- EPF: "EPF - {Employer Name or UAN}"
- NPS: "NPS Tier {I/II}"
- Insurance: "{Insurer} {Type}"
- Loan: "{Lender} {Type}"
- Credit Card: "{Issuer} Credit Card"
- Cash: "Cash in Hand"

## Existing Patterns to Follow

- The modal uses a 2-step flow (step 1: type selection, step 2: form)
- Uses shadcn/ui components: Dialog, Button, Input, Label, DatePickerInput, MoneyInput, ResponsiveSelect
- Uses motion/react (framer-motion) for AnimatePresence transitions
- Form state is managed with useState (not react-hook-form)
- Currency input uses CurrencyInput component
- Money inputs use MoneyInput component
- The `kind` maps to `AlternativeAssetKindApi` for the API request

## Files to Create/Modify

1. **Create**: `apps/frontend/src/pages/asset/alternative-assets/components/preset-form-fields.tsx` — Preset-specific form field components
2. **Modify**: `apps/frontend/src/pages/asset/alternative-assets/components/alternative-asset-quick-add-modal.tsx` — Add preset category, preset selector, and preset form integration
3. **Modify**: `apps/frontend/src/pages/asset/alternative-assets/components/alternative-asset-quick-add-schema.ts` — Add preset-related constants and types

## Constraints

- Do NOT modify backend Rust code
- Follow existing UI patterns and component library usage
- All preset-specific form fields should be optional except those in the `*_REQUIRED_FIELDS` constants
- The `instrument_preset` key in metadata is always required when a preset is selected
- Use existing shadcn/ui components — don't add new npm packages
