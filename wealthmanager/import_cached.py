import os
import sys
import json
import urllib.request
import urllib.error
import logging

# Setup logging to console
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[logging.StreamHandler(sys.stdout)]
)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

WEALTHFOLIO_BASE = os.environ.get("WEALTHFOLIO_BASE", "http://127.0.0.1:8080")
WEALTHFOLIO_URL = f"{WEALTHFOLIO_BASE}/api/v1/activities/import"
ACCOUNTS_URL = f"{WEALTHFOLIO_BASE}/api/v1/accounts"

def get_accounts():
    try:
        req = urllib.request.Request(ACCOUNTS_URL)
        with urllib.request.urlopen(req, timeout=10) as response:
            return json.loads(response.read().decode('utf-8'))
    except Exception as e:
        logging.error(f"Failed to fetch accounts: {str(e)}")
        return []

def import_cached_file(json_path):
    json_name = os.path.basename(json_path)
    # Extract original statement filename from the cached output filename
    # e.g., "ai_output_Axis bank AcctStatement_XXX3596_30032026.xls.json" -> "Axis bank AcctStatement_XXX3596_30032026.xls"
    file_name = json_name.replace("ai_output_", "").replace(".json", "")
    logging.info(f"--- Importing cached data for {file_name} ---")

    # Load cached JSON
    try:
        with open(json_path, 'r') as f:
            json_data = json.load(f)
    except Exception as e:
        logging.error(f"Failed to read cached JSON file {json_name}: {e}")
        return

    # Fetch existing accounts
    accounts = get_accounts()
    
    # Ingestion steps
    try:
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
                logging.info(f"Created/referenced account '{account_name}' with ID: {target_account_id}")
        except Exception as e:
            logging.error(f"Failed to create account: {str(e)}")
            # Fallback to first existing account if creation fails (e.g. if database is empty but has a default)
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
                net_change = sum(
                    float(act.get('amount', act.get('unitPrice', 0))) if act.get('activityType') == 'DEPOSIT' else -float(act.get('amount', act.get('unitPrice', 0)))
                    for act in activities
                )
                opening_balance = cb_float - net_change
                
                dates = [act.get('date') for act in activities if act.get('date')]
                earliest_date = min(dates) if dates else "2000-01-01"
                
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
            logging.info(f"Ingestion successful: {wf_res}")
    except urllib.error.HTTPError as e:
        error_msg = e.read().decode('utf-8')
        logging.error(f"API returned error {e.code}: {error_msg}")
    except urllib.error.URLError as e:
        logging.error(f"Network error communicating with Wealthfolio: {e.reason}. Ensure server is running on port 8080.")
    except Exception as e:
        logging.error(f"Ingestion failed: {str(e)}")

def main():
    files = [f for f in os.listdir(SCRIPT_DIR) if f.startswith('ai_output_') and f.endswith('.json')]
    if not files:
        logging.info("No cached AI outputs (ai_output_*.json) found to import.")
        return

    logging.info(f"Found {len(files)} cached JSON outputs to import. Starting...")
    for file in files:
        import_cached_file(os.path.join(SCRIPT_DIR, file))

if __name__ == "__main__":
    main()
