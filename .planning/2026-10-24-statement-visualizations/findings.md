# Findings

- React/Vite/Recharts frontend, Axum server; this checkout contains no Tauri Rust application despite stale AGENTS paths.
- Existing addon POSTs activities/import/statement; bank-only prompt; hardcoded qwen fallback; Ollama selection unused; silent remote Gemini fallback.
- wealthmanager scripts Python and ingest.py require PDF/Excel dependencies even for Markdown.
- Sample nsdl_values.md: 801 lines, doubled OCR headings, pipe/space mixed rows, 13 monthly history entries, NSDL/CDSL equities and ETF holdings, MF cost/unrealized data, NPS, unit-only demat transactions and unrelated personal/footer data.
- Reported portfolio INR 32,468,211.65; preserve reported market values (NAV rounded), face values excluded; pledged quantities are subsets.
- Summary account identifiers differ from detailed sections in dummy sample; report mismatch, do not join by invented identifiers.
- LM Studio unreachable on localhost:1234 at inspection. Need mocked native/v0/OpenAI model discovery tests; live inference conditional on server.
- Official LM Studio native /api/v1/models reports models[].type, key, loaded_instances[].id/config.context_length. v0 reports data[].state; /v1/models compatibility last fallback.
- Worktree has substantial pre-existing edits; preserve all unrelated changes.
