# Local AI Model Recommendations for Financial Extraction

For extracting transactions from raw text or CSV bank statements, you need a model that fits in your GPU VRAM (RTX 5060 has 8GB VRAM), supports strict JSON schemas, excels at tabular layout parsing, and maintains low hallucination rates.

Here are the best models to load in LM Studio:

---

## 1. Qwen-2.5-7B-Instruct (or 14B-Instruct) — *Top Recommendation*
*   **Why**: The Qwen series is trained heavily on tabular data, mathematical reasoning, and coding, making it the absolute best at parsing structured sheets, markdown tables, and multi-column bank logs.
*   **VRAM Footprint**: The **7B (Q5_K_M)** version fits entirely inside your 8GB VRAM (under 6GB active size), meaning 100% GPU offloading and extremely fast processing (60+ tokens/sec).
*   **Settings**: Set `temperature: 0.0` and configure the model in LM Studio to use **JSON Mode** or specify the schema in the system prompt.

## 2. DeepSeek-R1-Distill-Qwen-14B (or Llama-8B) — *Best for Complex/Messy Statements*
*   **Why**: Distilled R1 models write reasoning steps (`<think>...</think>`) before outputting JSON. This allows the model to "think through" which column represents deposits vs. withdrawals, compute balancing checks, and double-check date formats.
*   **Performance**: Slightly slower due to reasoning token generation, but has unmatched accuracy on unstructured or poorly formatted PDFs.
*   **Quantization**: Use **8B (Q5_K_M)** for maximum speed, or **14B (Q4_K_M)** for maximum reasoning capability (offloading 70-80% of layers to GPU).

## 3. Llama-3.1-8B-Instruct / Llama-3.3-8B-Instruct — *Consistent & Fast*
*   **Why**: Llama-3.1 is the industry standard for general instruction-following and structured output. It is highly reliable and handles function-calling/JSON formatting seamlessly.
*   **Performance**: Fast, stable, and has very low hallucinations when temperature is set to `0.0`.

---

## LM Studio Setup Optimization for RTX 5060 (8GB VRAM)

1.  **GPU Offload**:
    *   Set **GPU Override** to `Max` or slide the GPU Offload slider all the way to the right.
    *   For 7B or 8B models (e.g., Qwen-2.5-7B), offload **all layers** (usually 32 or 33 layers).
2.  **Context Length**:
    *   Set the context limit to at least **8,000 tokens** (8K). Bank statements can be wordy; if the context length is too low, statements will be truncated, leading to missed transactions.
3.  **Temperature & Top-P**:
    *   Set **Temperature** to `0.0` (makes the output deterministic).
    *   Set **Top-P** to `1.0` or disable it. Low temperature prevents the model from "inventing" transactions.
