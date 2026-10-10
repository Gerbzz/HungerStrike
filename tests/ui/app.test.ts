// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import { mountApp } from "../../src/app";
import type { ReceiptDraftDto, SnapshotDto } from "../../src/api";

const blank: SnapshotDto = {
  receipt_count: 0, cash_flow_cents: 0, food_spending_cents: 0,
  household_goods_cents: 0, tax_cents: 0, consumed_cost_cents: 0,
  discarded_cost_cents: 0, remaining_food_cost_cents: 0,
  refund_without_stock_reversal_cents: 0, lots: []
};
const draft: ReceiptDraftDto = {
  id: "R001", purchase_date: "2026-10-10", source_sha256: "abc",
  items: [], refunds: [], unallocated_adjustments: [], known_subtotal_cents: 0,
  printed_subtotal_cents: 0, printed_tax_cents: 0, printed_total_cents: 0,
  status: "pending_confirmation", uncertainty: []
};

describe("local receipt workflow", () => {
  it("imports a draft without a write, cancels, then confirms only on explicit action", async () => {
    const importText = vi.fn(async () => draft);
    const confirm = vi.fn(async () => "accepted" as const);
    const getSnapshot = vi.fn(async () => blank);
    const root = document.createElement("div");
    await mountApp(root, { importText, confirm, getSnapshot, action: vi.fn(async () => blank) });
    (root.querySelector("textarea[name=receipt-text]") as HTMLTextAreaElement).value = "test";
    (root.querySelector("button[data-action=import]") as HTMLButtonElement).click();
    await vi.waitFor(() => expect(importText).toHaveBeenCalledWith("test"));
    expect(confirm).not.toHaveBeenCalled();
    (root.querySelector("button[data-action=cancel]") as HTMLButtonElement).click();
    expect(confirm).not.toHaveBeenCalled();
    (root.querySelector("button[data-action=import]") as HTMLButtonElement).click();
    await vi.waitFor(() => expect(root.querySelector("button[data-action=confirm]")).not.toBeNull());
    (root.querySelector("button[data-action=confirm]") as HTMLButtonElement).click();
    await vi.waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
  });
});
