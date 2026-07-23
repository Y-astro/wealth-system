use std::str::FromStr;
use std::collections::HashMap;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::activities::ActivityImport;
use crate::assets::{AssetServiceTrait, AssetKind, InstrumentType, QuoteMode, NewAsset};

pub struct BrokerCsvParser;

#[derive(Debug, PartialEq)]
pub enum BrokerType {
    Zerodha,
    Groww,
    AngelOne,
    Upstox,
    Unknown,
}

impl BrokerCsvParser {
    /// Detect if the provided text is a supported broker CSV by checking headers.
    pub fn detect_broker(csv_text: &str) -> BrokerType {
        let first_line = csv_text.lines().next().unwrap_or("").to_lowercase();
        
        if first_line.contains("symbol") && first_line.contains("isin") && first_line.contains("trade_date") && first_line.contains("trade_type") {
            BrokerType::Zerodha
        } else if first_line.contains("stock/mf name") || (first_line.contains("date") && first_line.contains("transaction type") && first_line.contains("amount")) {
            BrokerType::Groww
        } else if first_line.contains("angelone") || (first_line.contains("scrip") && first_line.contains("buy/sell")) {
            BrokerType::AngelOne
        } else if first_line.contains("upstox") || first_line.contains("scrip name") {
            BrokerType::Upstox
        } else {
            BrokerType::Unknown
        }
    }

    /// Parse the CSV based on detected broker
    pub fn parse(csv_text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let broker = Self::detect_broker(csv_text);
        match broker {
            BrokerType::Zerodha => Self::parse_zerodha(csv_text, account_id),
            BrokerType::Groww => Self::parse_groww(csv_text, account_id),
            BrokerType::AngelOne | BrokerType::Upstox => Self::parse_generic_indian(csv_text, account_id),
            BrokerType::Unknown => Err("Unsupported or unknown broker CSV format".to_string()),
        }
    }

    fn parse_zerodha(csv_text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(csv_text.as_bytes());
        let mut activities = Vec::new();

        let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
        let header_map: HashMap<String, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.trim().to_lowercase(), i))
            .collect();

        let sym_idx = *header_map.get("symbol").ok_or("Missing symbol column")?;
        let isin_idx = *header_map.get("isin").ok_or("Missing isin column")?;
        let date_idx = *header_map.get("trade_date").ok_or("Missing trade_date column")?;
        let type_idx = *header_map.get("trade_type").ok_or("Missing trade_type column")?;
        let qty_idx = *header_map.get("quantity").ok_or("Missing quantity column")?;
        let price_idx = *header_map.get("price").ok_or("Missing price column")?;
        let amt_idx = header_map.get("trade_value").copied();

        for (i, result) in rdr.records().enumerate() {
            let record = result.map_err(|e| format!("Row {}: {}", i + 2, e))?;
            
            let symbol = record.get(sym_idx).unwrap_or("").trim().to_string();
            let date_str = record.get(date_idx).unwrap_or("").trim();
            let trade_type = record.get(type_idx).unwrap_or("").trim().to_lowercase();
            let qty_str = record.get(qty_idx).unwrap_or("").trim();
            let price_str = record.get(price_idx).unwrap_or("").trim();
            
            let activity_type = match trade_type.as_str() {
                "buy" | "purchase" => "BUY".to_string(),
                "sell" | "redemption" => "SELL".to_string(),
                _ => continue, // Skip unknown types
            };

            let quantity = Decimal::from_str(qty_str).ok();
            let unit_price = Decimal::from_str(price_str).ok();
            
            let mut amount = None;
            if let Some(idx) = amt_idx {
                if let Some(val) = record.get(idx) {
                    amount = Decimal::from_str(val.trim()).ok();
                }
            }
            if amount.is_none() {
                if let (Some(q), Some(p)) = (quantity, unit_price) {
                    amount = Some(q * p);
                }
            }

            let date = Self::parse_date(date_str).unwrap_or_else(|| date_str.to_string());
            let isin = record.get(isin_idx).unwrap_or("").trim().to_string();
            let trade_id = format!("zerodha:{}", Uuid::new_v4()); // Idempotency key generated below

            let mut import = ActivityImport {
                id: None,
                date,
                symbol,
                activity_type,
                quantity,
                unit_price,
                currency: "INR".to_string(),
                fee: None,
                amount,
                comment: Some(format!("IdempotencyKey: zerodha:{}_{}_{}", date_str, isin, i)), // use index to make unique per day
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
                isin: Some(isin.clone()),
                force_import: false,
            };
            activities.push(import);
        }

        Ok(activities)
    }

    fn parse_groww(csv_text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(csv_text.as_bytes());
        let mut activities = Vec::new();

        let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
        let header_map: HashMap<String, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.trim().to_lowercase(), i))
            .collect();

        let date_idx = *header_map.get("date").ok_or("Missing date column")?;
        let name_idx = *header_map.get("stock/mf name").or_else(|| header_map.get("stock name")).or_else(|| header_map.get("name")).ok_or("Missing name column")?;
        let type_idx = *header_map.get("transaction type").or_else(|| header_map.get("type")).ok_or("Missing transaction type column")?;
        let qty_idx = *header_map.get("quantity").ok_or("Missing quantity column")?;
        let price_idx = *header_map.get("price").ok_or("Missing price column")?;
        let amt_idx = header_map.get("amount").copied();

        for (i, result) in rdr.records().enumerate() {
            let record = result.map_err(|e| format!("Row {}: {}", i + 2, e))?;
            
            let symbol = record.get(name_idx).unwrap_or("").trim().to_string();
            let date_str = record.get(date_idx).unwrap_or("").trim();
            let trade_type = record.get(type_idx).unwrap_or("").trim().to_lowercase();
            let qty_str = record.get(qty_idx).unwrap_or("").trim();
            let price_str = record.get(price_idx).unwrap_or("").trim();
            
            let activity_type = match trade_type.as_str() {
                "buy" | "purchase" => "BUY".to_string(),
                "sell" | "redemption" => "SELL".to_string(),
                _ => continue,
            };

            let quantity = Decimal::from_str(qty_str.replace(",", "").as_str()).ok();
            let unit_price = Decimal::from_str(price_str.replace(",", "").as_str()).ok();
            
            let mut amount = None;
            if let Some(idx) = amt_idx {
                if let Some(val) = record.get(idx) {
                    amount = Decimal::from_str(val.trim().replace(",", "").as_str()).ok();
                }
            }
            if amount.is_none() {
                if let (Some(q), Some(p)) = (quantity, unit_price) {
                    amount = Some(q * p);
                }
            }

            let date = Self::parse_date(date_str).unwrap_or_else(|| date_str.to_string());
            let import = ActivityImport {
                id: None,
                date,
                symbol: symbol.clone(),
                activity_type,
                quantity,
                unit_price,
                currency: "INR".to_string(),
                fee: None,
                amount,
                comment: Some(format!("IdempotencyKey: groww:{}_{}_{}", date_str, symbol.replace(" ", ""), i)),
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

    fn parse_generic_indian(csv_text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(csv_text.as_bytes());
        let mut activities = Vec::new();

        let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
        let header_map: HashMap<String, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.trim().to_lowercase(), i))
            .collect();

        // Very generic fallback
        let date_idx = header_map.get("date").or_else(|| header_map.get("trade date"));
        let sym_idx = header_map.get("symbol").or_else(|| header_map.get("scrip")).or_else(|| header_map.get("scrip name"));
        let type_idx = header_map.get("type").or_else(|| header_map.get("buy/sell")).or_else(|| header_map.get("transaction type"));
        let qty_idx = header_map.get("quantity").or_else(|| header_map.get("qty"));
        let price_idx = header_map.get("price").or_else(|| header_map.get("rate"));

        if date_idx.is_none() || sym_idx.is_none() || type_idx.is_none() || qty_idx.is_none() || price_idx.is_none() {
            return Err("Missing required columns for generic Indian broker parsing".to_string());
        }
        
        let date_idx = *date_idx.unwrap();
        let sym_idx = *sym_idx.unwrap();
        let type_idx = *type_idx.unwrap();
        let qty_idx = *qty_idx.unwrap();
        let price_idx = *price_idx.unwrap();

        for (i, result) in rdr.records().enumerate() {
            let record = result.map_err(|e| format!("Row {}: {}", i + 2, e))?;
            
            let symbol = record.get(sym_idx).unwrap_or("").trim().to_string();
            let date_str = record.get(date_idx).unwrap_or("").trim();
            let trade_type = record.get(type_idx).unwrap_or("").trim().to_lowercase();
            let qty_str = record.get(qty_idx).unwrap_or("").trim();
            let price_str = record.get(price_idx).unwrap_or("").trim();
            
            let activity_type = match trade_type.as_str() {
                "buy" | "b" | "purchase" => "BUY".to_string(),
                "sell" | "s" | "redemption" => "SELL".to_string(),
                _ => continue,
            };

            let quantity = Decimal::from_str(qty_str.replace(",", "").as_str()).ok();
            let unit_price = Decimal::from_str(price_str.replace(",", "").as_str()).ok();
            
            let mut amount = None;
            if let (Some(q), Some(p)) = (quantity, unit_price) {
                amount = Some(q * p);
            }

            let date = Self::parse_date(date_str).unwrap_or_else(|| date_str.to_string());
            let import = ActivityImport {
                id: None,
                date,
                symbol: symbol.clone(),
                activity_type,
                quantity,
                unit_price,
                currency: "INR".to_string(),
                fee: None,
                amount,
                comment: Some(format!("IdempotencyKey: generic:{}_{}_{}", date_str, symbol.replace(" ", ""), i)),
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
}

pub async fn resolve_and_auto_create_broker_assets(
    activities: &mut Vec<ActivityImport>,
    asset_service: &dyn AssetServiceTrait,
) -> Result<(), String> {
    let existing_assets = asset_service.get_assets().map_err(|e| e.to_string())?;
    
    let mut symbol_to_asset_id: HashMap<String, String> = HashMap::new();

    for act in activities.iter_mut() {
        let symbol_key = act.symbol.clone();
        
        if let Some(asset_id) = symbol_to_asset_id.get(&symbol_key) {
            act.asset_id = Some(asset_id.clone());
            continue;
        }

        let matched_asset = existing_assets.iter().find(|a| {
            let a_symbol = a.instrument_symbol.as_deref().unwrap_or("");
            let act_isin = act.isin.as_deref().unwrap_or("");
            a_symbol == symbol_key ||
            (!act_isin.is_empty() && a_symbol == act_isin) ||
            (a.display_code.as_deref().unwrap_or("") == symbol_key)
        });

        if let Some(asset) = matched_asset {
            act.asset_id = Some(asset.id.clone());
            symbol_to_asset_id.insert(symbol_key, asset.id.clone());
        } else {
            let new_asset = NewAsset {
                id: None,
                kind: AssetKind::Investment,
                name: Some(act.symbol.clone()),
                display_code: Some(act.symbol.clone()),
                notes: None,
                metadata: None,
                is_active: true,
                quote_mode: QuoteMode::Market,
                quote_ccy: "INR".to_string(),
                instrument_type: Some(InstrumentType::Equity),
                instrument_symbol: Some(act.isin.clone().unwrap_or_else(|| act.symbol.clone())),
                instrument_exchange_mic: Some("XNSE".to_string()),
                provider_config: None,
            };
            if let Ok(created) = asset_service.create_asset(new_asset).await {
                act.asset_id = Some(created.id.clone());
                symbol_to_asset_id.insert(symbol_key, created.id);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_zerodha() {
        let csv = "symbol, isin, trade_date, exchange, segment, trade_type, quantity, price, trade_value, brokerage, stt, charges\nRELIANCE,INE002A01018,2023-10-10,NSE,EQ,buy,10,2300,23000,20,23,5";
        assert_eq!(BrokerCsvParser::detect_broker(csv), BrokerType::Zerodha);
    }

    #[test]
    fn test_parse_zerodha() {
        let csv = "symbol, isin, trade_date, exchange, segment, trade_type, quantity, price, trade_value, brokerage, stt, charges\nRELIANCE,INE002A01018,2023-10-10,NSE,EQ,buy,10,2300,23000,20,23,5";
        let res = BrokerCsvParser::parse(csv, "acc1").unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].activity_type, "BUY");
        assert_eq!(res[0].symbol, "RELIANCE");
        assert_eq!(res[0].quantity, Some(Decimal::from_str("10").unwrap()));
        assert_eq!(res[0].unit_price, Some(Decimal::from_str("2300").unwrap()));
    }

    #[test]
    fn test_detect_groww() {
        let csv = "Date, Stock/MF Name, Transaction Type, Quantity, Price, Amount\n10/10/2023,TCS,Sell,5,3500,17500";
        assert_eq!(BrokerCsvParser::detect_broker(csv), BrokerType::Groww);
    }
}
