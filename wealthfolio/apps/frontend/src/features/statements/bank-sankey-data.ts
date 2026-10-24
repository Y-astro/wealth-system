import { expenseAdjustedTransactions } from "./credit-card-data";
import type { StatementTransaction } from "./statement-data";

export type FlowKind = "income" | "expense" | "transfer" | "balance" | "cash";
export type CashTransaction = StatementTransaction & { amount: number };
export interface CashNode {
  key: string;
  name: string;
  kind: FlowKind;
  level: number;
  value: number;
  count: number;
  category?: string;
}
export interface CashLink {
  source: number;
  target: number;
  value: number;
  count: number;
  kind: FlowKind;
  category?: string;
}

const rules: Array<[string, RegExp]> = [
  ["Salary", /salary|payroll/i],
  ["Interest", /interest|\bint\.?(?:paid|cr)\b/i],
  ["Dividends", /dividend/i],
  ["Refunds & reversals", /refund|reversal|reversed|cashback/i],
  ["Groceries", /grocery|groceries|supermarket|bigbasket|blinkit|zepto|dmart|d-mart|instamart/i],
  ["Dining & food delivery", /swiggy|zomato|restaurant|\bcafe\b|coffee|bakery|food court/i],
  ["Housing", /\brent\b|housing|maintenance|apartment|society/i],
  [
    "Utilities & bills",
    /electric|broadband|utilities|water bill|mobile bill|airtel|jio|bescom|gas bill/i,
  ],
  ["Transport & fuel", /\buber\b|\bola\b|metro|petrol|diesel|fuel|fastag|parking|rapido|irctc/i],
  ["Health & pharmacy", /hospital|pharmacy|medical|clinic|apollo|medplus|diagnostic/i],
  ["Education", /school|college|tuition|university|course|udemy|coursera|exam fee/i],
  ["Insurance", /insurance|\blic\b|premium/i],
  ["Shopping", /amazon|flipkart|myntra|shopping|retail|nykaa|ajio/i],
  [
    "Subscriptions & entertainment",
    /netflix|spotify|youtube|hotstar|cinema|pvr|subscription|bookmyshow/i,
  ],
  ["Travel & stays", /hotel|airbnb|makemytrip|goibibo|flight|airlines|indigo/i],
  ["Fees & taxes", /\bfees?\b|\bcharges?\b|\btax\b|\bgst\b|\btds\b/i],
  ["Cash withdrawals", /\batm\b|cash withdrawal/i],
  ["Card repayments", /credit.?card.*(?:payment|repay)|card payment|cc payment/i],
  ["Loan repayments", /\bemi\b|loan repayment/i],
  ["Investments", /\bsip\b|mutual fund|zerodha|groww|fixed deposit|sweep/i],
];
const generic = /^(other (?:expenses|income)|uncategorized(?: expenses| income)?|unknown)$/i;

/** Refine only generic labels; keep categories the user has already reviewed. */
export function spendingCategory(transaction: StatementTransaction): string {
  const current = transaction.category?.trim() || "";
  if (current && !generic.test(current)) return current;
  const matched = rules.find(([, pattern]) => pattern.test(transaction.description));
  return (
    matched?.[0] ||
    (transaction.direction === "out" ? "Uncategorized expenses" : "Uncategorized income")
  );
}

const cents = (value: number) => Math.round(value * 100);
const money = (value: number) => value / 100;

export function cashTransactions(
  transactions: StatementTransaction[],
  currency: string,
): CashTransaction[] {
  return transactions.filter(
    (item): item is CashTransaction =>
      item.unit === "cash" &&
      item.currency === currency &&
      typeof item.amount === "number" &&
      Number.isFinite(item.amount) &&
      item.amount > 0,
  );
}

function groups(transactions: CashTransaction[], key: (item: StatementTransaction) => string) {
  const entries = new Map<string, { name: string; cents: number; count: number }>();
  for (const item of transactions) {
    const name = key(item);
    const group = entries.get(name) || { name, cents: 0, count: 0 };
    group.cents += cents(item.amount);
    group.count++;
    entries.set(name, group);
  }
  return [...entries.values()].sort((a, b) => b.cents - a.cents || a.name.localeCompare(b.name));
}

function descriptionGroup(item: StatementTransaction): string {
  const label = item.description
    .replace(/\b\d{6,}\b/g, "")
    .replace(/^(?:upi|neft|imps|rtgs)[\s/-]*(?:dr|cr)?[\s/-]*/i, "")
    .replace(/\s+/g, " ")
    .trim();
  return label.length > 42 ? label.slice(0, 39) + "…" : label || "Other statement descriptions";
}

/** Every split is computed in paise/cents so branch totals match their parent. */
export function detailedCashFlow(
  transactions: StatementTransaction[],
  currency: string,
  expanded: boolean,
  selectedCategory: string | null = null,
) {
  const original = cashTransactions(transactions, currency);
  const cash = cashTransactions(expenseAdjustedTransactions(original), currency);
  const incoming = cash.filter((item) => item.direction === "in");
  const outgoing = cash.filter((item) => item.direction === "out");
  const expenses = outgoing.filter((item) => item.flowType !== "transfer");
  const transfers = outgoing.filter((item) => item.flowType === "transfer");
  const sum = (items: CashTransaction[]) =>
    items.reduce((total, item) => total + cents(item.amount), 0);
  const credits = sum(incoming),
    debits = sum(outgoing),
    spending = sum(expenses);
  const funding = Math.max(0, debits - credits),
    retained = Math.max(0, credits - debits);
  const available = credits + funding;
  const nodes: CashNode[] = [];
  const links: CashLink[] = [];
  const node = (
    key: string,
    name: string,
    kind: FlowKind,
    level: number,
    amount: number,
    count: number,
    category?: string,
  ) => {
    nodes.push({ key, name, kind, level, value: money(amount), count, category });
    return nodes.length - 1;
  };
  const link = (
    source: number,
    target: number,
    amount: number,
    count: number,
    kind: FlowKind,
    category?: string,
  ) => {
    if (amount > 0) links.push({ source, target, value: money(amount), count, kind, category });
  };
  const center = node("cash", "Cash available", "cash", 1, available, cash.length);
  for (const group of groups(
    incoming,
    (item) =>
      (item.expenseOffset
        ? "Expense credit · "
        : item.flowType === "transfer"
          ? "Transfers in · "
          : "Income · ") + spendingCategory(item),
  )) {
    const kind: FlowKind = group.name.startsWith("Expense credit")
      ? "balance"
      : group.name.startsWith("Transfers")
        ? "transfer"
        : "income";
    const index = node("in:" + group.name, group.name, kind, 0, group.cents, group.count);
    link(index, center, group.cents, group.count, kind);
  }
  if (funding) {
    const index = node("funding", "Balance / other funding", "balance", 0, funding, 0);
    link(index, center, funding, 0, "balance");
  }
  if (spending) {
    const expenseNode = node("expenses", "Expenses", "expense", 2, spending, expenses.length);
    link(center, expenseNode, spending, expenses.length, "expense");
    if (expanded) {
      for (const category of groups(expenses, spendingCategory)) {
        const categoryNode = node(
          "expense:" + category.name,
          category.name,
          "expense",
          3,
          category.cents,
          category.count,
          category.name,
        );
        link(expenseNode, categoryNode, category.cents, category.count, "expense", category.name);
        if (selectedCategory === category.name) {
          const items = expenses.filter((item) => spendingCategory(item) === category.name);
          const descriptions = groups(items, descriptionGroup);
          const shown = descriptions.slice(0, 8);
          if (descriptions.length > 8) {
            const rest = descriptions.slice(8);
            shown.push({
              name: "Other descriptions (" + rest.length + ")",
              cents: rest.reduce((sum, item) => sum + item.cents, 0),
              count: rest.reduce((sum, item) => sum + item.count, 0),
            });
          }
          for (const description of shown) {
            const detail = node(
              "description:" + description.name,
              description.name,
              "expense",
              4,
              description.cents,
              description.count,
              category.name,
            );
            link(
              categoryNode,
              detail,
              description.cents,
              description.count,
              "expense",
              category.name,
            );
          }
        }
      }
    }
  }
  const transferTotal = sum(transfers);
  if (transferTotal) {
    const index = node(
      "transfers",
      "Transfers out",
      "transfer",
      2,
      transferTotal,
      transfers.length,
    );
    link(center, index, transferTotal, transfers.length, "transfer");
  }
  if (retained) {
    const index = node("retained", "Net cash retained", "balance", 2, retained, 0);
    link(center, index, retained, 0, "balance");
  }
  return {
    nodes,
    links,
    expenses,
    cash,
    available: money(available),
    spending: money(spending),
    refundTotal: money(
      sum(original.filter((item) => item.expenseOffset && item.direction === "in")),
    ),
    credits: money(credits),
    transferTotal: money(transferTotal),
    retained: money(retained),
  };
}
