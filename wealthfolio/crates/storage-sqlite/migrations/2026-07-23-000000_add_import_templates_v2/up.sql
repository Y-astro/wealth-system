ALTER TABLE import_templates ADD COLUMN source_type TEXT DEFAULT 'bank_statement';
ALTER TABLE import_templates ADD COLUMN ai_template_json TEXT;
