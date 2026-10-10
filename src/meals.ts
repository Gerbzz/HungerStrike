import type { MealDto } from "./api";

const money = (cents: number) => `$${(cents / 100).toFixed(2)}`;

export function renderMeals(root: HTMLElement, meals: MealDto[]): void {
  root.replaceChildren();
  if (!meals.length) { root.textContent = "No recipes are available yet."; return; }
  for (const meal of meals) {
    const card = document.createElement("article"); card.className = "meal-card";
    const title = document.createElement("h3"); title.textContent = meal.name;
    const status = document.createElement("p");
    status.textContent = meal.available ? "Available from confirmed inventory" : "Missing ingredients";
    const cost = document.createElement("strong");
    cost.textContent = `${meal.available ? "Estimated ingredient cost" : "Partial estimate from available stock"}: ${meal.estimate_cents === null ? "Unknown" : money(meal.estimate_cents)}`;
    const list = document.createElement("ul");
    for (const ingredient of meal.ingredients) {
      const row = document.createElement("li");
      row.textContent = `${ingredient.required_quantity} ${ingredient.unit} ${ingredient.name}: ${money(ingredient.estimated_cost_cents)} from stock`;
      if (Number(ingredient.missing_quantity) > 0) row.textContent += `; ${ingredient.missing_quantity} ${ingredient.unit} ${ingredient.name} missing`;
      list.append(row);
    }
    const expiry = document.createElement("p"); expiry.className = "hint";
    const notices = [];
    if (meal.soon_expiring.length) notices.push(`Soon expiring: ${meal.soon_expiring.join(", ")}`);
    if (meal.expiry_unknown) notices.push("Expiry unknown for one or more ingredients");
    expiry.textContent = notices.join(". ");
    card.append(title, status, cost, list, expiry);
    root.append(card);
  }
}
