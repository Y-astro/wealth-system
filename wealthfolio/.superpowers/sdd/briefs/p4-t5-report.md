# P4-T5 Report: Credit Card Parser & Addon UI Handoff

## Summary of Changes
1. **Credit Card Parser**:
   - Created `crates/core/src/importers/credit_card.rs` containing `CreditCardParser`.
   - The parser deterministically extracts Date, Narration/Merchant, Amount, and Debit/Credit flag from CSV text.
   - Includes logic to infer expense category hints using simple heuristics based on the merchant name.
   - Registered the `credit_card` module in `crates/core/src/importers/mod.rs`.
   - Added unit tests directly in the `credit_card.rs` module.

2. **Addon UI Updates (`addons/statement-importer/src/addon.tsx`)**:
   - Added a Source Type selection dropdown featuring 'Bank Statement', 'Mutual Fund CAS', 'Brokerage CSV', and 'Credit Card Statement'.
   - Bound the Source Type selector to `uploadMutation` so it's forwarded correctly.
   - Enhanced the statement preview table with a verification badge displaying the calculated opening and closing balance.
   - Resolved Typescript mismatch errors caused by backend changes to `ActivityImport` fields (`activity_type` to `activityType`, `unit_price` to `unitPrice`, etc.) inside the frontend.

## Testing & Verification
- `cargo test --workspace` ran successfully and no tests failed (79 passed).
- Frontend Addon Typecheck (`npx tsc --noEmit`) succeeded without errors inside `addons/statement-importer`.
- Frontend Workspace Typecheck (`npx tsc --noEmit`) succeeded without errors inside `apps/frontend`.

## Commit Hash
`e23f8f1e56e05357b408b9580fff08de99db9709`

## Status
DONE
