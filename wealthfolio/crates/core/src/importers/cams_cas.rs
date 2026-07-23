use regex::Regex;
use rust_decimal::Decimal;
use std::str::FromStr;
use crate::activities::ActivityImport;

pub struct CamsCasParser;

impl CamsCasParser {
    pub fn is_cams_kfintech_statement(text: &str) -> bool {
        let text_lower = text.to_lowercase();
        text_lower.contains("consolidated account statement") &&
        (text_lower.contains("cams") || text_lower.contains("kfintech") || text_lower.contains("karvy"))
    }

    pub fn parse(text: &str, account_id: &str) -> Result<Vec<ActivityImport>, String> {
        let mut activities = Vec::new();
        
        // This is a simplified regex-based parser for CAMS/KFintech CAS
        // It looks for typical transaction rows.
        // E.g., "01-Jan-2023  Purchase  1000.00  10.000  100.0000  10.000"
        let line_re = Regex::new(r"(?i)^(\d{2}-[a-zA-Z]{3}-\d{4})\s+(.*?)\s+([\d,]+\.\d+)\s+([\d,]+\.\d+)\s+([\d,]+\.\d+)\s+([\d,]+\.\d+)").unwrap();
        
        let folio_re = Regex::new(r"(?i)Folio No\s*:\s*([A-Z0-9/]+)").unwrap();
        let scheme_re = Regex::new(r"(?i)Scheme\s*:\s*(.+)").unwrap();
        let isin_re = Regex::new(r"(?i)ISIN\s*:\s*([A-Z0-9]+)").unwrap();

        let mut current_folio = String::new();
        let mut current_scheme = String::new();
        let mut current_isin = String::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }

            if let Some(caps) = folio_re.captures(line) {
                current_folio = caps.get(1).unwrap().as_str().trim().to_string();
                continue;
            }
            if let Some(caps) = scheme_re.captures(line) {
                current_scheme = caps.get(1).unwrap().as_str().trim().to_string();
                continue;
            }
            if let Some(caps) = isin_re.captures(line) {
                current_isin = caps.get(1).unwrap().as_str().trim().to_string();
                continue;
            }

            if let Some(caps) = line_re.captures(line) {
                let date_str = caps.get(1).unwrap().as_str();
                let type_str = caps.get(2).unwrap().as_str().trim();
                let amount_str = caps.get(3).unwrap().as_str().replace(",", "");
                let units_str = caps.get(4).unwrap().as_str().replace(",", "");
                let nav_str = caps.get(5).unwrap().as_str().replace(",", "");
                // closing balance is caps.get(6)

                // Parse Date (DD-MMM-YYYY to YYYY-MM-DD)
                let date = if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(date_str, "%d-%b-%Y") {
                    parsed_date.format("%Y-%m-%d").to_string()
                } else {
                    date_str.to_string()
                };

                // Map transaction type
                let type_lower = type_str.to_lowercase();
                let mut activity_type = "BUY".to_string();
                let mut subtype = None;
                
                if type_lower.contains("purchase") || type_lower.contains("sip") {
                    activity_type = "BUY".to_string();
                    if type_lower.contains("sip") {
                        subtype = Some("SIP".to_string());
                    }
                } else if type_lower.contains("redemption") || type_lower.contains("swp") || type_lower.contains("sell") {
                    activity_type = "SELL".to_string();
                    if type_lower.contains("swp") {
                        subtype = Some("SWP".to_string());
                    }
                } else if type_lower.contains("dividend reinvestment") || type_lower.contains("drip") {
                    activity_type = "DIVIDEND".to_string();
                    subtype = Some("DRIP".to_string());
                } else if type_lower.contains("dividend payout") || type_lower.contains("dividend") {
                    activity_type = "DIVIDEND".to_string();
                } else if type_lower.contains("stp") {
                    // depends on direction, assuming SELL for now or BUY
                    if type_lower.contains("in") {
                        activity_type = "BUY".to_string();
                    } else {
                        activity_type = "SELL".to_string();
                    }
                }

                let amount = Decimal::from_str(&amount_str).unwrap_or(Decimal::ZERO);
                let quantity = Decimal::from_str(&units_str).unwrap_or(Decimal::ZERO);
                let unit_price = Decimal::from_str(&nav_str).unwrap_or(Decimal::ZERO);

                let symbol = if !current_isin.is_empty() {
                    current_isin.clone()
                } else if !current_scheme.is_empty() {
                    current_scheme.clone()
                } else {
                    current_folio.clone()
                };

                let comment = format!("{} | Folio: {}", type_str, current_folio);

                activities.push(ActivityImport {
                    id: None,
                    date,
                    symbol,
                    activity_type,
                    quantity: Some(quantity),
                    unit_price: Some(unit_price),
                    currency: "INR".to_string(), // CAS is Indian
                    fee: Some(Decimal::ZERO),
                    amount: Some(amount),
                    comment: Some(comment),
                    account_id: Some(account_id.to_string()),
                    account_name: None,
                    symbol_name: if !current_scheme.is_empty() { Some(current_scheme.clone()) } else { None },
                    exchange_mic: None,
                    quote_ccy: None,
                    instrument_type: Some("MUTUAL_FUND".to_string()),
                    quote_mode: None,
                    errors: None,
                    warnings: None,
                    duplicate_of_id: None,
                    duplicate_of_line_number: None,
                    is_draft: true,
                    is_valid: true,
                    line_number: None,
                    fx_rate: None,
                    subtype,
                    force_import: false,
                    asset_id: None,
                    isin: if !current_isin.is_empty() { Some(current_isin.clone()) } else { None },
                });
            }
        }
        
        Ok(activities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cams_kfintech_detection() {
        let text1 = "Consolidated Account Statement\nGenerated by CAMS";
        assert!(CamsCasParser::is_cams_kfintech_statement(text1));

        let text2 = "Something else bank statement";
        assert!(!CamsCasParser::is_cams_kfintech_statement(text2));
    }

    #[test]
    fn test_parse_cams_cas() {
        let text = "\
        Consolidated Account Statement
        Folio No: 12345/67
        ISIN: INF123456789
        Scheme: HDFC Flexi Cap Fund
        Date Transaction Amount Units NAV Balance
        01-Jan-2023 Purchase 1000.00 10.000 100.0000 10.000
        15-Feb-2023 SIP 500.00 4.800 104.1666 14.800
        20-Mar-2023 Redemption 200.00 1.818 110.0110 12.982
        ";

        let activities = CamsCasParser::parse(text, "acc-1").unwrap();
        assert_eq!(activities.len(), 3);

        assert_eq!(activities[0].activity_type, "BUY");
        assert_eq!(activities[0].quantity.unwrap(), Decimal::from_str("10.000").unwrap());
        assert_eq!(activities[0].amount.unwrap(), Decimal::from_str("1000.00").unwrap());
        
        assert_eq!(activities[1].activity_type, "BUY");
        assert_eq!(activities[1].subtype.as_deref(), Some("SIP"));

        assert_eq!(activities[2].activity_type, "SELL");
    }
}
