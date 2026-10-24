import sys
import os
import argparse
import io
import json
import logging
import re
from typing import Optional, List

# Setup basic logging to stderr
logging.basicConfig(level=logging.INFO, format='%(levelname)s: %(message)s', stream=sys.stderr)
logger = logging.getLogger(__name__)

def is_date(text: str) -> bool:
    """Rough check if a string looks like a date (DD/MM/YYYY, YYYY-MM-DD, etc.)"""
    # DD/MM/YYYY or DD-MM-YYYY or DD.MM.YYYY
    if re.search(r'\d{1,2}[/\-\.]\d{1,2}[/\-\.]\d{2,4}', text):
        return True
    # YYYY-MM-DD
    if re.search(r'\d{4}-\d{1,2}-\d{1,2}', text):
        return True
    return False

def clean_text(text: str) -> str:
    """General text cleaning: reduce whitespace, remove lines with only symbols."""
    # Remove lines that are just separators like ************ or ------------
    lines = [line for line in text.splitlines() if not re.match(r'^[*\-=_]{5,}$', line.strip())]
    # Strip leading/trailing whitespace and remove empty lines
    cleaned_lines = [line.strip() for line in lines if line.strip()]
    return "\n".join(cleaned_lines)

def extract_pdf(file_path: str) -> str:
    """Convert locally with the installed MarkItDown CLI; preserve its Markdown."""
    import shutil
    import subprocess
    import tempfile
    from pathlib import Path

    executable = os.environ.get("WEALTH_MARKITDOWN") or shutil.which("markitdown")
    if not executable:
        local = Path.home() / ".local" / "bin" / "markitdown"
        if local.is_file() and os.access(local, os.X_OK):
            executable = str(local)
    if not executable:
        raise ValueError("The local MarkItDown converter is unavailable. Restore its installation and retry the saved file.")
    try:
        with tempfile.TemporaryDirectory(prefix="wealth-markitdown-") as directory:
            output = Path(directory) / "converted.md"
            result = subprocess.run(
                [executable, str(Path(file_path).resolve()), "-o", str(output)],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, timeout=180, check=False,
            )
            if result.returncode != 0:
                # Converter diagnostics may contain statement text; never expose them.
                diagnostic = result.stderr.decode("utf-8", errors="replace").lower()
                if "password" in diagnostic or "encrypt" in diagnostic:
                    raise ValueError("This PDF is password-protected. Upload an unlocked copy to continue.")
                raise ValueError("MarkItDown could not convert this PDF. Check that it is readable and the local PDF converter dependencies are installed, then retry the saved file.")
            if not output.is_file():
                raise ValueError("MarkItDown produced no Markdown. Check the PDF and retry the saved file.")
            markdown = output.read_text(encoding="utf-8-sig")
            if not markdown.strip():
                raise ValueError("No readable text was found. Scanned PDFs need OCR; password-protected PDFs must be unlocked first.")
            return markdown
    except subprocess.TimeoutExpired as error:
        raise ValueError("Local PDF conversion exceeded three minutes. Retry the saved file or split the PDF into smaller statements.") from error
    except OSError as error:
        raise ValueError("The local MarkItDown converter could not start. Check its installation and retry the saved file.") from error


def extract_excel(file_path: str) -> str:
    """Find bank table headers before conversion; preserve spreadsheet row evidence."""
    import datetime
    import pandas as pd
    from statement_parser import BANK_HEADERS, header_key, iso_date, number

    def cell(value):
        if pd.isna(value):
            return ""
        if isinstance(value, (datetime.datetime, datetime.date)):
            return value.strftime("%Y-%m-%d")
        return str(value).strip().replace("\r", " ").replace("\n", " ").replace("|", "/")

    def date_cell(value):
        rendered = cell(value)
        parsed = iso_date(rendered) or iso_date(rendered.split(" ")[0])
        if parsed:
            return parsed
        if isinstance(value, (int, float)) and 20_000 < value < 80_000:
            return (datetime.datetime(1899, 12, 30) + datetime.timedelta(days=float(value))).date().isoformat()
        return ""

    result = []
    excel = pd.ExcelFile(file_path)
    try:
        for sheet in excel.sheet_names:
            frame = pd.read_excel(excel, sheet_name=sheet, header=None, dtype=object)
            frame = frame.dropna(axis=0, how="all").dropna(axis=1, how="all")
            if frame.empty:
                continue
            table = list(frame.iterrows())
            header = None
            columns = {}
            for position, (_, values) in enumerate(table[:100]):
                keys = [header_key(cell(value)) for value in values]
                found = {key: next((i for i, value in enumerate(keys) if value in aliases), None) for key, aliases in BANK_HEADERS.items()}
                if found["date"] is not None and found["description"] is not None and any(found[key] is not None for key in ("debit", "credit", "amount")):
                    header, columns = position, found
                    break
            if header is None:
                # Retain generic extraction for unfamiliar layouts; local AI remains the fallback.
                output = frame.to_csv(index=False, header=False)
                result.append("--- Sheet: " + sheet + " ---\n" + output)
                continue
            output = ["--- Sheet: " + sheet + " ---"]
            currency_codes = set(re.findall(r"\b(INR|USD|EUR|GBP)\b", " ".join(cell(value) for value in table[header][1]), re.I))
            if len(currency_codes) == 1:
                output.append("Currency: " + next(iter(currency_codes)).upper())
            for original_index, values in table:
                rendered = [cell(value) for value in values]
                for index, value in enumerate(rendered):
                    match = re.search(r"(opening|closing)\s*balance", value, re.I)
                    if not match:
                        continue
                    candidates = [value[match.end():]] + rendered[index + 1:index + 3]
                    amount = next((number(value) for value in candidates if number(value) is not None), None)
                    if amount is not None:
                        output.append(match[1].title() + " Balance: " + str(amount))
            selected = [(name, key) for name, key in [("Date", "date"), ("Narration", "description"), ("Debit", "debit"), ("Credit", "credit"), ("Amount", "amount"), ("Direction", "direction"), ("Balance", "balance"), ("Currency", "currency")] if columns.get(key) is not None]
            output.append("| " + " | ".join(name for name, _ in selected) + " | Source row |")
            output.append("| " + " | ".join("---" for _ in range(len(selected) + 1)) + " |")
            transactions = []
            notes = []
            for original_index, values in table[header + 1:]:
                raw_values = list(values)
                def value(key):
                    index = columns.get(key)
                    return raw_values[index] if index is not None and index < len(raw_values) else ""
                date = date_cell(value("date"))
                description = cell(value("description"))
                amounts = [number(cell(value(key))) for key in ("debit", "credit", "amount")]
                if not date:
                    if any(amount not in (None, 0) for amount in amounts) and re.match(r"[0-9]", cell(value("date"))):
                        notes.append("excluded spreadsheet row " + str(original_index + 1) + ": unrecognized transaction date")
                    elif transactions and not cell(value("date")) and description and not any(amount not in (None, 0) for amount in amounts) and not re.match(r"(?:note|this |please |closing|opening|generated|end of|abbreviation|legend|customer|the |we )", description, re.I):
                        narration_index = next(i for i, (_, key) in enumerate(selected) if key == "description")
                        transactions[-1][narration_index] += " " + description
                    continue
                if not description and not any(amount not in (None, 0) for amount in amounts):
                    continue
                transactions.append([date if key == "date" else cell(value(key)) for _, key in selected] + [str(original_index + 1)])
            output.extend("| " + " | ".join(row) + " |" for row in transactions)
            output.extend(notes)
            result.append("\n".join(output))
    finally:
        excel.close()
    return "\n\n".join(result).strip()

def main():
    parser = argparse.ArgumentParser(description="Wealthmanager Ingestion Layer: Optimized text extraction.")
    parser.add_argument("file_path", help="Path to the file to extract.")
    parser.add_argument("--raw", action="store_true", help="Output raw text without filtering (for debugging).")
    
    args = parser.parse_args()
    file_path = args.file_path

    if not os.path.exists(file_path):
        logger.error(f"Error: File not found: {file_path}")
        sys.exit(1)

    _, extension = os.path.splitext(file_path.lower())
    
    try:
        if extension == '.pdf':
            extracted_text = extract_pdf(file_path)
        elif extension in ['.xlsx', '.xls']:
            extracted_text = extract_excel(file_path)
        elif extension == '.csv':
            with open(file_path, encoding='utf-8-sig') as source:
                extracted_text = source.read()
        else:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                extracted_text = f.read()

        # Final compression of whitespace
        extracted_text = re.sub(r'\n{3,}', '\n\n', extracted_text)
        print(extracted_text.strip())
        
        # Log stats to stderr
        token_estimate = len(extracted_text) // 4  # Very rough estimate
        logger.info(f"Extraction complete. Estimated tokens: {token_estimate}")
        
    except Exception as e:
        logger.error(f"Processing Error: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    main()
