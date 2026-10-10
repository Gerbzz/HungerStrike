import { expect, it } from "vitest";
import { renderReview } from "../../src/review";
import type { ReceiptDraftDto } from "../../src/api";

const draft: ReceiptDraftDto = {
  id: "R002",
  purchase_date: "2026-10-04",
  source_sha256: "fixture-hash",
  items: [{
    line_id: "R002-L01", raw: "APPLES 0.80 LB 1.19", name: "apples",
    quantity: "0.80", unit: "lb", gross_cents: 119, discount_cents: 0,
    net_cents: 119, category: "food.produce", expiry_date: null, uncertainty: []
  }],
  refunds: [{
    name: "milk 2%", category: "food.dairy", quantity: "1", unit: "gal", amount_cents: -429,
    original_line_id: null, inventory_quantity_to_reverse: null
  }],
  unallocated_adjustments: [],
  known_subtotal_cents: -310,
  printed_subtotal_cents: -310,
  printed_tax_cents: 0,
  printed_total_cents: -310,
  status: "pending_confirmation",
  uncertainty: []
};

it("shows raw source and submits reviewed category and explicit refund link", () => {
  const root = document.createElement("div");
  let submitted: ReceiptDraftDto | null = null;
  renderReview(root, draft, "RAW SYNTHETIC RECEIPT", [{
    id: "R001-L02", name: "milk 2%", quantity: "1", unit: "gal", zone: "fridge",
    remaining_cost_cents: 429
  }], (value) => { submitted = value; }, () => {});
  expect(root.textContent).toContain("RAW SYNTHETIC RECEIPT");
  const category = root.querySelector<HTMLInputElement>('[data-line-id="R002-L01"] [name="category"]')!;
  category.value = "food.fruit";
  category.dispatchEvent(new Event("change", { bubbles: true }));
  const link = root.querySelector<HTMLSelectElement>('[name="refund-link-0"]')!;
  link.value = "R001-L02";
  link.dispatchEvent(new Event("change", { bubbles: true }));
  const quantity = root.querySelector<HTMLInputElement>('[name="refund-reverse-0"]')!;
  quantity.value = "1";
  quantity.dispatchEvent(new Event("change", { bubbles: true }));
  root.querySelector<HTMLButtonElement>('[data-action="confirm"]')!.click();
  expect(submitted!.items[0].category).toBe("food.fruit");
  expect(submitted!.refunds[0].original_line_id).toBe("R001-L02");
  expect(submitted!.refunds[0].inventory_quantity_to_reverse).toBe("1");
  expect(draft.refunds[0].original_line_id).toBeNull();
});

it("cancelling a receipt draft never calls confirmation", () => {
  const root = document.createElement("div");
  let confirmations = 0;
  let cancellations = 0;
  renderReview(root, draft, "RAW", [], () => { confirmations++; }, () => { cancellations++; });
  root.querySelector<HTMLButtonElement>('[data-action="cancel"]')!.click();
  expect(confirmations).toBe(0);
  expect(cancellations).toBe(1);
});
