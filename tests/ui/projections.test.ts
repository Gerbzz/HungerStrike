// @vitest-environment jsdom
import { expect, it, vi } from "vitest";
import { renderMeals } from "../../src/meals";
import { renderSpending } from "../../src/spending";
import type { BudgetDto, MealDto } from "../../src/api";

it("labels partial meal estimates and missing ingredients", () => {
  const root = document.createElement("div");
  const meal: MealDto = {
    id: "tomato-pasta", name: "Tomato pasta", servings: 1, steps: [], available: false,
    estimate_cents: 90, estimate_status: "partial_missing_ingredient",
    ingredients: [
      { name: "pasta", required_quantity: "1", unit: "each", stock_used_quantity: "1", estimated_cost_cents: 90, missing_quantity: "0" },
      { name: "tomatoes", required_quantity: "0.25", unit: "lb", stock_used_quantity: "0", estimated_cost_cents: 0, missing_quantity: "0.25" }
    ], soon_expiring: [], expiry_unknown: true
  };
  renderMeals(root, [meal]);
  expect(root.textContent).toContain("Partial estimate");
  expect(root.textContent).toContain("0.25 lb tomatoes missing");
  expect(root.textContent).toContain("$0.90");
  expect(root.textContent).toContain("Expiry unknown");
});

it("shows cash flow separately and saves a budget in integer cents", () => {
  const root = document.createElement("div");
  const save = vi.fn();
  const view: BudgetDto = {
    period: "month", start_date: "2026-10-01", end_date: "2026-10-31", receipt_count: 2,
    budget_cents: 3000, remaining_budget_cents: 275, cash_flow_cents: 2725,
    food_spending_cents: 1711, household_goods_cents: 948, tax_cents: 66,
    gross_items_cents: 3333, discounts_cents: -179, refunds_cents: -429, reconciliation_cents: 0
  };
  renderSpending(root, view, save);
  expect(root.textContent).toContain("$27.25");
  expect(root.textContent).toContain("$17.11");
  expect(root.textContent).toContain("Discarded food is separate");
  const budget = root.querySelector<HTMLInputElement>('input[name="budget-dollars"]')!;
  budget.value = "35.50";
  root.querySelector<HTMLButtonElement>('button[data-action="save-budget"]')!.click();
  expect(save).toHaveBeenCalledWith(3550);
});
