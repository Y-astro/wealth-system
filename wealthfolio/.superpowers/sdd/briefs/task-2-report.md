# P1-T2 Report: Instrument Preset Metadata Validation

## Status: DONE

## Commits

| Hash | Message |
|------|---------|
| `fa829d8` | feat(assets): add instrument preset metadata validation |

## What Was Implemented

### File modified
`crates/core/src/assets/alternative_assets_service.rs`

### Changes (143 lines inserted)

1. **New imports** — added `assets_constants` field-name arrays and `InstrumentPreset` to the
   existing import block (no new files needed).

2. **`extract_instrument_preset(metadata: &Value) -> Option<InstrumentPreset>`** (pub fn)
   Reads the `"instrument_preset"` key from a JSON `Value` and deserialises it into the
   `InstrumentPreset` enum via `serde_json`. Returns `None` for a missing or unrecognised key.

3. **`validate_instrument_preset_metadata(metadata: &Option<Value>) -> Result<()>`** (pub fn)
   - Returns `Ok(())` immediately when metadata is `None` or when the JSON contains no
     `instrument_preset` key.
   - Resolves the required field list per preset (delegates to the constants from Task 1;
     insurance variants share an inline slice).
   - Collects any missing or null fields and returns a `ValidationError::InvalidInput` listing
     them; otherwise returns `Ok(())`.

4. **Call-site integration** — `validate_instrument_preset_metadata(&request.metadata)?` is
   called at the very start of `create_alternative_asset`, right after the asset-kind guard.

5. **Unit tests** — `mod preset_validation_tests` (8 tests):
   - test_no_metadata_is_valid
   - test_metadata_without_preset_is_valid
   - test_valid_fd_metadata
   - test_fd_missing_required_field
   - test_extract_preset_from_json
   - test_extract_preset_missing_key
   - test_valid_cash_metadata
   - test_valid_nps_metadata

## Test Summary

8 / 8 passing.

```
running 8 tests
test assets::alternative_assets_service::preset_validation_tests::test_no_metadata_is_valid ... ok
test assets::alternative_assets_service::preset_validation_tests::test_extract_preset_missing_key ... ok
test assets::alternative_assets_service::preset_validation_tests::test_metadata_without_preset_is_valid ... ok
test assets::alternative_assets_service::preset_validation_tests::test_valid_fd_metadata ... ok
test assets::alternative_assets_service::preset_validation_tests::test_valid_nps_metadata ... ok
test assets::alternative_assets_service::preset_validation_tests::test_valid_cash_metadata ... ok
test assets::alternative_assets_service::preset_validation_tests::test_extract_preset_from_json ... ok
test assets::alternative_assets_service::preset_validation_tests::test_fd_missing_required_field ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 797 filtered out
```

## Compile Check

`cargo check -p wealthfolio-core` — Finished with 0 errors.
Warnings present are all pre-existing "constant never used" notices for the *_OPTIONAL_FIELDS
constants added in Task 1; none are introduced by this task.

## Concerns

None. Implementation exactly matches the brief contract.
