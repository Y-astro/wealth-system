"""Canonical statement reports. No network, database writes, or optional dependencies.

Every sourceLines array contains 1-based line numbers into the ORIGINAL input.
PDF tables are stateful: section/account/header determines a row's meaning.
"""
from __future__ import annotations

import calendar
import csv
import hashlib
import io
import math
import re
from datetime import datetime
from decimal import Decimal, InvalidOperation
from pathlib import Path

NUMBER = r"[+-]?(?:\d[\d,]*)(?:\.\d+)?"
ISIN = re.compile(r"\bIN[A-Z0-9]{10}\b")
SYMBOL = re.compile(r"\b[A-Z0-9&-]+\.(?:NSE|BSE)\b")
DATE = r"(?:\d{4}-\d{2}-\d{2}|\d{1,2}[-/](?:[A-Za-z]{3}|\d{1,2})[-/]\d{2,4})"


def number(value: str | int | float | None) -> float | None:
    if value is None or isinstance(value, bool):
        return None
    text = str(value).strip().replace(",", "").replace("₹", "").replace("`", "")
    text = re.sub(r"^(?:INR|Rs\.?|USD|EUR|GBP)\s*", "", text, flags=re.I)
    text = re.sub(r"\s*(?:CR|DR)$", "", text, flags=re.I)
    if text.startswith("(") and text.endswith(")"):
        text = "-" + text[1:-1]
    try:
        result = Decimal(text)
        return float(result) if result.is_finite() and math.isfinite(float(result)) else None
    except InvalidOperation:
        return None


def money_sum(values) -> float:
    return float(sum((Decimal(str(v)) for v in values if v is not None), Decimal(0)))


def iso_date(value: str) -> str | None:
    for fmt in ("%d %b %Y", "%d %B %Y", "%d-%m-%Y", "%d-%m-%y", "%Y/%m/%d", "%Y-%m-%d", "%d-%b-%Y", "%d/%b/%Y", "%d/%m/%Y", "%d-%m-%Y", "%d/%m/%y", "%d-%m-%y"):
        try:
            return datetime.strptime(value.strip(), fmt).date().isoformat()
        except ValueError:
            pass
    return None


def normalize_line(raw: str) -> str:
    # Only collapse when almost the entire word consists of duplicated pairs.
    # Never collapse legitimate double letters (e.g. ALLIED, 1,000 or 2200).
    def undouble(match):
        word = match.group()
        if len(word) >= 8 and len(word) % 2 == 0 and all(word[i] == word[i + 1] for i in range(0, len(word), 2)):
            return word[::2]
        return word
    raw = re.sub(r"[A-Za-z]{8,}", undouble, raw)
    return re.sub(r"\s+", " ", raw.replace("|", " ").replace("\u00a0", " ")).strip()


def source_rows(text: str) -> list[dict]:
    return [{"line": i, "text": raw, "normalized": normalize_line(raw)}
            for i, raw in enumerate(text.splitlines(), 1)]


def report_base(text: str, filename: str, kind: str) -> dict:
    return {
        "schemaVersion": 1,
        "id": hashlib.sha256(text.encode("utf-8")).hexdigest(),
        "kind": kind,
        "filename": Path(filename).name,
        "currency": "INR",
        "period": {"from": None, "to": None},
        "summary": {}, "accounts": [], "allocations": [], "history": [],
        "holdings": [], "transactions": [], "mappings": [], "warnings": [],
        "source": source_rows(text),
        "extraction": {"method": "deterministic", "model": None},
    }


def mapping(report, target, lines, rule):
    report["mappings"].append({"target": target, "sourceLines": list(dict.fromkeys(lines)), "rule": rule})


def warn(report, message):
    if message not in report["warnings"]:
        report["warnings"].append(message)


def is_nsdl(text: str) -> bool:
    normalized = "\n".join(normalize_line(line) for line in text.splitlines())
    return bool(re.search(r"CONSOLIDATED\s+ACCOUNT\s+STATEMENT", normalized, re.I)
                and ("NSDL" in normalized.upper() or "NATIONAL SECURITIES DEPOSITORY" in normalized.upper()))


def parse_nsdl(text: str, filename: str = "statement.md") -> dict:
    report = report_base(text, filename, "investment")
    rows = report["source"]
    account = None
    mode = "summary"
    holding_section = False
    transaction_section = False
    txn_isin = None
    txn_subtype = "Beneficiary"
    cdsl = False
    last_holding = None
    summary_ids = []
    details_ids = []
    summary_account = None
    pending_provider = ""
    for idx, row in enumerate(rows):
        line, raw, ln = row["normalized"], row["text"], row["line"]
        if not line or re.fullmatch(r"[-: ]+", line):
            continue
        compact = line.replace(" ", "")
        period = re.search(rf"period\s*from\s*({DATE})\s*to\s*({DATE})", line, re.I)
        if period and report["period"]["to"] is None:
            report["period"] = {"from": iso_date(period[1]), "to": iso_date(period[2])}
            mapping(report, "period", [ln], "Statement date range; not transaction dates.")
        if "YOUR CONSOLIDATED PORTFOLIO VALUE" in line.upper() and "reportedValue" not in report["summary"]:
            vals = re.findall(NUMBER, line)
            if vals:
                report["summary"]["reportedValue"] = number(vals[-1])
                mapping(report, "summary.reportedValue", [ln], "Reported consolidated market value in INR; never sum repeated totals.")
        history = re.match(r"^([A-Z]{3}) (20\d{2})\s+(" + NUMBER + r")(?:\s+(" + NUMBER + r"|NA))?(?:\s+(" + NUMBER + r"|NA))?", line)
        if history:
            month = list(calendar.month_abbr).index(history[1].title()) if history[1].title() in calendar.month_abbr else 0
            if month:
                date = f"{history[2]}-{month:02d}-{calendar.monthrange(int(history[2]), month)[1]:02d}"
                report["history"].append({"date": date, "value": number(history[3]), "change": number(history[4]), "changePercent": number(history[5]), "sourceLines": [ln]})
                mapping(report, f"history[{len(report['history']) - 1}]", [ln], "Month, portfolio value, absolute change, percentage change. Change includes cash flows; not investment return.")
                continue
        allocation = re.match(r"^([A-Za-z ]+)\(([A-Z]+)\)\s+(" + NUMBER + r")\s+(" + NUMBER + r")%", line)
        if allocation and not holding_section:
            report["allocations"].append({"name": re.sub(r"(?<=[a-z])(?=[A-Z])", " ", allocation[1]).strip(), "code": allocation[2], "value": number(allocation[3]), "percent": number(allocation[4]), "sourceLines": [ln]})
            mapping(report, f"allocations[{len(report['allocations']) - 1}]", [ln], "Composition summary; percentages are already percent units, not fractions. Separate from holding rows.")
            continue
        # Summary is not holdings. Keep its reported values and IDs independently.
        if mode == "summary":
            summary_match = re.match(r"^(NSDLDematAccount|CDSLDematAccount|MutualFundFolios|NPSAccount)\s*(.*?)\s+(\d+(?:Folios)?)\s+(" + NUMBER + r")$", line)
            if summary_match:
                provider = summary_match[2].strip() or pending_provider or summary_match[1]
                summary_account = {"id": f"summary-{ln}", "name": provider, "type": summary_match[1], "reportedValue": number(summary_match[4]), "value": number(summary_match[4]), "sourceLines": [ln]}
                report["accounts"].append(summary_account)
                mapping(report, f"accounts[{len(report['accounts']) - 1}]", [ln], "Summary account total; not an individual holding or transaction.")
                pending_provider = ""
            elif re.match(r"^[A-Z][A-Z ]+(?:LIMITED|Limited)$", line) or "TechnologiesLimited" in line:
                pending_provider = line
            ids = re.search(r"DPID:([A-Z0-9]+)ClientID:([A-Z0-9]+)", compact, re.I)
            if ids and summary_account:
                summary_account["summaryAccountId"] = ids[1] + "/" + ids[2]
                summary_ids.append(summary_account["summaryAccountId"])
        if line in ("NSDL Demat Account", "CDSL Demat Account") and not transaction_section:
            holding_section = True
            last_holding = None
            cdsl = line.startswith("CDSL")
            mode = "cdsl" if cdsl else "equity"
            # Detailed account identity must come from this section, not summary IDs.
            lookahead = rows[idx + 1:idx + 7]
            provider = next((r["normalized"] for r in lookahead if "LIMITED" in r["normalized"]), line)
            ids = next((re.search(r"DP ID:\s*(\w+)\s+Client ID:\s*(\w+)", r["normalized"]) for r in lookahead if "DP ID:" in r["normalized"]), None)
            key = (ids[1] + "/" + ids[2]) if ids else f"details-{ln}"
            details_ids.append(key)
            account = {"id": key, "name": provider, "type": "CDSL" if cdsl else "NSDL", "value": 0, "reportedValue": None, "sourceLines": [ln] + [r["line"] for r in lookahead if "DP ID:" in r["normalized"]]}
            report["accounts"].append(account)
            last_holding = None
            continue
        if line == "Transactions":
            transaction_section = True
            holding_section = False
            account = None
            mode = "securities"
            continue
        if "End of Statement" in line:
            break
        if transaction_section:
            # Depository debit/credit columns are UNITS, not rupees or trade prices.
            if line == "CDSL Demat Account":
                cdsl = True
            elif line == "NSDL Demat Account":
                cdsl = False
            if "National Pension System (NPS) Transaction" in line:
                mode = "nps-transactions"
            im = re.search(r"ISIN\s*:\s*(IN[A-Z0-9]{10})", line)
            if im:
                txn_isin = im[1]
                txn_subtype = "Beneficiary"
                continue
            if line.startswith("Beneficiary"):
                txn_subtype = line
            tx = re.match(rf"^({DATE})\s+(.*)", line)
            if tx:
                date = iso_date(tx[1])
                if not date:
                    continue
                rest = tx[2]
                if "Opening Balance" in rest or "Closing Balance" in rest:
                    continue
                if mode == "nps-transactions":
                    m = re.search(r"\b(DR|CR)\s+(" + NUMBER + r")\s+(" + NUMBER + r")\s+(" + NUMBER + r")$", rest)
                    if m:
                        report["transactions"].append({"date": date, "description": rest[:m.start()].strip(), "direction": "out" if m[1] == "DR" else "in", "amount": number(m[2]), "quantity": number(m[4]), "currency": "INR", "flowType": "expense" if m[1] == "DR" else "income", "category": "Pension fees", "unit": "cash", "sourceLines": [ln]})
                elif txn_isin:
                    nums = re.findall(NUMBER, rest)
                    count = 2 if cdsl else 4
                    if len(nums) >= count:
                        tail = list(map(number, nums[-count:]))
                        opening, debit, credit, closing = (None, tail[0], 0, tail[1]) if cdsl else tail
                        report["transactions"].append({"date": date, "isin": txn_isin, "description": rest, "direction": "out" if debit else "in", "quantity": debit or credit, "openingQuantity": opening, "closingQuantity": closing, "amount": None, "currency": None, "flowType": "security", "category": txn_subtype, "unit": "units", "sourceLines": [ln]})
                if report["transactions"] and report["transactions"][-1]["sourceLines"] == [ln]:
                    mapping(report, f"transactions[{len(report['transactions']) - 1}]", [ln], "NPS fee amount/NAV/units." if mode == "nps-transactions" else "Demat movement in units; do not fabricate buy/sell cash or use current valuation as trade price. Blocked movements remain distinct.")
            continue
        if not holding_section:
            continue
        if re.match(r"^Mutual Fund Folios\s*\(F\)", line):
            mode = "folio"
            last_holding = None
            account = {"id": "mutual-fund-folios", "name": "Mutual fund folios", "type": "FOLIO", "value": 0, "reportedValue": None, "sourceLines": [ln]}
            report["accounts"].append(account)
            continue
        if "National Pension System (NPS) Holding Details" in line:
            mode = "nps"
            last_holding = None
            account = {"id": "nps", "name": "National Pension System", "type": "NPS", "value": 0, "reportedValue": None, "sourceLines": [ln]}
            report["accounts"].append(account)
            continue
        if re.match(r"^Mutual Funds\s*\(M\)", line):
            last_holding = None
            mode = "cdsl-fund" if cdsl else "fund"
            continue
        total = re.match(r"^Total\s+(" + NUMBER + r")$", line)
        if total and account:
            account["reportedValue"] = number(total[1])
            mapping(report, f"accountTotal.{account['id']}", [ln], "Reported section total used only for reconciliation.")
            continue
        if mode == "nps" and line.startswith("NPS TRUST-"):
            m = re.match(r"^(.*?)\s+(" + NUMBER + r")\s+(" + NUMBER + r")\s+(" + NUMBER + r")$", line)
            if m:
                holding = {"isin": None, "symbol": None, "name": m[1], "assetClass": "NPS", "quantity": number(m[2]), "price": number(m[3]), "value": number(m[4]), "cost": None, "unrealizedGain": None}
            else:
                continue
        else:
            m = re.match(r"^(IN[A-Z0-9]{10})\s+(.+?)\s+((?:" + NUMBER + r"\s+){2,}" + NUMBER + r")$", line)
            if not m:
                # Symbol and wrapped name belong to the preceding ISIN row.
                if last_holding is not None and not any(s in line for s in ("Sub Total", "Total", "ISIN", "Company Name")):
                    sym = SYMBOL.search(line)
                    if sym:
                        last_holding["symbol"] = sym[0]
                    # Strip symbols, pledge subrows and numeric sub-balances from name.
                    continuation = SYMBOL.sub("", line).strip()
                    continuation = re.sub(r"^MF[A-Z0-9]+\s*", "", continuation)
                    if sym:
                        continuation = re.sub(NUMBER, "", continuation).strip()
                    if re.search(r"\b(?:NAV|Units|Value|Bal|Cost|UCC|PFM|TIER|Stock|Current|Price)\b", continuation, re.I):
                        continuation = ""
                    if continuation and not re.search(r"\d|Pledge|Unconfirmed|Consolidated|Summary|National Securities|Equities|Mutual|Shares$", continuation, re.I):
                        last_holding["name"] += " " + continuation
                    if sym or (continuation and re.fullmatch(r"[A-Za-z &'()./-]+", continuation)):
                        last_holding["sourceLines"].append(ln)
                        report["mappings"][last_holding["mappingIndex"]]["sourceLines"].append(ln)
                    if "Pledged" in line or "Unconfirmed Pledge" in line:
                        pledge = re.findall(NUMBER, line)
                        if pledge:
                            last_holding["pledgedQuantity"] = number(pledge[-1])
                continue
            values = list(map(number, re.findall(NUMBER, m[3])))
            if mode == "folio" and len(values) == 7:
                folio, quantity, average, cost, price, value, gain = values
                holding = {"isin": m[1], "symbol": None, "name": m[2], "assetClass": "Mutual fund folios", "quantity": quantity, "price": price, "value": value, "cost": cost, "unrealizedGain": gain, "folio": m[3].split()[0], "averageCost": average}
            elif mode in ("cdsl", "cdsl-fund") and len(values) == 5:
                holding = {"isin": m[1], "symbol": None, "name": m[2], "assetClass": "Equities" if mode == "cdsl" else "Mutual funds / ETFs", "quantity": values[0], "price": values[-2], "value": values[-1], "cost": None, "unrealizedGain": None, "pledgedQuantity": values[2]}
            elif mode == "equity" and len(values) == 4:
                holding = {"isin": m[1], "symbol": None, "name": m[2], "assetClass": "Equities", "faceValue": values[0], "quantity": values[1], "price": values[2], "value": values[3], "cost": None, "unrealizedGain": None}
            elif mode == "fund" and len(values) == 3:
                holding = {"isin": m[1], "symbol": None, "name": m[2], "assetClass": "Mutual funds / ETFs", "quantity": values[0], "price": values[1], "value": values[2], "cost": None, "unrealizedGain": None}
            else:
                warn(report, f"Unmapped holding at source line {ln}: expected {mode} columns; review the original row.")
                continue
        holding.update({"id": f"holding-{ln}", "accountId": account["id"] if account else "unknown", "accountName": account["name"] if account else "Unknown", "currency": "INR", "sourceLines": [ln]})
        rule = {"equity": "ISIN | name | faceValue (not price) | quantity | marketPrice | marketValue", "fund": "ISIN | name | units | NAV | reportedValue (NAV may be rounded)", "cdsl": "ISIN | security | currentBalance | safekeep | pledged | marketPrice | marketValue; free/locked/lent subrows are subsets, not additional holdings", "cdsl-fund": "ISIN | security | currentBalance | safekeep | pledged | NAV | reportedValue", "folio": "ISIN | name | folio | units | averageCost | totalCost | NAV | currentValue | unrealizedGain; use currentValue, never the final gain column", "nps": "PFM scheme | units | NAV | currentValue"}[mode]
        mapping(report, f"holdings[{len(report['holdings'])}]", [ln], rule)
        holding["mappingIndex"] = len(report["mappings"]) - 1
        report["holdings"].append(holding)
        last_holding = holding
    for holding in report["holdings"]:
        holding.pop("mappingIndex", None)
    # Finalize without treating summaries or per-account duplicate ISINs as new lots.
    for acct in report["accounts"]:
        if acct["id"].startswith("summary-"):
            continue
        acct["value"] = money_sum(h["value"] for h in report["holdings"] if h["accountId"] == acct["id"])
        if acct["reportedValue"] is not None and abs(acct["value"] - acct["reportedValue"]) > 0.05:
            warn(report, f"{acct['name']}: parsed holdings differ from reported account total by INR {acct['value'] - acct['reportedValue']:.2f}.")
    parsed_value = money_sum(h["value"] for h in report["holdings"])
    reported = report["summary"].get("reportedValue")
    report["summary"].update({"parsedValue": parsed_value, "difference": round(parsed_value - reported, 2) if reported is not None else None, "holdingCount": len(report["holdings"]), "securityCount": len({h['isin'] or h['name'] for h in report['holdings']}), "knownCost": money_sum(h["cost"] for h in report["holdings"]), "knownUnrealizedGain": money_sum(h["unrealizedGain"] for h in report["holdings"])})
    if reported is not None and abs(parsed_value - reported) > 0.05:
        warn(report, f"Parsed holdings differ from reported consolidated value by INR {parsed_value - reported:.2f}; inspect source mappings.")
    if summary_ids and any(key not in summary_ids for key in details_ids):
        warn(report, "Summary and detailed account identifiers differ. Detailed holdings use their own section's account IDs; summary IDs were not substituted.")
    if not report["holdings"]:
        raise ValueError("Recognized an NSDL statement but could not map holdings. Check the source layout; no values were invented.")
    report["history"].sort(key=lambda p: p["date"])
    return report


CATEGORIES = [
    ("Salary", r"salary|payroll"), ("Interest", r"interest|int\.paid|int\.cr"),
    ("Dividends", r"dividend"), ("Refunds", r"refund|reversal"),
    ("Housing", r"\brent\b|mortgage|housing"), ("Groceries", r"grocery|groceries|supermarket|bigbasket|dmart"),
    ("Food & dining", r"swiggy|zomato|restaurant|cafe|dining"), ("Transport", r"uber|ola\b|petrol|fuel|metro"),
    ("Utilities", r"electric|broadband|utility|utilities|water bill|mobile bill"),
    ("Health", r"hospital|pharmacy|medical|health"), ("Shopping", r"amazon|flipkart|shopping"),
    ("Fees & taxes", r"\bfee\b|\bfees\b|charge|\btax\b|\bgst\b"),
]


def categorize(description, direction):
    if re.search(r"credit card payment|card repayment|bbps payment received", description, re.I):
        return "Card repayments", "transfer"
    # UPI/NEFT/IMPS alone do not prove an internal transfer.
    if re.search(r"own account|self transfer|internal transfer|transfer to self|transfer from self", description, re.I):
        return "Internal transfers", "transfer"
    if re.search(r"\bsip\b|mutual fund|investment|broker|zerodha|groww", description, re.I):
        return "Investments", "transfer"
    category = next((name for name, pattern in CATEGORIES if re.search(pattern, description, re.I)), "Other income" if direction == "in" else "Other expenses")
    return category, "income" if direction == "in" else "expense"


def header_key(value):
    return re.sub(r"(?:inr|usd|eur|gbp)$", "", re.sub(r"[^a-z]", "", value.lower()))


BANK_HEADERS = {
    "date": {"date", "txndate", "transactiondate", "valuedate", "postingdate"},
    "description": {"description", "narration", "particulars", "transactiondetails", "remarks", "details"},
    "debit": {"debit", "debitamount", "withdrawal", "withdrawals", "withdrawalamount", "withdrawalamt", "dr", "paidout"},
    "credit": {"credit", "creditamount", "deposit", "deposits", "depositamount", "depositamt", "cr", "paidin"},
    "amount": {"amount", "transactionamount", "txnamount", "amountinrs"},
    "direction": {"type", "drcr", "crdr", "debitcredit", "transactiontype", "billingamountsign"},
    "balance": {"bal", "balance", "closingbalance", "runningbalance", "availablebalance"},
    "currency": {"currency", "ccy"},
}


BANK_HEADERS["date"].update({"trandate", "transactiondt", "postingdt"})
BANK_HEADERS["description"].update({"descriptionnarration", "narrationdescription", "transactiondescription"})

def bank_records(text, sources):
    # CSV quoted narrations can span lines and contain pipes/tabs. Parse the
    # entire stream with csv.reader instead of splitting physical rows first.
    first = next((r["text"] for r in sources if r["text"].strip()), "")
    delimiter = "\t" if "\t" in first and "," not in first else ","
    is_csv = first.lstrip().startswith('"') or ("," in first and "|" not in first)
    for source in sources[:100]:
        raw = source["text"]
        if "|" in raw:
            continue
        delim = "\t" if "\t" in raw else ","
        keys = {header_key(c) for c in next(csv.reader([raw], delimiter=delim))}
        if keys & BANK_HEADERS["date"] and keys & BANK_HEADERS["description"] and any(keys & BANK_HEADERS[k] for k in ("amount", "debit", "credit")):
            is_csv, delimiter = True, delim
            break
    if is_csv:
        reader = csv.reader(io.StringIO(text), delimiter=delimiter)
        previous = 0
        for cells in reader:
            lines = list(range(previous + 1, reader.line_num + 1))
            previous = reader.line_num
            raw = " ".join(sources[i - 1]["text"] for i in lines)
            yield cells, raw, lines
    else:
        for source in sources:
            raw = source["text"]
            cells = ([c.strip() for c in raw.strip().strip("|").split("|")]
                     if "|" in raw else next(csv.reader([raw], delimiter="\t" if "\t" in raw else ",")))
            yield cells, raw, [source["line"]]


def parse_bank(text: str, filename: str = "statement.md") -> dict:
    report = report_base(text, filename, "bank")
    # Currency declarations and table headers are evidence; merchant narrations
    # and FX footnotes must never relabel an entire account.
    declared_currency = None
    for row in report["source"][:30]:
        if re.match(rf"^\s*{DATE}\b", row["normalized"]):
            break
        declaration = re.search(r"\b(?:account\s+)?currency\s*[:=]\s*([A-Z]{3})\b", row["normalized"], re.I)
        if declaration:
            declared_currency = declaration[1].upper()
            report["currency"] = declared_currency
            mapping(report, "currency", [row["line"]], "Explicit account currency declaration, not keywords in merchant descriptions or FX notes.")
            break
    columns = None
    header_line = None
    card_billing = False
    for cells, raw, source_lines in bank_records(text, report["source"]):
        ln = source_lines[0]
        keys = [header_key(c) for c in cells]
        found = {key: next((i for i, cell in enumerate(keys) if cell in aliases), None) for key, aliases in BANK_HEADERS.items()}
        if found["date"] is not None and found["description"] is not None and any(found[k] is not None for k in ("credit", "debit", "amount")):
            columns, header_line = found, ln
            # ICICI credit-card exports: billed INR, explicit CR for credits,
            # blank BillingAmountSign means a debit. Do not apply to generic CSV.
            card_billing = "billingamountsign" in keys and "amountinrs" in keys
            if card_billing:
                report["statementType"] = "credit-card"
                report["currency"] = declared_currency = "INR"
                mapping(report, "currency", [ln], "Explicit Amount(in Rs) billed currency; ignore Intl.Amount and reward points.")
            header_currencies = set(re.findall(r"\b(?:INR|USD|EUR|GBP)\b", raw, re.I))
            if declared_currency is None and len(header_currencies) == 1:
                report["currency"] = next(iter(header_currencies)).upper()
                declared_currency = report["currency"]
                mapping(report, "currency", [ln], "Currency declared in table headers.")
            mapping(report, "transactions.columns", [ln], "Map by header name, including empty cells. Debit/withdrawal=out; credit/deposit=in. Balance is a running total, never a transaction amount.")
            continue
        normalized = normalize_line(raw)
        for key, label in (("openingBalance", "opening"), ("closingBalance", "closing")):
            m = re.search(rf"{label}\s+balance\s*[:=]?\s*(?:INR|USD|EUR|GBP|Rs\.?|₹|`)?\s*({NUMBER})(?:\s*(CR|DR))?", normalized, re.I)
            if m:
                val = number(m[1])
                report["summary"][key] = -abs(val) if m[2] and m[2].upper() == "DR" else val
                mapping(report, f"summary.{key}", [ln], "Balance used for reconciliation and funding only; excluded from income/expense transactions.")
        if columns is None:
            continue
        def cell(key):
            idx = columns[key]
            return cells[idx].strip() if idx is not None and idx < len(cells) else ""
        date = iso_date(cell("date"))
        if date is None:
            if re.match(r"^\d{1,4}[-/]", cell("date")):
                warn(report, f"Invalid transaction date at line {ln}; row excluded.")
            continue
        description = cell("description")
        if re.search(r"^(?:opening|closing|brought forward|carried forward)\s*balance", description, re.I):
            balance = number(cell("balance"))
            if balance is not None:
                report["summary"]["openingBalance" if re.search(r"opening|brought", description, re.I) else "closingBalance"] = balance
            continue
        debit, credit = number(cell("debit")), number(cell("credit"))
        if (debit is not None and debit < 0) or (credit is not None and credit < 0):
            warn(report, f"Signed debit/credit at line {ln} needs reversal review; row excluded.")
            continue
        if debit and credit:
            warn(report, f"Ambiguous bank row at line {ln}: both debit and credit are nonzero; excluded from charts.")
            continue
        direction, amount = ("out", abs(debit)) if debit else (("in", abs(credit)) if credit else (None, None))
        if amount is None and columns["amount"] is not None:
            amount = number(cell("amount"))
            drcr = cell("direction").lower()
            if amount is not None and amount != 0:
                if drcr in ("dr", "debit", "withdrawal", "out"):
                    direction = "out"
                elif drcr in ("cr", "credit", "deposit", "in"):
                    direction = "in"
                elif card_billing and drcr == "" and amount > 0:
                    direction = "out"
                elif amount < 0:
                    direction = "out"
                else:
                    warn(report, f"Bank row at line {ln} has a positive amount without debit/credit direction; excluded rather than guessed.")
                    continue
                amount = abs(amount)
        if direction is None or not amount:
            continue
        category, flow_type = categorize(description, direction)
        balance = number(cell("balance"))
        currency = cell("currency").upper() or report["currency"]
        if not re.fullmatch(r"[A-Z]{3}", currency):
            warn(report, f"Invalid currency at bank row {ln}; excluded.")
            continue
        txn = {"id": f"transaction-{ln}", "date": date, "description": description, "direction": direction, "amount": amount, "currency": currency, "category": category, "flowType": flow_type, "unit": "cash", "balance": balance, "sourceLines": source_lines}
        report["transactions"].append(txn)
        mapping(report, f"transactions[{len(report['transactions']) - 1}]", [header_line, *source_lines], "Transaction mapped by source columns. ICICI billed amount: blank sign=debit, CR=credit. Category is an editable local suggestion." if card_billing else "Transaction date/narration/debit/credit mapped by columns. Category is a local keyword suggestion; editable in review.")
    if declared_currency is None and report["transactions"] and (columns is None or columns["currency"] is None):
        warn(report, "No explicit account currency found; INR assumed. Verify the currency before interpreting values.")
    finalize_bank(report)
    return report


def finalize_bank(report):
    from credit_cards import apply_credit_card, finalize_card
    card = apply_credit_card(report)
    txns = report["transactions"]
    currencies = sorted({t["currency"] for t in txns})
    if len(currencies) > 1:
        warn(report, "Multiple currencies detected. Filter by currency; totals and Sankey must never combine currencies.")
    report["currencies"] = currencies or [report["currency"]]
    report["summary"]["transactionCount"] = len(txns)
    if not txns:
        return
    report["period"] = {"from": min(t["date"] for t in txns), "to": max(t["date"] for t in txns)}
    if len(currencies) != 1:
        return
    report["currency"] = currencies[0]
    net = money_sum(t["amount"] if t["direction"] == "in" else -t["amount"] for t in txns)
    summary = report["summary"]
    summary.update({"income": money_sum(t["amount"] for t in txns if t["flowType"] == "income"), "expenses": money_sum(t["amount"] for t in txns if t["flowType"] == "expense"), "netCashFlow": net})
    if card:
        finalize_card(report)
        return
    # Source order may be reverse chronological. Infer only when ordering is clear.
    dated = sorted(enumerate(txns), key=lambda pair: (pair[1]["date"], pair[0]))
    first, last = dated[0][1], dated[-1][1]
    source_dates = [t["date"] for t in txns]
    chronological = source_dates == sorted(source_dates)
    ordered = txns
    reverse = list(reversed(txns))
    if not chronological and source_dates == sorted(source_dates, reverse=True):
        balances_valid = all(a.get("balance") is not None and b.get("balance") is not None
                             and abs(a["balance"] + (b["amount"] if b["direction"] == "in" else -b["amount"]) - b["balance"]) < 0.05
                             for a, b in zip(reverse, reverse[1:]))
        if balances_valid:
            ordered = reverse
            first, last = ordered[0], ordered[-1]
            chronological = True
    if chronological and summary.get("openingBalance") is None and first.get("balance") is not None:
        summary["openingBalance"] = round(first["balance"] - (first["amount"] if first["direction"] == "in" else -first["amount"]), 2)
        mapping(report, "summary.openingBalance", first["sourceLines"], "Derived: first running balance minus first signed transaction. This is balance funding, not income.")
    if summary.get("closingBalance") is None and chronological and last.get("balance") is not None:
        summary["closingBalance"] = last["balance"]
    opening, closing = summary.get("openingBalance"), summary.get("closingBalance")
    if opening is not None and closing is not None:
        difference = round(opening + net - closing, 2)
        summary["balanceDifference"] = difference
        if abs(difference) > 0.05:
            warn(report, f"Bank balances do not reconcile: opening + credits - debits differs from closing by {difference:.2f} {report['currency']}. Check missing or ambiguous rows.")
    warn(report, "Categories are suggestions. Review transfers, refunds and uncategorized entries before interpreting income and expenses.")


def analyze(text: str, filename: str = "statement.md") -> dict:
    if not text.strip():
        raise ValueError("The statement is empty.")
    if len(text.encode("utf-8")) > 50_000_000:
        raise ValueError("Converted statement exceeds the 50 MB text limit.")
    return parse_nsdl(text, filename) if is_nsdl(text) else parse_bank(text, filename)
