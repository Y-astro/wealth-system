//! Read-only statement analysis. Python owns extraction; the app owns chart rendering.
use std::{
    path::PathBuf,
    process::Stdio,
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::{io::AsyncWriteExt, process::Command, sync::Semaphore};

use crate::{
    error::{ApiError, ApiResult},
    main_lib::AppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRequest {
    text: String,
    filename: String,
    #[serde(default = "default_use_ai")]
    use_ai: bool,
}

fn default_use_ai() -> bool {
    true
}

fn analyzer_path() -> PathBuf {
    std::env::var_os("WEALTH_STATEMENT_ANALYZER")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../wealthmanager/analyze_statement.py")
        })
}

pub(super) async fn run_analyzer(payload: Value) -> ApiResult<Value> {
    let use_ai = payload
        .get("useAi")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut offline_payload = payload.clone();
    offline_payload["useAi"] = Value::Bool(false);
    match run_worker(offline_payload).await {
        Ok(report) => return Ok(report),
        Err(ApiError::BadRequest(message))
            if use_ai && message.starts_with("No bank rows could be mapped.") => {}
        Err(error) => return Err(error),
    }
    // Only GPU inference is serialized; offline NSDL/table parsing stays available.
    static WORKER: OnceLock<Semaphore> = OnceLock::new();
    // Fail quickly instead of piling up GPU inference requests.
    let _permit = WORKER
        .get_or_init(|| Semaphore::new(1))
        .try_acquire()
        .map_err(|_| {
            ApiError::BadRequest(
                "Another statement is being analyzed. Wait for it to finish.".into(),
            )
        })?;
    run_worker(payload).await
}

async fn run_worker(payload: Value) -> ApiResult<Value> {
    let mut child = Command::new(std::env::var("WEALTH_PYTHON").unwrap_or_else(|_| "python3".into()))
        .arg(analyzer_path())
        .arg("--stdin-json")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn().map_err(|_| ApiError::Internal("Cannot start statement analyzer. Set WEALTH_STATEMENT_ANALYZER and WEALTH_PYTHON for this installation.".into()))?;
    let input = serde_json::to_vec(&payload)
        .map_err(|_| ApiError::BadRequest("Invalid statement input.".into()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ApiError::Internal("Statement analyzer input unavailable.".into()))?;
    // Write concurrently with waiting so large pipes cannot deadlock.
    let work = async {
        let writer = async move {
            stdin.write_all(&input).await?;
            stdin.shutdown().await
        };
        let (written, output) = tokio::join!(writer, child.wait_with_output());
        written.map_err(|_| ApiError::Internal("Could not send statement to analyzer.".into()))?;
        let output = output.map_err(|_| ApiError::Internal("Statement analyzer failed.".into()))?;
        if !output.status.success() {
            // The CLI emits one sanitized error object, never the raw statement/model output.
            let error = serde_json::from_slice::<Value>(&output.stderr)
                .ok()
                .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_else(|| {
                    "Statement analysis failed. Check the local analyzer installation.".into()
                });
            return Err(ApiError::BadRequest(error));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|_| ApiError::Internal("Analyzer returned an invalid report.".into()))
    };
    tokio::time::timeout(Duration::from_secs(240), work)
        .await
        .map_err(|_| {
            ApiError::BadRequest(
                "Statement analysis exceeded four minutes. Split the file into smaller statements."
                    .into(),
            )
        })?
}

async fn create_analysis(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<AnalysisRequest>,
) -> ApiResult<Json<Value>> {
    if body.text.trim().is_empty() || body.text.len() > 2_000_000 {
        return Err(ApiError::BadRequest(
            "Provide a nonempty statement of at most 2 MB.".into(),
        ));
    }
    let report = run_analyzer(
        serde_json::json!({"text": body.text, "filename": body.filename, "useAi": body.use_ai}),
    )
    .await?;
    Ok(Json(report))
}

async fn current_model(State(_state): State<Arc<AppState>>) -> ApiResult<Json<Value>> {
    let output = tokio::time::timeout(
        Duration::from_secs(20),
        Command::new(std::env::var("WEALTH_PYTHON").unwrap_or_else(|_| "python3".into()))
            .arg(analyzer_path())
            .arg("--model")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| ApiError::BadRequest("LM Studio model discovery timed out.".into()))?
    .map_err(|_| ApiError::Internal("Cannot start statement analyzer.".into()))?;
    if !output.status.success() {
        let message = serde_json::from_slice::<Value>(&output.stderr)
            .ok()
            .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned))
            .unwrap_or_else(|| "Local model discovery failed.".into());
        return Err(ApiError::BadRequest(message));
    }
    Ok(Json(serde_json::from_slice(&output.stdout).map_err(
        |_| ApiError::Internal("Invalid local model status.".into()),
    )?))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/statements/library/{id}/retry",
            post(retry_library_statement),
        )
        .route("/statements/analyses", post(create_analysis))
        .route("/statements/model", get(current_model))
        .route(
            "/statements/library",
            get(list_library).post(upload_statement),
        )
        .route(
            "/statements/library/{id}",
            axum::routing::put(save_library_report).delete(archive_library_report),
        )
        .layer(DefaultBodyLimit::max(51_000_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shared_analyzer_maps_holdings_bank_cash_and_invalid_input() {
        let report = run_analyzer(serde_json::json!({
            "text": include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../wealthmanager/tests/fixtures/nsdl_values.md")),
            "filename": "nsdl_values.md", "useAi": false
        })).await.unwrap();
        assert_eq!(report["kind"], "investment");
        assert_eq!(report["holdings"].as_array().unwrap().len(), 58);
        assert_eq!(report["summary"]["difference"], 0.0);
        let bank = run_analyzer(serde_json::json!({
            "text": include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../wealthmanager/tests/fixtures/bank_statement.md")),
            "filename": "bank.md", "useAi": false
        })).await.unwrap();
        assert_eq!(bank["summary"]["income"], 81200.0);
        assert_eq!(bank["summary"]["expenses"], 27500.0);
        assert_eq!(bank["summary"]["balanceDifference"], 0.0);
        let (first, second) = tokio::join!(
            run_analyzer(
                serde_json::json!({"text": include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../wealthmanager/tests/fixtures/bank_statement.md")), "filename": "one.md", "useAi": false})
            ),
            run_analyzer(
                serde_json::json!({"text": include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../wealthmanager/tests/fixtures/bank_statement.md")), "filename": "two.md", "useAi": false})
            )
        );
        assert!(
            first.is_ok() && second.is_ok(),
            "offline parsing must not share the inference lock"
        );
        let invalid = run_analyzer(serde_json::json!({"text": "unrecognized format", "filename": "bank.md", "useAi": false})).await;
        assert!(matches!(invalid, Err(ApiError::BadRequest(_))));
    }
}

// Durable upload library. Conversion workers outlive the browser request.
async fn library_command(payload: Value) -> ApiResult<Value> {
    let mut child =
        Command::new(std::env::var("WEALTH_PYTHON").unwrap_or_else(|_| "python3".into()))
            .arg(analyzer_path().with_file_name("statement_library.py"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| ApiError::Internal("Cannot start the statement library.".into()))?;
    let input = serde_json::to_vec(&payload)
        .map_err(|_| ApiError::BadRequest("Invalid library request.".into()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ApiError::Internal("Cannot open statement library input.".into()))?;
    let work = async {
        let writer = tokio::spawn(async move {
            stdin.write_all(&input).await?;
            stdin.shutdown().await
        });
        let (written, output) = tokio::join!(writer, child.wait_with_output());
        written
            .map_err(|_| ApiError::Internal("Statement library writer failed.".into()))?
            .map_err(|_| ApiError::Internal("Cannot send statement library input.".into()))?;
        let output = output.map_err(|_| ApiError::Internal("Statement library failed.".into()))?;
        if !output.status.success() {
            let message = serde_json::from_slice::<Value>(&output.stderr)
                .ok()
                .and_then(|value| value["error"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "Statement library request failed.".into());
            return Err(ApiError::BadRequest(message));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|_| ApiError::Internal("Invalid statement library response.".into()))
    };
    tokio::time::timeout(Duration::from_secs(30), work)
        .await
        .map_err(|_| ApiError::BadRequest("Statement upload timed out. Please try again.".into()))?
}

async fn list_library() -> ApiResult<Json<Value>> {
    Ok(Json(
        library_command(serde_json::json!({"action":"list"})).await?,
    ))
}

async fn upload_statement(mut multipart: axum::extract::Multipart) -> ApiResult<Json<Value>> {
    // A guard removes the temporary original after Python durably saves it,
    // including disconnects, timeouts and rejected files.
    struct UploadPath(PathBuf);
    impl Drop for UploadPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let mut field = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("Cannot read statement upload.".into()))?
        .ok_or_else(|| ApiError::BadRequest("Choose a statement file.".into()))?;
    let filename = field.file_name().unwrap_or("statement.txt").to_owned();
    let path =
        UploadPath(std::env::temp_dir().join(format!("wealth-upload-{}", uuid::Uuid::new_v4())));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&path.0)
        .map_err(|_| ApiError::Internal("Cannot create statement upload buffer.".into()))?;
    let mut file = tokio::fs::File::from_std(file);
    let mut size = 0usize;
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|_| ApiError::BadRequest("Statement must be at most 50 MB.".into()))?
    {
        size += chunk.len();
        if size > 50_000_000 {
            return Err(ApiError::BadRequest(
                "Choose a statement up to 50 MB.".into(),
            ));
        }
        file.write_all(&chunk)
            .await
            .map_err(|_| ApiError::Internal("Cannot buffer statement upload.".into()))?;
    }
    if size == 0 {
        return Err(ApiError::BadRequest("Choose a nonempty statement.".into()));
    }
    file.flush()
        .await
        .map_err(|_| ApiError::Internal("Cannot save statement upload buffer.".into()))?;
    drop(file);
    let result = library_command(serde_json::json!({
        "action":"upload", "filename":filename, "uploadPath":path.0
    }))
    .await?;
    Ok(Json(result))
}

async fn save_library_report(
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(report): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        library_command(serde_json::json!({"action":"save", "id":id, "report":report})).await?,
    ))
}

async fn archive_library_report(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        library_command(serde_json::json!({"action":"archive", "id":id})).await?,
    ))
}

async fn retry_library_statement(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        library_command(serde_json::json!({"action":"retry", "id":id})).await?,
    ))
}
