import os
import sys
import subprocess
import json
import urllib.request
import urllib.error
import re
import logging

# Setup path resolution relative to script
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# Setup comprehensive logging
LOG_FILE = os.path.join(SCRIPT_DIR, "process_all.log")
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler(LOG_FILE),
        logging.StreamHandler(sys.stdout)
    ]
)

INPUT_DIR = os.path.join(SCRIPT_DIR, "inputs")
OUTPUT_DIR = SCRIPT_DIR
INGEST_SCRIPT = os.path.join(SCRIPT_DIR, "ingest.py")
LM_STUDIO_URL = "http://127.0.0.1:1234/v1/chat/completions"
WEALTHFOLIO_URL = "http://127.0.0.1:8080/api/v1/activities/import"
ACCOUNTS_URL = "http://127.0.0.1:8080/api/v1/accounts"

def get_accounts():
    try:
        req = urllib.request.Request(ACCOUNTS_URL)
        with urllib.request.urlopen(req, timeout=10) as response:
            return json.loads(response.read().decode('utf-8'))
    except Exception as e:
        logging.error(f"Failed to fetch accounts: {str(e)}")
        return []

def process_file(file_path):
    file_name = os.path.basename(file_path)
    logging.info(f"--- Starting processing for {file_name} ---")
    
    # Fetch accounts to provide context to AI
    accounts = get_accounts()
    accounts_context = "\n".join([f"- Name: {a['name']}, ID: {a['id']}, Currency: {a['currency']}" for a in accounts])
    
    # 1. Text Extraction
    try:
        logging.info(f"Running ingest.py for {file_name}...")
        result = subprocess.run(['python3', INGEST_SCRIPT, file_path], capture_output=True, text=True, check=True)
        extracted_text = result.stdout.strip()
        if not extracted_text:
            logging.warning(f"No text extracted from {file_name}")
            return
    except subprocess.CalledProcessError as e:
        logging.error(f"Error ingesting {file_name}: {e.stderr}")
        return

    # Save ai_input_<filename>.txt
    input_log_path = os.path.join(OUTPUT_DIR, f"ai_input_{file_name}.txt")
    try:
        with open(input_log_path, 'w') as f:
            f.write(extracted_text)
        logging.info(f"Saved extracted text to {input_log_path}")
    except Exception as e:
        logging.error(f"Failed to save input log for {file_name}: {str(e)}")

    # 2. AI Formatting via LM Studio
    prompt = (
        "You are a financial data extraction expert. Your task is to extract all transactions from the provided bank statement text "
        "into a strict JSON format compatible with Wealthfolio.\n\n"
        f"### Available Wealthfolio Accounts:\n{accounts_context}\n\n"
        "### CRITICAL INSTRUCTIONS:\n"
        "1. **CSV/Tabular Structure**: Pay close attention to the column headers (e.g., Date, Narration, Withdrawal, Deposit, Balance).\n"
        "   - **NEVER** use the 'Balance' column as the transaction amount.\n"
        "   - The transaction amount is in either the 'Withdrawal/Debit' or 'Deposit/Credit' column.\n"
        "2. **Mandatory Fields for Activities**:\n"
        "   - `date`: ISO format (YYYY-MM-DD).\n"
        "   - `activityType`: MUST be either \"DEPOSIT\" or \"WITHDRAWAL\" (for bank statements).\n"
        "   - `symbol`: Use the currency code (e.g., \"USD\", \"INR\").\n"
        "   - `quantity`: Always use 1.\n"
        "   - `unitPrice`: The absolute amount of the transaction.\n"
        "   - `currency`: The currency code.\n"
        "   - `comment`: The transaction narration.\n"
        "   - `accountId`: Choose the best matching Account ID from the list above.\n"
        "   - `isDraft`: false.\n"
        "   - `isValid`: true.\n"
        "3. **Closing Balance**: You MUST extract the final closing balance on the statement and include it as `closingBalance`. Also provide the statement's ending date as `closingBalanceDate`.\n\n"
        "### EXPECTED JSON SCHEMA:\n"
        "{\n"
        "  \"closingBalance\": 12345.67,\n"
        "  \"closingBalanceDate\": \"YYYY-MM-DD\",\n"
        "  \"activities\": [\n"
        "    {\n"
        "      \"date\": \"YYYY-MM-DD\",\n"
        "      \"activityType\": \"DEPOSIT|WITHDRAWAL\",\n"
        "      \"symbol\": \"USD\",\n"
        "      \"quantity\": 1,\n"
        "      \"unitPrice\": 123.45,\n"
        "      \"currency\": \"USD\",\n"
        "      \"comment\": \"Narration\",\n"
        "      \"accountId\": \"UUID\",\n"
        "      \"isDraft\": false,\n"
        "      \"isValid\": true\n"
        "    }\n"
        "  ]\n"
        "}\n\n"
        "Return ONLY the JSON object. No preamble."
    )

    payload = {
        "model": "qwen",
        "messages": [
            {"role": "system", "content": prompt},
            {"role": "user", "content": extracted_text}
        ],
        "temperature": 0.0
    }

    logging.info(f"Sending text to AI model... (5-minute timeout enabled)")
    try:
        req = urllib.request.Request(
            LM_STUDIO_URL, 
            data=json.dumps(payload).encode('utf-8'), 
            headers={'Content-Type': 'application/json'}
        )
        # 5 minute timer
        with urllib.request.urlopen(req, timeout=300) as response:
            res_body = response.read().decode('utf-8')
            res_json = json.loads(res_body)
            content = res_json['choices'][0]['message']['content']
            logging.info(f"AI Response received ({len(content)} chars).")
            
            # Extract JSON reliably
            # Try to find a list or an object
            json_match = re.search(r'(\[.*\]|\{.*\})', content, re.DOTALL)
            if json_match:
                final_json_str = json_match.group(1)
            else:
                final_json_str = content.strip()

            # Remove potential markdown code blocks
            final_json_str = re.sub(r'^```json\s*', '', final_json_str)
            final_json_str = re.sub(r'\s*```$', '', final_json_str)

            json_data = json.loads(final_json_str)
            if isinstance(json_data, list):
                json_data = {"activities": json_data}
            elif "activities" not in json_data:
                json_data = {"activities": [json_data]}
                
    except urllib.error.HTTPError as e:
        error_msg = e.read().decode('utf-8')
        logging.error(f"LM Studio API returned error {e.code}: {error_msg}")
        return
    except urllib.error.URLError as e:
        logging.error(f"Network error communicating with LM Studio: {e.reason}")
        return
    except json.JSONDecodeError as e:
        logging.error(f"Failed to parse JSON from AI response: {str(e)}\nRaw AI Output: {content}")
        return
    except Exception as e:
        logging.error(f"Unexpected error formatting AI response: {str(e)}")
        return

    # Save ai_output_<filename>.json
    output_log_path = os.path.join(OUTPUT_DIR, f"ai_output_{file_name}.json")
    try:
        with open(output_log_path, 'w') as f:
            json.dump(json_data, f, indent=2)
        logging.info(f"Saved AI output to {output_log_path}")
    except Exception as e:
        logging.error(f"Failed to save AI output for {file_name}: {str(e)}")
        
    # 3. Direct Wealthfolio Ingestion
    logging.info(f"Ingesting {file_name} data into Wealthfolio...")
    try:
        # Create a new account in Wealthfolio for this file
        account_name = f"AI Import - {file_name}"
        payload_acc = {
            "name": account_name,
            "currency": "INR",
            "accountType": "CASH",
            "isDefault": False,
            "isActive": True,
            "trackingMode": "TRANSACTIONS"
        }
        target_account_id = None
        try:
            req_acc = urllib.request.Request(
                ACCOUNTS_URL,
                data=json.dumps(payload_acc).encode('utf-8'),
                headers={'Content-Type': 'application/json'}
            )
            with urllib.request.urlopen(req_acc, timeout=10) as response_acc:
                acc_res = json.loads(response_acc.read().decode('utf-8'))
                target_account_id = acc_res['id']
                logging.info(f"Created new account '{account_name}' with ID: {target_account_id}")
        except Exception as e:
            logging.error(f"Failed to create account for {file_name}: {str(e)}")
            target_account_id = accounts[0]['id'] if accounts else "0e64c3da-e5f5-4ef9-a5f3-84ebcb6eaa8a"
        
        activities = []
        closing_balance = None
        if isinstance(json_data, list):
            activities = json_data
        elif isinstance(json_data, dict) and 'activities' in json_data:
            activities = json_data['activities']
            closing_balance = json_data.get('closingBalance')
            
        for activity in activities:
            activity['accountId'] = target_account_id
            if 'unitPrice' in activity and 'amount' not in activity:
                activity['amount'] = activity['unitPrice']

        if closing_balance is not None and activities:
            try:
                cb_float = float(closing_balance)
                # Calculate the net change of all activities
                net_change = sum(
                    float(act.get('amount', act.get('unitPrice', 0))) if act.get('activityType') == 'DEPOSIT' else -float(act.get('amount', act.get('unitPrice', 0)))
                    for act in activities
                )
                opening_balance = cb_float - net_change
                
                # Get the earliest date for the opening balance, default to closingBalanceDate if available, else first activity
                dates = [act.get('date') for act in activities if act.get('date')]
                earliest_date = min(dates) if dates else "2000-01-01"
                
                # Add the opening balance adjustment transaction
                activities.insert(0, {
                    "date": earliest_date,
                    "activityType": "DEPOSIT" if opening_balance >= 0 else "WITHDRAWAL",
                    "symbol": "INR",
                    "quantity": 1,
                    "unitPrice": abs(opening_balance),
                    "amount": abs(opening_balance),
                    "currency": "INR",
                    "comment": "Opening Balance",
                    "accountId": target_account_id,
                    "isDraft": False,
                    "isValid": True
                })
                logging.info(f"Injected Opening Balance of {opening_balance} to reach closing balance {cb_float}")
            except Exception as e:
                logging.error(f"Failed to calculate opening balance: {e}")

        payload_wf = {"activities": activities}
        
        req_wf = urllib.request.Request(
            WEALTHFOLIO_URL, 
            data=json.dumps(payload_wf).encode('utf-8'), 
            headers={'Content-Type': 'application/json'}
        )
        with urllib.request.urlopen(req_wf, timeout=30) as response_wf:
            wf_res = response_wf.read().decode('utf-8')
            logging.info(f"Wealthfolio ingestion successful: {wf_res}")
    except urllib.error.HTTPError as e:
        error_msg = e.read().decode('utf-8')
        logging.error(f"Wealthfolio API returned error {e.code}: {error_msg}")
    except urllib.error.URLError as e:
        logging.error(f"Network error communicating with Wealthfolio: {e.reason}. Ensure the server is running on port 8080.")
    except Exception as e:
        logging.error(f"Wealthfolio ingestion failed: {str(e)}")
        
    logging.info(f"--- Finished processing {file_name} ---\n")

def main():
    if not os.path.exists(INPUT_DIR):
        logging.error(f"Input directory not found: {INPUT_DIR}")
        sys.exit(1)

    files = [f for f in os.listdir(INPUT_DIR) if f.endswith(('.pdf', '.xlsx', '.xls', '.csv', '.md', '.txt'))]
    if not files:
        logging.info("No files found to process.")
        return

    logging.info(f"Found {len(files)} files to process. Starting sequential processing...")
    # Send files one by one
    for file in files:
        process_file(os.path.join(INPUT_DIR, file))

if __name__ == "__main__":
    main()