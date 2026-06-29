import json
import urllib.request

LM_STUDIO_URL = "http://127.0.0.1:1234/v1/chat/completions"
payload = {
    "model": "qwen/qwen3.5-9b",
    "messages": [
        {"role": "system", "content": "Return the word 'TEST'"},
        {"role": "user", "content": "Hello"}
    ],
    "temperature": 0.0
}

try:
    req = urllib.request.Request(LM_STUDIO_URL, data=json.dumps(payload).encode('utf-8'), headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req) as response:
        print(response.read().decode('utf-8'))
except Exception as e:
    print(f"Error: {e}")
