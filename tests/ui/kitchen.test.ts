import { describe, expect, it } from "vitest";
import { renderInventoryList } from "../../src/inventory-list";
import { renderKitchen, zoneFromHitName } from "../../src/kitchen";

const lots = [
  { id: "R001-L03", name: "large eggs", quantity: "6", unit: "egg", zone: "fridge", remaining_cost_cents: 174 },
  { id: "R001-L04", name: "pasta", quantity: "2", unit: "each", zone: "pantry", remaining_cost_cents: 179 }
];

describe("fixed kitchen and accessible inventory", () => {
  it("offers every lot and location action in the semantic list", () => {
    const root = document.createElement("div");
    let moved = "";
    renderInventoryList(root, lots, (id, zone) => { moved = `${id}:${zone}`; });
    expect(root.querySelectorAll("li")).toHaveLength(2);
    expect(root.textContent).toContain("large eggs");
    const select = root.querySelector<HTMLSelectElement>('li[data-lot-id="R001-L03"] select')!;
    select.value = "freezer";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    expect(moved).toBe("R001-L03:freezer");
  });

  it("keeps the list usable when WebGL is unavailable", () => {
    const root = document.createElement("div");
    const result = renderKitchen(root, lots, () => {}, false);
    expect(result).toBe("list");
    expect(root.textContent).toContain("3D view unavailable");
    expect(root.textContent).toContain("large eggs");
  });

  it("maps all three clickable scene zones", () => {
    expect(zoneFromHitName("zone-fridge")).toBe("fridge");
    expect(zoneFromHitName("zone-freezer")).toBe("freezer");
    expect(zoneFromHitName("zone-pantry")).toBe("pantry");
  });
});

it("offers keyboard-operable consume and discard controls with explicit quantities", () => {
  const root = document.createElement("div");
  const actions: Array<[string, string, string]> = [];
  renderInventoryList(root, lots, () => {}, (id, kind, quantity) => actions.push([id, kind, quantity]));
  const first = root.querySelector<HTMLElement>('li[data-lot-id="R001-L03"]')!;
  const amount = first.querySelector<HTMLInputElement>('input[name="action-quantity"]')!;
  amount.value = "0.5";
  first.querySelector<HTMLButtonElement>('button[data-action="discard"]')!.click();
  expect(actions).toEqual([["R001-L03", "discard", "0.5"]]);
});
