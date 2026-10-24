//! Excel Bank Statement Parser (.xls, .xlsx, .xlsb, .ods)
//!
//! Provides conversion from Excel spreadsheet workbooks into tabular text / TSV format
//! for the AI Model & Template Engine, as well as native transaction extraction.

use calamine::{open_workbook_auto, Reader};
use rust_decimal::Decimal;
use std::io::Write;

use crate::activities::ActivityImport;

pub struct ExcelStatementParser;

impl ExcelStatementParser {
    /// Reads an Excel file (.xls, .xlsx, .xlsb) from bytes and converts all sheets
    /// into tab-separated text suitable for AI model formatting and template matching.
    pub fn convert_excel_to_tsv(bytes: &[u8]) -> Result<String, String> {
        let mut temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| format!("Failed to create temporary file: {}", e))?;
        temp_file
            .write_all(bytes)
            .map_err(|e| format!("Failed to write bytes to temp file: {}", e))?;

        let path = temp_file.path();
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| format!("Failed to open Excel workbook: {}", e))?;

        let mut output = String::new();

        for sheet_name in workbook.sheet_names().to_vec() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                output.push_str(&format!("--- Sheet: {} ---\n", sheet_name));
                for row in range.rows() {
                    let line: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
                    output.push_str(&line.join("\t"));
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }

    /// Parses tabular text extracted from an Excel statement into structured ActivityImports.
    pub fn parse_excel_tsv(tsv_text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let mut activities = Vec::new();
        let lines: Vec<&str> = tsv_text.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let cols: Vec<&str> = line.split('\t').map(|s| s.trim()).collect();
            if cols.len() < 3 {
                continue;
            }

            // Heuristic to detect transaction row: Date, Description, Amount
            if let Some(date) = chrono::NaiveDate::parse_from_str(cols[0], "%Y-%m-%d")
                .or_else(|_| chrono::NaiveDate::parse_from_str(cols[0], "%d-%m-%Y"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(cols[0], "%d/%m/%Y"))
                .ok()
            {
                let description = cols[1].to_string();
                let amount_str = cols[2].replace(',', "");

                if let Ok(amount) = amount_str.parse::<Decimal>() {
                    let activity_type = if amount < Decimal::ZERO {
                        "SELL".to_string()
                    } else {
                        "BUY".to_string()
                    };

                    activities.push(ActivityImport {
                        id: Some(format!("excel:{}", uuid::Uuid::new_v4())),
                        date: date.to_string(),
                        symbol: description.clone(),
                        activity_type,
                        quantity: Some(Decimal::ONE),
                        unit_price: Some(amount.abs()),
                        amount: Some(amount.abs()),
                        currency: "INR".to_string(),
                        fee: None,
                        comment: Some(description),
                        account_id: Some(account_id.to_string()),
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
                        line_number: Some((i + 1) as i32),
                        fx_rate: None,
                        subtype: None,
                        asset_id: None,
                        isin: None,
                        force_import: false,
                    });
                }
            }
        }

        Ok(activities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_excel_tsv() {
        let tsv = "Date\tDescription\tAmount\n2025-01-15\tSALARY CREDIT\t75000.00\n2025-01-16\tZOMATO ORDER\t-450.00\n";
        let res = ExcelStatementParser::parse_excel_tsv(tsv, "ACC-001").unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].amount, Some(Decimal::from(75000)));
        assert_eq!(res[1].amount, Some(Decimal::from(450)));
    }
}
