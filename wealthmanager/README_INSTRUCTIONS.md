# Wealthmanager: Extraction Layer (Layer 2) Instructions

After running the Ingestion Layer (`ingest.py`), you will have raw text or CSV data. Follow these steps to process it with Qwen via LM Studio.

## 1. Prerequisites
Ensure LM Studio is running and the **Qwen** model is loaded. 
Enable the **Local Server** (usually on `http://localhost:1234`).

## 2. API Payload Construction
LM Studio provides an OpenAI-compatible API. Your orchestrator (n8n or a script) should send a `POST` request to:
`http://localhost:1234/v1/chat/completions`

### Payload Structure:
```json
{
  "model": "qwen",
  "messages": [
    {
      "role": "system",
      "content": "You are a financial data extraction expert. Extract all transactions from the provided bank statement text into a strict JSON format compatible with Wealthfolio. \n\nExpected JSON Schema:\n{\n  \"activities\": [\n    {\n      \"date\": \"YYYY-MM-DD\",\n      \"type\": \"BUY|SELL|DIVIDEND|DEPOSIT|WITHDRAWAL|FEE|TAX\",\n      \"symbol\": \"USD\", \n      \"quantity\": 1,\n      \"price\": 123.45,\n      \"currency\": \"USD\",\n      \"fee\": 0,\n      \"note\": \"Transaction Description\"\n    }\n  ]\n}\nEnsure dates are in ISO format and amounts are numbers."
    },
    {
      "role": "user",
      "content": "EXTRACTED_TEXT_FROM_INGEST_PY"
    }
  ],
  "temperature": 0.0,
  "response_format": { "type": "json_object" }
}
```

## 3. Formatting Python Output for the API
When calling `ingest.py` from n8n or another tool:
1. Capture `stdout`.
2. Escape the `stdout` string to be valid JSON (handles newlines and quotes).
3. Replace `EXTRACTED_TEXT_FROM_INGEST_PY` in the payload above with this escaped string.

## 4. Wealthfolio Integration
Wealthfolio expects a JSON file for import. Once you receive the JSON response from LM Studio:
1. Parse the `choices[0].message.content`.
2. Save it as `import.json`.
3. In Wealthfolio UI, navigate to **Activities -> Import** and upload the generated JSON.

## 5. Local Optimization (RTX 5060)
Since you have an RTX 5060, ensure LM Studio is configured to use **GPU Offloading** (set GPU Layers to Max) for Qwen. This will make the extraction layer significantly faster during the hackathon.
