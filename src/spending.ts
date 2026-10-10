import type { BudgetDto } from "./api";

const money = (cents: number) => `$${(cents / 100).toFixed(2)}`;

export function renderSpending(root: HTMLElement, view: BudgetDto, onSave: (cents: number | null) => void): void {
  root.replaceChildren();
  const heading = document.createElement("h3");
  heading.textContent = `${view.period === "week" ? "Weekly" : "Monthly"} view: ${view.start_date} to ${view.end_date}`;
  root.append(heading);
  const form = document.createElement("div"); form.className = "budget-form";
  const label = document.createElement("label"); label.textContent = "Budget, dollars ";
  const input = document.createElement("input"); input.name = "budget-dollars"; input.inputMode = "decimal";
  input.value = view.budget_cents === null ? "" : (view.budget_cents / 100).toFixed(2);
  label.append(input);
  const save = document.createElement("button"); save.type = "button"; save.dataset.action = "save-budget"; save.textContent = "Save budget";
  const validation = document.createElement("span"); validation.setAttribute("role", "status");
  save.addEventListener("click", () => {
    const value = input.value.trim();
    if (!value) { onSave(null); return; }
    if (!/^\d+(?:\.\d{1,2})?$/.test(value)) { validation.textContent = "Enter a nonnegative amount with up to two decimals."; return; }
    const [whole, fractional = ""] = value.split(".");
    const cents = Number(whole) * 100 + Number(fractional.padEnd(2, "0"));
    if (!Number.isSafeInteger(cents)) { validation.textContent = "Budget amount is too large."; return; }
    validation.textContent = "";
    onSave(cents);
  });
  form.append(label, save, validation); root.append(form);
  const values: Array<[string, number | null]> = [
    ["Receipt cash flow", view.cash_flow_cents], ["Food purchases and refunds", view.food_spending_cents],
    ["Household goods", view.household_goods_cents], ["Tax", view.tax_cents],
    ["Gross items", view.gross_items_cents], ["Item discounts", view.discounts_cents],
    ["Refunds", view.refunds_cents], ["Remaining budget", view.remaining_budget_cents]
  ];
  for (const [name, value] of values) {
    const row = document.createElement("div"); row.className = "spending-row";
    const label = document.createElement("span"); label.textContent = name;
    const amount = document.createElement("strong"); amount.textContent = value === null ? "Not set" : money(value);
    row.append(label, amount); root.append(row);
  }
  const note = document.createElement("p"); note.className = "hint";
  note.textContent = "Discounts and refunds are already included in cash flow. Discarded food is separate from purchase spending.";
  root.append(note);
  if (view.reconciliation_cents !== 0) {
    const warning = document.createElement("p"); warning.className = "warning";
    warning.textContent = `Reconciliation difference: ${money(view.reconciliation_cents)}.`;
    root.append(warning);
  }
}
