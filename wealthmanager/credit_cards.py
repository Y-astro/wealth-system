"""Credit-card credits are payments or expense adjustments, never earnings."""
import re
from collections import defaultdict, deque


def is_credit_card_filename(filename):
    # A CC token or CCStatement suffix. Do not mistake 'account'/'Acct' for CC.
    return bool(re.search(r"(?:^|[^a-z])cc(?:[^a-z]|$)|cc[ _-]*statement", filename, re.I))


def apply_credit_card(report):
    if report.get("kind") != "bank" or not (report.get("statementType") == "credit-card" or is_credit_card_filename(report.get("filename", ""))):
        return False
    from statement_parser import CATEGORIES, mapping, warn
    report["statementType"] = "credit-card"
    normalize = lambda text: re.sub(r"\s+", " ", text.lower()).strip()
    debit_lists = defaultdict(list)
    for i, debit in enumerate(report["transactions"]):
        if debit.get("unit") == "cash" and debit["direction"] == "out" and debit["flowType"] == "expense":
            debit_lists[(debit.get("currency"), debit.get("amount"), normalize(debit["description"]))].append((i, debit))
    debit_pools = {key: deque(sorted(value, key=lambda pair: (pair[1]["date"], pair[0]))) for key, value in debit_lists.items()}
    for index, row in enumerate(report["transactions"]):
        if row.get("unit") != "cash":
            continue
        description = row["description"]
        payment = bool(re.search(r"bbps.*payment|payment.*received|payment.*thank|thank.*payment|card.*(?:payment|repay)|(?:autopay|auto.?debit)|payment credited", description, re.I))
        pool = debit_pools.get((row.get("currency"), row.get("amount"), normalize(description)))
        matched = pool[0] if row["direction"] == "in" and pool and pool[0][1]["date"] <= row["date"] else None
        tax = bool(re.search(r"\b(?:[ics]gst|gst)\b", description, re.I))
        if row["direction"] == "in":
            if row.get("cardCreditKind") in ("payment", "refund", "unclassified"):
                kind = row["cardCreditKind"]
            else:
                kind = "payment" if payment else ("refund" if matched or re.search(r"fuel|refund|revers|\brev\b|cashback|waiv|chargeback", description, re.I) else "unclassified")
            row["cardCreditKind"] = kind
            row["expenseOffset"] = kind == "refund"
            row["flowType"] = "expense" if kind == "refund" else "transfer"
            if kind == "payment":
                row["category"] = "Card repayments"
            elif kind == "unclassified":
                row["category"] = "Other card credits"
            elif row.get("category") in (None, "", "Other income", "Refunds", "Refunds & reversals", "Other card credits"):
                row["category"] = matched[1]["category"] if matched else ("Fees & taxes" if tax else next((name for name, pattern in CATEGORIES if name != "Refunds" and re.search(pattern, description, re.I)), "Other expenses"))
            if kind == "refund" and matched:
                pool.popleft()
            target = f"transactions[{index}].cardCreditKind"
            if not any(m["target"] == target for m in report["mappings"]):
                mapping(report, target, row["sourceLines"] + (matched[1]["sourceLines"] if kind == "refund" and matched else []), "Credit card: matched merchant credit has the same description, amount and currency as an earlier debit." if kind == "refund" and matched else "Credit card: payments are transfers; fuel credits/refunds/waivers offset expense categories. Other credits require review; none are earnings.")
        else:
            row.pop("expenseOffset", None)
            if tax and row.get("category") in ("Other expenses", "Fees & taxes"):
                row["category"] = "Fees & taxes"
            if payment and re.search(r"revers|return|reject", description, re.I):
                row["flowType"] = "transfer"
                row["cardCreditKind"] = "payment"
                row["category"] = "Card repayments"
    warn(report, "Credit-card payments are transfers, not income. Refunds and fuel credits reduce expenses. Payment coverage compares this statement's transactions; payments can settle an earlier bill and are not proof of the amount due.")
    report["warnings"] = [w for w in report["warnings"] if w != "Unclassified card credits need review before confirming whether payments match net expenses."]
    if any(row.get("cardCreditKind") == "unclassified" for row in report["transactions"]):
        warn(report, "Unclassified card credits need review before confirming whether payments match net expenses.")
    return True


def finalize_card(report):
    from statement_parser import money_sum, mapping, warn
    rows = report["transactions"]
    summary = report["summary"]
    charges = money_sum(t["amount"] for t in rows if t["direction"] == "out" and t["flowType"] == "expense")
    refunds = money_sum(t["amount"] for t in rows if t.get("expenseOffset") and t["direction"] == "in")
    payments = money_sum(t["amount"] * (1 if t["direction"] == "in" else -1) for t in rows if t.get("cardCreditKind") == "payment")
    other = money_sum(t["amount"] for t in rows if t.get("cardCreditKind") == "unclassified")
    summary.update(income=0, expenses=money_sum([charges, -refunds]), grossCharges=charges,
                   expenseRefunds=refunds, cardPayments=payments, unclassifiedCredits=other,
                   paymentDifference=money_sum([charges, -refunds, -payments]))
    # A credit-card running balance is a liability: debits increase the amount
    # owed, credits reduce it. Preserve genuine statement balances only.
    dates = [t["date"] for t in rows]
    ordered = rows
    reverse = list(reversed(rows))
    if dates != sorted(dates) and dates == sorted(dates, reverse=True):
        if all(a.get("balance") is not None and b.get("balance") is not None and
               abs(a["balance"] + (b["amount"] if b["direction"] == "out" else -b["amount"]) - b["balance"]) < .05
               for a, b in zip(reverse, reverse[1:])):
            ordered = reverse
    if [t["date"] for t in ordered] == sorted(dates):
        first, last = ordered[0], ordered[-1]
        if summary.get("openingBalance") is None and first.get("balance") is not None:
            summary["openingBalance"] = round(first["balance"] - (first["amount"] if first["direction"] == "out" else -first["amount"]), 2)
            mapping(report, "summary.openingBalance", first["sourceLines"], "Credit-card liability: first running balance minus debit, or plus credit.")
        if summary.get("closingBalance") is None and last.get("balance") is not None:
            summary["closingBalance"] = last["balance"]
    if summary.get("openingBalance") is not None and summary.get("closingBalance") is not None:
        difference = round(summary["openingBalance"] - summary["netCashFlow"] - summary["closingBalance"], 2)
        summary["balanceDifference"] = difference
        if abs(difference) > .05:
            warn(report, f"Card balances do not reconcile: opening + charges - credits differs from closing by {difference:.2f} {report['currency']}.")
