export type Lot = {
  id: string;
  name: string;
  quantity: string;
  unit: string;
  zone: string;
  remaining_cost_cents: number;
};

export function renderInventoryList(
  root: HTMLElement,
  lots: Lot[],
  onMove: (id: string, zone: string) => void,
  onAdjust?: (id: string, kind: "consume" | "discard", quantity: string) => void
): void {
  root.replaceChildren();
  const section = document.createElement("section");
  section.setAttribute("aria-label", "Inventory list");
  const heading = document.createElement("h3");
  heading.textContent = "Inventory list";
  section.append(heading);
  const list = document.createElement("ul");
  for (const lot of lots) {
    const item = document.createElement("li");
    item.dataset.lotId = lot.id;
    const summary = document.createElement("span");
    summary.textContent = `${lot.name}: ${lot.quantity} ${lot.unit} in ${lot.zone}`;
    const label = document.createElement("label");
    label.textContent = `Move ${lot.name} to `;
    const select = document.createElement("select");
    select.setAttribute("aria-label", `Move ${lot.name} to`);
    for (const zone of ["fridge", "freezer", "pantry"]) {
      const option = document.createElement("option");
      option.value = zone;
      option.textContent = zone;
      select.append(option);
    }
    select.value = lot.zone;
    select.addEventListener("change", () => onMove(lot.id, select.value));
    label.append(select);
    item.append(summary, label);
    if (onAdjust) {
      const controls = document.createElement("div"); controls.className = "stock-actions";
      const quantityLabel = document.createElement("label");
      quantityLabel.textContent = `Amount of ${lot.name} (${lot.unit}) `;
      const quantity = document.createElement("input");
      quantity.name = "action-quantity"; quantity.type = "number"; quantity.min = "0.001";
      quantity.step = "any"; quantity.value = "1";
      quantityLabel.append(quantity); controls.append(quantityLabel);
      for (const kind of ["consume", "discard"] as const) {
        const button = document.createElement("button"); button.type = "button";
        button.dataset.action = kind;
        button.textContent = kind === "consume" ? "Use" : "Discard";
        button.addEventListener("click", () => {
          if (Number(quantity.value) > 0) onAdjust(lot.id, kind, quantity.value);
        });
        controls.append(button);
      }
      item.append(controls);
    }
    list.append(item);
  }
  if (lots.length === 0) {
    const empty = document.createElement("p");
    empty.textContent = "No items in this view.";
    section.append(empty);
  }
  section.append(list);
  root.append(section);
}
