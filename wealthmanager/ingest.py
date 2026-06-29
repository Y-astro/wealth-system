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

try:
    import fitz  # PyMuPDF
except ImportError:
    logger.error("Dependency 'PyMuPDF' (fitz) is missing. Install it using 'pip install pymupdf'.")
    sys.exit(1)

try:
    import pandas as pd
except ImportError:
    logger.error("Dependency 'pandas' is missing. Install it using 'pip install pandas openpyxl'.")
    sys.exit(1)

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
    """Extracts text from PDF and filters for lines that look like transaction data."""
    extracted_text = []
    try:
        with fitz.open(file_path) as doc:
            for page in doc:
                text = page.get_text("text")
                for line in text.splitlines():
                    line = line.strip()
                    if not line:
                        continue
                    
                    # Heuristic: Keep lines that contain a date or are very likely part of a narration
                    # For bank statements, transactions usually start with a date.
                    if is_date(line) or (len(line) > 10 and any(c.isdigit() for c in line)):
                        extracted_text.append(line)
        
        # Join and perform final cleanup
        return clean_text("\n".join(extracted_text))
    except Exception as e:
        raise RuntimeError(f"Failed to extract PDF: {str(e)}")

def extract_excel(file_path: str) -> str:
    """Extracts tabular data from an Excel file and converts it to a cleaned, compact CSV string."""
    try:
        excel_file = pd.ExcelFile(file_path)
        final_output = []

        for sheet_name in excel_file.sheet_names:
            df = pd.read_excel(file_path, sheet_name=sheet_name)
            if df.empty:
                continue

            # Drop completely empty columns and rows
            df = df.dropna(how='all').dropna(axis=1, how='all')

            if not df.empty:
                csv_output = io.StringIO()
                df.to_csv(csv_output, index=False)
                csv_str = csv_output.getvalue()
                
                # Cleanup CSV: remove rows that are just commas or empty
                cleaned_csv = "\n".join([l for l in csv_str.splitlines() if re.search(r'[a-zA-Z0-9]', l)])
                final_output.append(f"--- Sheet: {sheet_name} ---\n{cleaned_csv}")
        
        return "\n".join(final_output).strip()
    except Exception as e:
        raise RuntimeError(f"Failed to extract Excel: {str(e)}")

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
            df = pd.read_csv(file_path)
            extracted_text = df.to_csv(index=False)
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
