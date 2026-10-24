//! Auto-Ingestion Engine for Automatic Folder Scanning
//!
//! Automatically scans an `inputs/` folder, detects statement types (.txt CAS, .csv broker/credit card,
//! .xls/.xlsx Excel bank statements, .json portfolio files), parses them into structured `ActivityImport`
//! records, and returns an ingestion report.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::activities::ActivityImport;
use crate::importers::broker_csv::{BrokerCsvParser, BrokerType};
use crate::importers::cams_cas::CamsCasParser;
use crate::importers::credit_card::CreditCardParser;
use crate::importers::excel_parser::ExcelStatementParser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIngestResult {
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub activity_count: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoIngestSummary {
    pub inputs_dir: String,
    pub total_files_found: usize,
    pub total_activities_imported: usize,
    pub file_results: Vec<FileIngestResult>,
    pub activities: Vec<ActivityImport>,
}

pub struct AutoIngestEngine;

impl AutoIngestEngine {
    /// Scans the target inputs directory, detects all files, and automatically parses them.
    pub fn scan_and_ingest_folder(inputs_dir: &Path, account_id: &str) -> Result<AutoIngestSummary, String> {
        if !inputs_dir.exists() {
            fs::create_dir_all(inputs_dir)
                .map_err(|e| format!("Failed to create inputs directory {}: {}", inputs_dir.display(), e))?;
        }

        let entries = fs::read_dir(inputs_dir)
            .map_err(|e| format!("Failed to read inputs directory {}: {}", inputs_dir.display(), e))?;

        let mut file_paths = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                file_paths.push(path);
            }
        }

        file_paths.sort();

        let mut file_results = Vec::new();
        let mut all_activities = Vec::new();
        let mut total_activities = 0;

        for path in &file_paths {
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();

            match Self::ingest_single_file(path, &ext, account_id) {
                Ok((file_type, activities)) => {
                    let count = activities.len();
                    total_activities += count;
                    all_activities.extend(activities);

                    file_results.push(FileIngestResult {
                        file_path: path.to_string_lossy().to_string(),
                        file_name: filename,
                        file_type,
                        activity_count: count,
                        success: true,
                        error_message: None,
                    });
                }
                Err(err) => {
                    file_results.push(FileIngestResult {
                        file_path: path.to_string_lossy().to_string(),
                        file_name: filename,
                        file_type: ext,
                        activity_count: 0,
                        success: false,
                        error_message: Some(err),
                    });
                }
            }
        }

        Ok(AutoIngestSummary {
            inputs_dir: inputs_dir.to_string_lossy().to_string(),
            total_files_found: file_paths.len(),
            total_activities_imported: total_activities,
            file_results,
            activities: all_activities,
        })
    }

    /// Automatically ingests a single file based on its extension & content signatures.
    pub fn ingest_single_file(
        path: &Path,
        ext: &str,
        account_id: &str,
    ) -> Result<(String, Vec<ActivityImport>), String> {
        match ext {
            "txt" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read txt file: {}", e))?;
                let activities = CamsCasParser::parse(&content, account_id)?;
                Ok(("CAMS/KFintech CAS Text".to_string(), activities))
            }
            "csv" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read csv file: {}", e))?;
                let broker_type = BrokerCsvParser::detect_broker(&content);

                match broker_type {
                    BrokerType::Zerodha | BrokerType::Groww | BrokerType::AngelOne | BrokerType::Upstox => {
                        let activities = BrokerCsvParser::parse(&content, account_id)?;
                        Ok((format!("Broker CSV ({:?})", broker_type), activities))
                    }
                    BrokerType::Unknown => {
                        // Fallback to Credit Card parser
                        let activities = CreditCardParser::parse(&content, account_id)?;
                        Ok(("Credit Card CSV".to_string(), activities))
                    }
                }
            }
            "xls" | "xlsx" | "xlsb" | "ods" => {
                let bytes = fs::read(path)
                    .map_err(|e| format!("Failed to read excel file bytes: {}", e))?;
                let tsv = ExcelStatementParser::convert_excel_to_tsv(&bytes)?;
                let activities = ExcelStatementParser::parse_excel_tsv(&tsv, account_id)?;
                Ok(("Excel Bank Statement".to_string(), activities))
            }
            "json" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read json file: {}", e))?;
                // Parse custom portfolio JSON or activity array
                if let Ok(activities) = serde_json::from_str::<Vec<ActivityImport>>(&content) {
                    Ok(("JSON Activity List".to_string(), activities))
                } else {
                    Err("JSON format not matched to ActivityImport array".to_string())
                }
            }
            _ => Err(format!("Unsupported file extension '.{}'", ext)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_and_ingest_folder() {
        let dir = tempdir().unwrap();
        let inputs_path = dir.path();

        // Create sample CAS file
        let cas_path = inputs_path.join("cams.txt");
        fs::write(
            &cas_path,
            "01-Jan-2025  PURCHASE     25000.00        312.500     80.0000       312.500\n",
        )
        .unwrap();

        // Create sample Zerodha CSV
        let broker_path = inputs_path.join("tradebook.csv");
        fs::write(
            &broker_path,
            "symbol,isin,trade_date,exchange,segment,trade_type,quantity,price,trade_value,brokerage,stt\nRELIANCE,INE002A01018,2024-05-10,NSE,EQ,buy,50,2450.00,122500.00,20.00,122.50\n",
        )
        .unwrap();

        let summary = AutoIngestEngine::scan_and_ingest_folder(inputs_path, "ACC-TEST").unwrap();
        assert_eq!(summary.total_files_found, 2);
        assert_eq!(summary.total_activities_imported, 2);
        assert_eq!(summary.file_results.len(), 2);
        assert!(summary.file_results.iter().all(|r| r.success));
    }
}
