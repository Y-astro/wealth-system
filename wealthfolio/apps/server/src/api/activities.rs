use std::collections::HashMap;
use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use wealthfolio_core::accounts::AccountServiceTrait;
use wealthfolio_core::activities::{
    import_type, Activity, ActivityBulkMutationRequest, ActivityBulkMutationResult, ActivityImport,
    ActivitySearchResponse, ActivityUpdate, ImportActivitiesResult, ImportAssetCandidate,
    ImportAssetPreviewItem, ImportMappingData, ImportTemplateData, NewActivity, ParseConfig,
    ParsedCsvResult,
};

use super::shared::parse_date_optional;

#[derive(serde::Deserialize, ts_rs::TS)]
#[serde(untagged)]
#[ts(export)]
enum SortWrapper {
    One(wealthfolio_core::activities::Sort),
    Many(Vec<wealthfolio_core::activities::Sort>),
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[serde(untagged)]
#[ts(export)]
enum StringOrVec {
    One(String),
    Many(Vec<String>),
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct ActivitySearchBody {
    page: i64,
    #[serde(rename = "pageSize")]
    page_size: i64,
    #[serde(rename = "accountIdFilter")]
    account_id_filter: Option<StringOrVec>,
    #[serde(rename = "activityTypeFilter")]
    activity_type_filter: Option<StringOrVec>,
    #[serde(rename = "assetIdKeyword")]
    asset_id_keyword: Option<String>,
    // Allow addons to pass either a single sort or an array (we pick the first)
    sort: Option<SortWrapper>,
    #[serde(rename = "needsReviewFilter")]
    needs_review_filter: Option<bool>,
    #[serde(rename = "dateFrom")]
    date_from: Option<String>, // YYYY-MM-DD format
    #[serde(rename = "dateTo")]
    date_to: Option<String>, // YYYY-MM-DD format
    #[serde(rename = "instrumentTypeFilter")]
    instrument_type_filter: Option<StringOrVec>,
}

async fn search_activities(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ActivitySearchBody>,
) -> ApiResult<Json<ActivitySearchResponse>> {
    // Normalize sort to a single value if provided
    let sort_normalized: Option<wealthfolio_core::activities::Sort> = match body.sort {
        Some(SortWrapper::One(s)) => Some(s),
        Some(SortWrapper::Many(v)) => v.into_iter().next(),
        None => None,
    };
    let account_ids: Option<Vec<String>> = match body.account_id_filter {
        Some(StringOrVec::One(s)) => Some(vec![s]),
        Some(StringOrVec::Many(v)) => Some(v),
        None => None,
    };
    let types: Option<Vec<String>> = match body.activity_type_filter {
        Some(StringOrVec::One(s)) => Some(vec![s]),
        Some(StringOrVec::Many(v)) => Some(v),
        None => None,
    };
    let instrument_types: Option<Vec<String>> = match body.instrument_type_filter {
        Some(StringOrVec::One(s)) => Some(vec![s]),
        Some(StringOrVec::Many(v)) => Some(v),
        None => None,
    };
    // Parse date filters
    let date_from_parsed = parse_date_optional(body.date_from, "dateFrom")?;
    let date_to_parsed = parse_date_optional(body.date_to, "dateTo")?;

    let resp = state.activity_service.search_activities(
        body.page,
        body.page_size,
        account_ids,
        types,
        body.asset_id_keyword,
        sort_normalized,
        body.needs_review_filter,
        date_from_parsed,
        date_to_parsed,
        instrument_types,
    )?;
    Ok(Json(resp))
}

async fn create_activity(
    State(state): State<Arc<AppState>>,
    Json(activity): Json<NewActivity>,
) -> ApiResult<Json<Activity>> {
    let created = state.activity_service.create_activity(activity).await?;

    // Domain events handle asset enrichment and portfolio recalculation
    Ok(Json(created))
}

async fn update_activity(
    State(state): State<Arc<AppState>>,
    Json(activity): Json<ActivityUpdate>,
) -> ApiResult<Json<Activity>> {
    let updated = state.activity_service.update_activity(activity).await?;
    // Domain events handle asset enrichment and portfolio recalculation
    Ok(Json(updated))
}

async fn save_activities(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ActivityBulkMutationRequest>,
) -> ApiResult<Json<ActivityBulkMutationResult>> {
    let result = state
        .activity_service
        .bulk_mutate_activities(request)
        .await?;
    // Domain events handle asset enrichment and portfolio recalculation
    Ok(Json(result))
}

async fn delete_activity(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Activity>> {
    let deleted = state.activity_service.delete_activity(id).await?;
    // Domain events handle portfolio recalculation
    Ok(Json(deleted))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct ImportCheckBody {
    activities: Vec<ActivityImport>,
}

async fn check_activities_import(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ImportCheckBody>,
) -> ApiResult<Json<Vec<ActivityImport>>> {
    let res = state
        .activity_service
        .check_activities_import(body.activities)
        .await?;
    Ok(Json(res))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct ImportBody {
    activities: Vec<ActivityImport>,
}

async fn import_activities(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ImportBody>,
) -> ApiResult<Json<ImportActivitiesResult>> {
    let result = state
        .activity_service
        .import_activities(body.activities)
        .await?;
    // Domain events handle asset enrichment and portfolio recalculation
    Ok(Json(result))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct AssetPreviewBody {
    candidates: Vec<ImportAssetCandidate>,
}

async fn preview_import_assets(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AssetPreviewBody>,
) -> ApiResult<Json<Vec<ImportAssetPreviewItem>>> {
    let result = state
        .activity_service
        .preview_import_assets(body.candidates)
        .await?;
    Ok(Json(result))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct MappingQuery {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "contextKind", default = "default_activity_context_kind")]
    context_kind: String,
}

fn default_activity_context_kind() -> String {
    import_type::ACTIVITY.to_string()
}

async fn get_account_import_mapping(
    State(state): State<Arc<AppState>>,
    Query(q): Query<MappingQuery>,
) -> ApiResult<Json<ImportMappingData>> {
    let res = state
        .activity_service
        .get_import_mapping(q.account_id, q.context_kind)?;
    Ok(Json(res))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct SaveMappingBody {
    mapping: ImportMappingData,
}

async fn save_account_import_mapping(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveMappingBody>,
) -> ApiResult<Json<ImportMappingData>> {
    let res = state
        .activity_service
        .save_import_mapping(body.mapping)
        .await?;
    Ok(Json(res))
}

async fn list_import_templates(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ImportTemplateData>>> {
    Ok(Json(state.activity_service.list_import_templates()?))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct ImportTemplateQuery {
    id: String,
}

async fn get_import_template(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ImportTemplateQuery>,
) -> ApiResult<Json<ImportTemplateData>> {
    Ok(Json(state.activity_service.get_import_template(q.id)?))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct SaveImportTemplateBody {
    template: ImportTemplateData,
}

async fn save_import_template(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveImportTemplateBody>,
) -> ApiResult<Json<ImportTemplateData>> {
    let result = state
        .activity_service
        .save_import_template(body.template)
        .await?;
    Ok(Json(result))
}

async fn delete_import_template(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ImportTemplateQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    state.activity_service.delete_import_template(q.id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct LinkAccountTemplateBody {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "templateId")]
    template_id: String,
    #[serde(rename = "contextKind", default = "default_activity_context_kind")]
    context_kind: String,
}

async fn link_account_template(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LinkAccountTemplateBody>,
) -> ApiResult<Json<serde_json::Value>> {
    state
        .activity_service
        .link_account_template(body.account_id, body.template_id, body.context_kind)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export)]
struct CheckDuplicatesBody {
    #[serde(rename = "idempotencyKeys")]
    idempotency_keys: Vec<String>,
}

#[derive(serde::Serialize, ts_rs::TS)]
#[ts(export)]
struct CheckDuplicatesResponse {
    duplicates: HashMap<String, String>,
}

async fn check_existing_duplicates(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CheckDuplicatesBody>,
) -> ApiResult<Json<CheckDuplicatesResponse>> {
    let duplicates = state
        .activity_service
        .check_existing_duplicates(body.idempotency_keys)?;
    Ok(Json(CheckDuplicatesResponse { duplicates }))
}

async fn parse_csv_endpoint(
    State(_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> ApiResult<Json<ParsedCsvResult>> {
    let mut file_content: Option<Vec<u8>> = None;
    let mut config = ParseConfig::default();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        crate::error::ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_content = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            crate::error::ApiError::BadRequest(format!(
                                "Failed to read file content: {}",
                                e
                            ))
                        })?
                        .to_vec(),
                );
            }
            "config" => {
                let config_bytes = field.bytes().await.map_err(|e| {
                    crate::error::ApiError::BadRequest(format!("Failed to read config: {}", e))
                })?;
                config = serde_json::from_slice(&config_bytes).map_err(|e| {
                    crate::error::ApiError::BadRequest(format!("Invalid config JSON: {}", e))
                })?;
            }
            _ => {}
        }
    }

    let content = file_content.ok_or_else(|| {
        crate::error::ApiError::BadRequest("Missing file in multipart request".to_string())
    })?;

    let result = wealthfolio_core::activities::parse_csv(&content, &config)?;
    Ok(Json(result))
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LlmActivity {
    date: String,
    activity_type: String,
    symbol: String,
    quantity: Option<rust_decimal::Decimal>,
    unit_price: Option<rust_decimal::Decimal>,
    currency: String,
    comment: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LlmOutput {
    opening_balance: Option<rust_decimal::Decimal>,
    opening_balance_date: Option<String>,
    closing_balance: Option<rust_decimal::Decimal>,
    closing_balance_date: Option<String>,
    activities: Vec<LlmActivity>,
}

async fn import_statement_endpoint(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> ApiResult<Json<Vec<ActivityImport>>> {
    let mut file_content: Option<Vec<u8>> = None;
    let mut filename = "statement.pdf".to_string();
    let mut account_id: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        crate::error::ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                filename = field.file_name().unwrap_or("statement.pdf").to_string();
                file_content = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            crate::error::ApiError::BadRequest(format!(
                                "Failed to read file content: {}",
                                e
                            ))
                        })?
                        .to_vec(),
                );
            }
            "accountId" => {
                let bytes = field.bytes().await.map_err(|e| {
                    crate::error::ApiError::BadRequest(format!("Failed to read accountId field: {}", e))
                })?;
                let val = String::from_utf8(bytes.to_vec())
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !val.is_empty() {
                    account_id = Some(val);
                }
            }
            _ => {}
        }
    }

    let content = file_content.ok_or_else(|| {
        crate::error::ApiError::BadRequest("Missing file in multipart request".to_string())
    })?;

    // 1. Save file to temporary path preserving original extension
    let extension = std::path::Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("pdf");
    
    let temp_dir = std::env::temp_dir();
    let temp_filename = format!("statement_{}.{}", uuid::Uuid::new_v4(), extension);
    let temp_filepath = temp_dir.join(temp_filename);
    
    std::fs::write(&temp_filepath, &content).map_err(|e| {
        crate::error::ApiError::Internal(format!("Failed to write temporary file: {}", e))
    })?;

    // 2. Call python script ingest.py to extract text
    let output = std::process::Command::new("python3")
        .arg("/home/astro/wealth-system/wealthmanager/ingest.py")
        .arg(&temp_filepath)
        .output();
        
    // Clean up temporary file immediately
    let _ = std::fs::remove_file(&temp_filepath);

    let output = output.map_err(|e| {
        crate::error::ApiError::Internal(format!("Failed to execute ingest.py: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(crate::error::ApiError::Internal(format!(
            "ingest.py failed: {}",
            stderr
        )));
    }

    let extracted_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if extracted_text.is_empty() {
        return Err(crate::error::ApiError::BadRequest(
            "No text could be extracted from the statement file.".to_string(),
        ));
    }

    // 3. Resolve active accounts context
    let accounts = state.account_service.get_all_accounts()?;
    
    let target_account_id = account_id.unwrap_or_else(|| {
        accounts
            .first()
            .map(|a| a.id.clone())
            .unwrap_or_else(|| "0e64c3da-e5f5-4ef9-a5f3-84ebcb6eaa8a".to_string())
    });

    if wealthfolio_core::importers::cams_cas::CamsCasParser::is_cams_kfintech_statement(&extracted_text) {
        let mut activities = wealthfolio_core::importers::cams_cas::CamsCasParser::parse(&extracted_text, &target_account_id).map_err(|e| {
            crate::error::ApiError::Internal(format!("Failed to parse CAMS CAS: {}", e))
        })?;

        resolve_and_auto_create_cas_assets(&mut activities, state.asset_service.as_ref()).await?;

        return Ok(Json(activities));
    }

    let templates = state.activity_service.list_import_templates()?;
    let engine = wealthfolio_core::importers::template_engine::TemplateEngine::new();
    if let Some(matched_template) = engine.match_template(&extracted_text, &templates) {
        let activities = engine.extract_with_template(&extracted_text, matched_template, &target_account_id).map_err(|e| {
            crate::error::ApiError::Internal(format!("Template extraction failed: {}", e))
        })?;
        return Ok(Json(activities));
    }

    let accounts_context = accounts
        .iter()
        .map(|a| format!("- Name: {}, ID: {}, Currency: {}", a.name, a.id, a.currency))
        .collect::<Vec<_>>()
        .join("\n");

    // 4. Query LM Studio for model dynamically
    let lm_studio_base = std::env::var("LM_STUDIO_BASE").unwrap_or_else(|_| "http://127.0.0.1:1234".to_string());
    
    // Resolve model name
    let client = reqwest::Client::new();
    let mut active_model = "qwen".to_string();
    
    if let Ok(res) = client.get(format!("{}/v1/models", lm_studio_base)).send().await {
        if let Ok(json_res) = res.json::<serde_json::Value>().await {
            if let Some(models) = json_res.get("data").and_then(|v| v.as_array()) {
                for m in models {
                    if let Some(model_id) = m.get("id").and_then(|v| v.as_str()) {
                        let lower = model_id.to_lowercase();
                        if !lower.contains("embed") && !lower.contains("rerank") {
                            active_model = model_id.to_string();
                            break;
                        }
                    }
                }
            }
        }
    }

    // 5. Construct Chat Prompt
    let prompt = format!(
        "You are a financial data extraction expert. Your task is to extract all transactions from the provided bank statement text \
        into a strict JSON format compatible with Wealthfolio.\n\n\
        ### Available Wealthfolio Accounts:\n{}\n\n\
        ### CRITICAL INSTRUCTIONS:\n\
        1. **CSV/Tabular Structure**: Pay close attention to the column headers (e.g., Date, Narration, Withdrawal, Deposit, Balance).\n   \
           - **NEVER** use the 'Balance' column as the transaction amount.\n   \
           - The transaction amount is in either the 'Withdrawal/Debit' or 'Deposit/Credit' column.\n\
        2. **Mandatory Fields for Activities**:\n   \
           - `date`: Strict ISO format (YYYY-MM-DD) for all activities.\n   \
           - `activityType`: MUST be strictly \"DEPOSIT\" or \"WITHDRAWAL\".\n   \
           - `symbol`: Use the currency code (e.g., \"USD\", \"INR\").\n   \
           - `quantity`: Always use 1.\n   \
           - `unitPrice`: The absolute amount of the transaction.\n   \
           - `currency`: The currency code.\n   \
           - `comment`: The transaction narration.\n   \
           - `accountId`: Choose the best matching Account ID from the list above.\n   \
           - `isDraft`: false.\n   \
           - `isValid`: true.\n\
        3. **Balances**: You MUST extract the final closing balance on the statement and include it as `closingBalance` (number) and `closingBalanceDate` (YYYY-MM-DD string).\n   \
           - Also extract the starting balance as `openingBalance` (number) and `openingBalanceDate` (YYYY-MM-DD string) if present.\n\n\
        ### EXPECTED JSON SCHEMA:\n\
        {{\n  \
          \"openingBalance\": 1000.00,\n  \
          \"openingBalanceDate\": \"YYYY-MM-DD\",\n  \
          \"closingBalance\": 12345.67,\n  \
          \"closingBalanceDate\": \"YYYY-MM-DD\",\n  \
          \"activities\": [\n    \
            {{\n      \
              \"date\": \"YYYY-MM-DD\",\n      \
              \"activityType\": \"DEPOSIT|WITHDRAWAL\",\n      \
              \"symbol\": \"USD\",\n      \
              \"quantity\": 1,\n      \
              \"unitPrice\": 123.45,\n      \
              \"currency\": \"USD\",\n      \
              \"comment\": \"Narration\",\n      \
              \"accountId\": \"UUID\",\n      \
              \"isDraft\": false,\n      \
              \"isValid\": true\n    \
            }}\n  \
          ]\n\
        }}\n\n\
        Return ONLY the JSON object. No preamble.",
        accounts_context
    );

    let payload = serde_json::json!({
        "model": active_model,
        "messages": [
            {"role": "system", "content": prompt},
            {"role": "user", "content": extracted_text}
        ],
        "temperature": 0.0
    });

    let mut content_opt = None;

    let llm_res_result = client
        .post(format!("{}/v1/chat/completions", lm_studio_base))
        .json(&payload)
        .send()
        .await;

    if let Ok(llm_res) = llm_res_result {
        if llm_res.status().is_success() {
            if let Ok(llm_json) = llm_res.json::<serde_json::Value>().await {
                if let Some(c) = llm_json
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|m| m.get("message"))
                    .and_then(|msg| msg.get("content"))
                    .and_then(|c| c.as_str())
                {
                    content_opt = Some(c.trim().to_string());
                }
            }
        }
    }

    // Fallback to Gemini if LM Studio failed or was unreachable
    if content_opt.is_none() {
        if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
            let gemini_url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}", api_key);
            let gemini_payload = serde_json::json!({
                "system_instruction": {
                    "parts": [{"text": prompt}]
                },
                "contents": [
                    {
                        "parts": [
                            {"text": extracted_text}
                        ]
                    }
                ],
                "generationConfig": {
                    "temperature": 0.0,
                    "response_mime_type": "application/json"
                }
            });

            if let Ok(gemini_res) = client.post(&gemini_url).json(&gemini_payload).send().await {
                if gemini_res.status().is_success() {
                    if let Ok(gemini_json) = gemini_res.json::<serde_json::Value>().await {
                        if let Some(c) = gemini_json
                            .get("candidates")
                            .and_then(|c| c.as_array())
                            .and_then(|a| a.first())
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.as_array())
                            .and_then(|a| a.first())
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            content_opt = Some(c.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    let content = content_opt.ok_or_else(|| {
        crate::error::ApiError::Internal("Failed to get a response from LM Studio and Gemini fallback".to_string())
    })?;

    // Clean JSON content by slicing block boundaries
    let clean_json = if let Some(start_idx) = content.find('{').or_else(|| content.find('[')) {
        if let Some(end_idx) = content.rfind('}').or_else(|| content.rfind(']')) {
            if end_idx >= start_idx {
                content[start_idx..=end_idx].to_string()
            } else {
                content.to_string()
            }
        } else {
            content.to_string()
        }
    } else {
        content.to_string()
    };

    let llm_output: LlmOutput = serde_json::from_str(&clean_json).map_err(|e| {
        crate::error::ApiError::Internal(format!("Failed to parse AI output into strict schema: {}. Raw output: {}", e, content))
    })?;

    // 6. Map parsed activities to ActivityImport structure
    let mut import_activities = Vec::new();
    let mut net_change = rust_decimal::Decimal::ZERO;

    for act in llm_output.activities {
        let amount_val = act.unit_price.unwrap_or(rust_decimal::Decimal::ZERO);
        if act.activity_type == "DEPOSIT" {
            net_change += amount_val;
        } else {
            net_change -= amount_val;
        }

        import_activities.push(ActivityImport {
            id: None,
            date: act.date,
            symbol: act.symbol,
            activity_type: act.activity_type,
            quantity: act.quantity.or(Some(rust_decimal::Decimal::ONE)),
            unit_price: act.unit_price,
            currency: act.currency,
            fee: Some(rust_decimal::Decimal::ZERO),
            amount: act.unit_price,
            comment: act.comment,
            account_id: Some(target_account_id.clone()),
            account_name: None,
            symbol_name: None,
            exchange_mic: None,
            quote_ccy: None,
            instrument_type: None,
            quote_mode: None,
            errors: None,
            warnings: None,
            duplicate_of_id: None,
            duplicate_of_line_number: None,
            is_draft: true,
            is_valid: true,
            line_number: None,
            fx_rate: None,
            subtype: None,
            force_import: false,
            asset_id: None,
            isin: None,
        });
    }

    // 7. Inject Opening Balance transaction if closingBalance is provided
    inject_opening_balance_activity(
        &mut import_activities,
        llm_output.closing_balance,
        llm_output.opening_balance,
        llm_output.opening_balance_date,
        net_change,
        &target_account_id,
    );

    // 8. Save template signature for future matches
    let new_template = wealthfolio_core::activities::ImportTemplateData {
        id: format!("tmpl-{}", uuid::Uuid::new_v4()),
        name: "Auto AI Template".to_string(),
        scope: wealthfolio_core::activities::ImportTemplateScope::User,
        kind: wealthfolio_core::activities::TemplateKind::CsvActivity,
        field_mappings: Default::default(),
        activity_mappings: Default::default(),
        symbol_mappings: Default::default(),
        account_mappings: Default::default(),
        symbol_mapping_meta: Default::default(),
        parse_config: None,
        template_signature: Some(engine.extract_signature(&extracted_text)),
    };
    let _ = state.activity_service.save_import_template(new_template).await;

    Ok(Json(import_activities))
}

pub async fn resolve_and_auto_create_cas_assets(
    activities: &mut Vec<wealthfolio_core::activities::ActivityImport>,
    asset_service: &dyn wealthfolio_core::assets::AssetServiceTrait,
) -> crate::error::ApiResult<()> {
    let existing_assets = asset_service.get_assets().map_err(|e| {
        crate::error::ApiError::Internal(format!("Failed to get assets: {}", e))
    })?;

    let mut symbol_to_asset_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for act in activities {
        let symbol_key = act.symbol.clone();

        if let Some(asset_id) = symbol_to_asset_id.get(&symbol_key) {
            act.asset_id = Some(asset_id.clone());
            continue;
        }

        let matched_asset = existing_assets.iter().find(|a| {
            let a_symbol = a.instrument_symbol.as_deref().unwrap_or("");
            let a_name = a.name.as_deref().unwrap_or("");
            let act_name = act.symbol_name.as_deref().unwrap_or("");
            let act_isin = act.isin.as_deref().unwrap_or("");

            a_symbol == act.symbol ||
            (!act_isin.is_empty() && a_symbol == act_isin) ||
            (!act_name.is_empty() && a_name == act_name) ||
            (a.display_code.as_deref().unwrap_or("") == act.symbol)
        });

        if let Some(asset) = matched_asset {
            act.asset_id = Some(asset.id.clone());
            symbol_to_asset_id.insert(symbol_key, asset.id.clone());
        } else {
            let new_asset = wealthfolio_core::assets::NewAsset {
                id: None,
                kind: wealthfolio_core::assets::AssetKind::Investment,
                name: act.symbol_name.clone().or_else(|| Some(act.symbol.clone())),
                display_code: act.isin.clone().or_else(|| Some(act.symbol.clone())),
                is_active: true,
                quote_mode: wealthfolio_core::assets::QuoteMode::Market,
                quote_ccy: "INR".to_string(),
                instrument_type: Some(wealthfolio_core::assets::InstrumentType::Equity),
                instrument_symbol: Some(act.isin.clone().unwrap_or_else(|| act.symbol.clone())),
                instrument_exchange_mic: None,
                provider_config: None,
                notes: None,
                metadata: None,
            };

            if let Ok(created) = asset_service.create_asset(new_asset).await {
                act.asset_id = Some(created.id.clone());
                symbol_to_asset_id.insert(symbol_key, created.id.clone());
            }
        }
    }
    Ok(())
}

pub fn inject_opening_balance_activity(
    import_activities: &mut Vec<ActivityImport>,
    closing_balance: Option<rust_decimal::Decimal>,
    opening_balance: Option<rust_decimal::Decimal>,
    opening_balance_date: Option<String>,
    net_change: rust_decimal::Decimal,
    target_account_id: &str,
) {
    if let Some(closing_balance) = closing_balance {
        let opening_balance = opening_balance.unwrap_or_else(|| closing_balance - net_change);
        
        let mut final_date = opening_balance_date.clone();
        if final_date.is_none() {
            let earliest_date_str = import_activities
                .iter()
                .map(|a| a.date.as_str())
                .min()
                .unwrap_or("2000-01-01");
                
            if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(earliest_date_str, "%Y-%m-%d") {
                let prev_day = parsed_date - chrono::Duration::try_days(1).unwrap_or(chrono::Duration::days(1));
                final_date = Some(prev_day.format("%Y-%m-%d").to_string());
            } else {
                final_date = Some(earliest_date_str.to_string());
            }
        }
        let date_to_use = final_date.unwrap_or_else(|| "2000-01-01".to_string());
        
        let currency = import_activities.first().map(|a| a.currency.clone()).unwrap_or_else(|| "USD".to_string());

        import_activities.insert(0, ActivityImport {
            id: None,
            date: date_to_use,
            symbol: currency.clone(),
            activity_type: if opening_balance >= rust_decimal::Decimal::ZERO {
                "DEPOSIT".to_string()
            } else {
                "WITHDRAWAL".to_string()
            },
            quantity: Some(rust_decimal::Decimal::ONE),
            unit_price: Some(opening_balance.abs()),
            currency: currency.clone(),
            fee: Some(rust_decimal::Decimal::ZERO),
            amount: Some(opening_balance.abs()),
            comment: Some("Opening Balance auto-injected from statement".to_string()),
            account_id: Some(target_account_id.to_string()),
            account_name: None,
            symbol_name: None,
            exchange_mic: None,
            quote_ccy: None,
            instrument_type: None,
            quote_mode: None,
            errors: None,
            warnings: None,
            duplicate_of_id: None,
            duplicate_of_line_number: None,
            is_draft: true,
            is_valid: true,
            line_number: None,
            fx_rate: None,
            subtype: None,
            force_import: false,
            asset_id: None,
            isin: None,
        });
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/activities/search", post(search_activities))
        .route("/activities", post(create_activity).put(update_activity))
        .route("/activities/bulk", post(save_activities))
        .route("/activities/{id}", delete(delete_activity))
        .route("/activities/import/check", post(check_activities_import))
        .route(
            "/activities/import/assets/preview",
            post(preview_import_assets),
        )
        .route("/activities/import", post(import_activities))
        .route("/activities/import/parse", post(parse_csv_endpoint))
        .route("/activities/import/statement", post(import_statement_endpoint))
        .route(
            "/activities/import/mapping",
            get(get_account_import_mapping).post(save_account_import_mapping),
        )
        .route(
            "/activities/import/templates",
            get(list_import_templates)
                .post(save_import_template)
                .delete(delete_import_template),
        )
        .route(
            "/activities/import/templates/item",
            get(get_import_template),
        )
        .route(
            "/activities/import/templates/link",
            post(link_account_template),
        )
        .route(
            "/activities/import/check-duplicates",
            post(check_existing_duplicates),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_inject_opening_balance_computation() {
        let mut activities = vec![];
        let closing_balance = Some(rust_decimal::Decimal::new(100, 0));
        let net_change = rust_decimal::Decimal::new(20, 0); // deposits - withdrawals
        
        inject_opening_balance_activity(&mut activities, closing_balance, None, None, net_change, "acc-1");
        
        assert_eq!(activities.len(), 1);
        let injected = &activities[0];
        assert_eq!(injected.activity_type, "DEPOSIT");
        assert_eq!(injected.amount, Some(rust_decimal::Decimal::new(80, 0))); // 100 - 20 = 80
        assert_eq!(injected.comment.as_deref(), Some("Opening Balance auto-injected from statement"));
    }

    #[tokio::test]
    async fn test_resolve_and_auto_create_cas_assets() {
        struct MockAssetService {
            assets: std::sync::Mutex<Vec<wealthfolio_core::assets::Asset>>,
        }
        
        #[async_trait::async_trait]
        impl wealthfolio_core::assets::AssetServiceTrait for MockAssetService {
            fn get_assets(&self) -> wealthfolio_core::errors::Result<Vec<wealthfolio_core::assets::Asset>> {
                Ok(self.assets.lock().unwrap().clone())
            }
            
            async fn create_asset(&self, new_asset: wealthfolio_core::assets::NewAsset) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::Asset> {
                let asset = wealthfolio_core::assets::Asset {
                    id: "new-id-123".to_string(),
                    instrument_symbol: new_asset.instrument_symbol.clone(),
                    name: new_asset.name.clone(),
                    ..Default::default()
                };
                self.assets.lock().unwrap().push(asset.clone());
                Ok(asset)
            }
            
            // Dummy implementations for other methods
            fn get_asset_by_id(&self, _id: &str) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::Asset> { unimplemented!() }
            async fn delete_asset(&self, _id: &str) -> wealthfolio_core::errors::Result<()> { unimplemented!() }
            async fn update_asset_profile(&self, _id: &str, _payload: wealthfolio_core::assets::UpdateAssetProfile) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::Asset> { unimplemented!() }
            async fn get_or_create_minimal_asset(&self, _id: &str, _ccy: Option<String>, _m: Option<wealthfolio_core::assets::AssetMetadata>, _qm: Option<String>) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::Asset> { unimplemented!() }
            async fn update_quote_mode(&self, _id: &str, _qm: &str) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::Asset> { unimplemented!() }
            async fn get_assets_by_asset_ids(&self, _ids: &[String]) -> wealthfolio_core::errors::Result<Vec<wealthfolio_core::assets::Asset>> { unimplemented!() }
            async fn enrich_asset_profile(&self, _id: &str) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::Asset> { unimplemented!() }
            async fn enrich_assets(&self, _ids: Vec<String>) -> wealthfolio_core::errors::Result<(usize, usize, usize)> { unimplemented!() }
            async fn cleanup_legacy_metadata(&self, _id: &str) -> wealthfolio_core::errors::Result<()> { unimplemented!() }
            async fn merge_unknown_asset(&self, _r: &str, _u: &str, _repo: &dyn wealthfolio_core::activities::ActivityRepositoryTrait) -> wealthfolio_core::errors::Result<u32> { unimplemented!() }
            async fn ensure_assets(&self, _specs: Vec<wealthfolio_core::assets::AssetSpec>, _repo: &dyn wealthfolio_core::activities::ActivityRepositoryTrait) -> wealthfolio_core::errors::Result<wealthfolio_core::assets::EnsureAssetsResult> { unimplemented!() }
        }

        let service = MockAssetService { assets: std::sync::Mutex::new(vec![]) };
        let mut activities = vec![
            wealthfolio_core::activities::ActivityImport {
                symbol: "HDFC-FLEXI".to_string(),
                symbol_name: Some("HDFC Flexi Cap Fund".to_string()),
                isin: Some("INF123456789".to_string()),
                ..Default::default()
            }
        ];

        let res = resolve_and_auto_create_cas_assets(&mut activities, &service).await;
        assert!(res.is_ok());

        assert_eq!(activities[0].asset_id, Some("new-id-123".to_string()));
        let assets = service.assets.lock().unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name.as_deref(), Some("HDFC Flexi Cap Fund"));
    }

    #[test]
    fn test_inject_opening_balance_provided() {
        let mut activities = vec![];
        let closing_balance = Some(rust_decimal::Decimal::new(100, 0));
        let opening_balance = Some(rust_decimal::Decimal::new(50, 0));
        let net_change = rust_decimal::Decimal::new(50, 0); 
        
        inject_opening_balance_activity(&mut activities, closing_balance, opening_balance, None, net_change, "acc-1");
        
        assert_eq!(activities.len(), 1);
        let injected = &activities[0];
        assert_eq!(injected.activity_type, "DEPOSIT");
        assert_eq!(injected.amount, Some(rust_decimal::Decimal::new(50, 0))); 
    }

    #[test]
    fn test_inject_opening_balance_date_fallback() {
        let mut activities = vec![
            ActivityImport {
                id: None,
                date: "2026-07-20".to_string(),
                symbol: "USD".to_string(),
                activity_type: "DEPOSIT".to_string(),
                quantity: None,
                unit_price: None,
                currency: "USD".to_string(),
                fee: None,
                amount: None,
                comment: None,
                account_id: None,
                account_name: None,
                symbol_name: None,
                exchange_mic: None,
                quote_ccy: None,
                instrument_type: None,
                quote_mode: None,
                errors: None,
                warnings: None,
                duplicate_of_id: None,
                duplicate_of_line_number: None,
                is_draft: false,
                is_valid: true,
                line_number: None,
                fx_rate: None,
                subtype: None,
                force_import: false,
                asset_id: None,
                isin: None,
            }
        ];
        let closing_balance = Some(rust_decimal::Decimal::new(100, 0));
        
        inject_opening_balance_activity(&mut activities, closing_balance, Some(rust_decimal::Decimal::new(50, 0)), None, rust_decimal::Decimal::new(0, 0), "acc-1");
        
        assert_eq!(activities.len(), 2);
        let injected = &activities[0]; // because insert(0)
        assert_eq!(injected.date, "2026-07-19");
    }
}
