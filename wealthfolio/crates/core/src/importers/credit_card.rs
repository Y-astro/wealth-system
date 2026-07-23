use std::collections::HashMap;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::activities::ActivityImport;

pub struct CreditCardParser;

impl CreditCardParser {
    pub fn parse(csv_text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(csv_text.as_bytes());
        let mut activities = Vec::new();

        let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
        let header_map: HashMap<String, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.trim().to_lowercase(), i))
            .collect();

        let date_idx = header_map.get("date");
        let narration_idx = header_map.get("narration")
            .or_else(|| header_map.get("merchant"))
            .or_else(|| header_map.get("description"));
        let amount_idx = header_map.get("amount");
        let type_idx = header_map.get("type").or_else(|| header_map.get("debit/credit"));
        let debit_idx = header_map.get("debit");
        let credit_idx = header_map.get("credit");

        if date_idx.is_none() || narration_idx.is_none() {
            return Err("Missing required columns: date and narration/merchant".to_string());
        }

        let date_idx = *date_idx.unwrap();
        let narration_idx = *narration_idx.unwrap();

        for (i, result) in rdr.records().enumerate() {
            let record = result.map_err(|e| format!("Row {}: {}", i + 2, e))?;
            
            let date_str = record.get(date_idx).unwrap_or("").trim();
            let narration = record.get(narration_idx).unwrap_or("").trim().to_string();
            
            let mut amount: Option<Decimal> = None;
            let mut activity_type = "WITHDRAWAL".to_string();
            
            if let Some(&idx) = type_idx {
                let t = record.get(idx).unwrap_or("").trim().to_lowercase();
                if t == "cr" || t == "credit" || t == "deposit" {
                    activity_type = "DEPOSIT".to_string();
                }
                if let Some(&amt_i) = amount_idx {
                    amount = Decimal::from_str(&record.get(amt_i).unwrap_or("").replace(",", "")).ok();
                }
            } else if let Some(&d_idx) = debit_idx {
                let d_val = record.get(d_idx).unwrap_or("").trim();
                if !d_val.is_empty() {
                    amount = Decimal::from_str(&d_val.replace(",", "")).ok();
                    activity_type = "WITHDRAWAL".to_string();
                } else if let Some(&c_idx) = credit_idx {
                    let c_val = record.get(c_idx).unwrap_or("").trim();
                    if !c_val.is_empty() {
                        amount = Decimal::from_str(&c_val.replace(",", "")).ok();
                        activity_type = "DEPOSIT".to_string();
                    }
                }
            } else if let Some(&amt_i) = amount_idx {
                if let Ok(amt) = Decimal::from_str(&record.get(amt_i).unwrap_or("").replace(",", "")) {
                    if amt < Decimal::ZERO {
                        activity_type = "DEPOSIT".to_string();
                        amount = Some(-amt);
                    } else {
                        amount = Some(amt);
                    }
                }
            }

            let date = Self::parse_date(date_str).unwrap_or_else(|| date_str.to_string());
            
            let hint = Self::suggest_category(&narration);

            let import = ActivityImport {
                id: None,
                date,
                symbol: narration.clone(),
                activity_type,
                quantity: Some(Decimal::ONE),
                unit_price: amount,
                currency: "INR".to_string(), // Default currency
                fee: None,
                amount,
                comment: Some(format!("Hint: {}", hint)),
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
                line_number: Some((i + 2) as i32),
                fx_rate: None,
                subtype: None,
                asset_id: None,
                isin: None,
                force_import: false,
            };
            activities.push(import);
        }

        Ok(activities)
    }

    fn parse_date(date_str: &str) -> Option<String> {
        let formats = [
            "%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d %b %Y", "%Y/%m/%d"
        ];
        
        for format in formats.iter() {
            if let Ok(d) = NaiveDate::parse_from_str(date_str, format) {
                return Some(d.format("%Y-%m-%d").to_string());
            }
        }
        None
    }
    
    fn suggest_category(narration: &str) -> String {
        let n = narration.to_lowercase();
        if n.contains("amazon") || n.contains("flipkart") {
            "Shopping".to_string()
        } else if n.contains("zomato") || n.contains("swiggy") || n.contains("restaurant") {
            "Food & Dining".to_string()
        } else if n.contains("uber") || n.contains("ola") {
            "Travel".to_string()
        } else if n.contains("netflix") || n.contains("spotify") {
            "Entertainment".to_string()
        } else {
            "General".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credit_card() {
        let csv = "Date,Narration,Amount,Type\n2023-10-10,Amazon Order,500,DR\n2023-10-12,Payment,1000,CR";
        let res = CreditCardParser::parse(csv, "acc1").unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].activity_type, "WITHDRAWAL");
        assert_eq!(res[0].symbol, "Amazon Order");
        assert_eq!(res[0].amount, Some(Decimal::from_str("500").unwrap()));
        assert_eq!(res[0].comment.as_ref().unwrap(), "Hint: Shopping");

        assert_eq!(res[1].activity_type, "DEPOSIT");
        assert_eq!(res[1].symbol, "Payment");
        assert_eq!(res[1].amount, Some(Decimal::from_str("1000").unwrap()));
        assert_eq!(res[1].comment.as_ref().unwrap(), "Hint: General");
    }
}
