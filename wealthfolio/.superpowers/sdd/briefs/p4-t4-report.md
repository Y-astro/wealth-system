# P4-T4 Report: Indian Broker CSV Parsers

## Summary
- **Status:** DONE
- **Commit Hash:** 08cfe274b987b619b2629989cae1303e8150b01c
- **Test Results:** 79 unit tests passed successfully. Tests were executed across the workspace (`cargo test --workspace`) and all crates compiled properly.

## Implementation Details
- **Broker Csv Parser:** `BrokerCsvParser` created in `crates/core/src/importers/broker_csv.rs` and registered in the `importers` module. It supports `Zerodha`, `Groww`, `AngelOne` and `Upstox` parsers based on unique column headers auto-detected by `BrokerCsvParser::detect_broker`.
- **Parsing Logic:**
  - Parses trade records and translates terms (e.g. `purchase`/`buy` -> `BUY`, `redemption`/`sell` -> `SELL`).
  - Idempotency keys are assigned directly to the `comment` field for unique tracking of processed activities and resolving deduplications.
- **Asset Resolution & Creation:** `resolve_and_auto_create_broker_assets` logic properly retrieves all `asset_service` assets and resolves symbols. If an asset is not found, it is automatically created as an `EQUITY` asset.
- **Endpoint Integration:** Updated `import_statement_endpoint` in `apps/server/src/api/activities.rs`. The endpoint dynamically evaluates `BrokerCsvParser::detect_broker` output for the text payload, directly parsing when appropriate to bypass LLM logic.

## Concerns / Notes
- The CSV auto-detection uses basic header `contains` matching. If a CSV has completely unstandardised naming conventions or new formats occur from the broker platforms in the future, the fallback generic parser might fail without direct alignment updates.
- The default asset creation maps everything as `EQUITY`, even for Mutual Funds for brokers like Groww, since the distinction from the CSV row format alone might be ambiguous without external APIs. This may require future refinement if more precise categorization is needed.
