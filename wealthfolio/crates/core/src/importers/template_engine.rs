use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemplateSignature {
    pub column_headers: Vec<String>,
    pub header_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEngine {
    pub templates: Vec<TemplateSignature>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self { templates: vec![] }
    }

    pub fn extract_signature(&self, raw_text: &str) -> TemplateSignature {
        let lines: Vec<&str> = raw_text.lines().take(50).collect();
        let mut column_headers = Vec::new();
        
        for line in lines {
            if line.to_lowercase().contains("date") && line.to_lowercase().contains("amount") {
                column_headers = line.split('\t').map(|s| s.trim().to_string()).collect();
                break;
            } else if line.to_lowercase().contains("date") && line.to_lowercase().contains("balance") {
                column_headers = line.split('\t').map(|s| s.trim().to_string()).collect();
                break;
            }
        }
        
        TemplateSignature {
            column_headers,
            header_markers: vec![],
        }
    }

    pub fn match_template<'a>(&self, raw_text: &str, templates: &'a [crate::activities::ImportTemplateData]) -> Option<&'a crate::activities::ImportTemplateData> {
        let sig = self.extract_signature(raw_text);
        if sig.column_headers.is_empty() {
            return None;
        }

        for template in templates {
            if let Some(t_sig) = &template.template_signature {
                if t_sig.column_headers == sig.column_headers {
                    return Some(template);
                }
            }
        }
        None
    }

    pub fn extract_with_template(
        &self,
        _raw_text: &str,
        _template: &crate::activities::ImportTemplateData,
        _account_id: &str,
    ) -> Result<Vec<crate::activities::ActivityImport>, String> {
        // Dummy implementation for now, just to satisfy the API
        // Real implementation would parse the text based on template signature
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_signature() {
        let engine = TemplateEngine::new();
        let text = "Header\nDate\tNarration\tWithdrawal\tDeposit\tBalance\n2023-01-01\tTest\t100\t\t900\n";
        let sig = engine.extract_signature(text);
        assert_eq!(sig.column_headers, vec!["Date", "Narration", "Withdrawal", "Deposit", "Balance"]);
    }

    #[test]
    fn test_match_template() {
        let engine = TemplateEngine::new();
        let template = crate::activities::ImportTemplateData {
            id: "test".to_string(),
            name: "Test".to_string(),
            scope: crate::activities::ImportTemplateScope::User,
            kind: crate::activities::TemplateKind::CsvActivity,
            field_mappings: Default::default(),
            activity_mappings: Default::default(),
            symbol_mappings: Default::default(),
            account_mappings: Default::default(),
            symbol_mapping_meta: Default::default(),
            parse_config: None,
            template_signature: Some(TemplateSignature {
                column_headers: vec!["Date".to_string(), "Amount".to_string()],
                header_markers: vec![],
            }),
        };

        let text = "Date\tAmount\n2023-01-01\t100\n";
        let binding = [template.clone()];
        let matched = engine.match_template(text, &binding);
        assert!(matched.is_some());

        let text2 = "Date\tBalance\n2023-01-01\t100\n";
        let binding2 = [template.clone()];
        let matched2 = engine.match_template(text2, &binding2);
        assert!(matched2.is_none());
    }
}
