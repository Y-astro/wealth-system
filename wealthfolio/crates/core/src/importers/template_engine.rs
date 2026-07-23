use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
