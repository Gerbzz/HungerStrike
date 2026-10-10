// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import { mountApp } from "../../src/app";
import type { ReceiptDraftDto, SnapshotDto } from "../../src/api";

const blank: SnapshotDto = {
  receipt_count: 0, cash_flow_cents: 0, food_spending_cents: 0,
  household_goods_cents: 0, tax_cents: 0, consumed_cost_cents: 0,
  discarded_cost_cents: 0, remaining_food_cost_cents: 0,
  refund_without_stock_reversal_cents: 0, lots: [], receipt_items: [], category_edits: []
};
const projectionApi = {
  meals: async () => [],
  budget: async (_date: string, period: "week" | "month") => ({
    period, start_date: "2026-10-01", end_date: "2026-10-31", receipt_count: 0,
    budget_cents: null, remaining_budget_cents: null, cash_flow_cents: 0,
    food_spending_cents: 0, household_goods_cents: 0, tax_cents: 0,
    gross_items_cents: 0, discounts_cents: 0, refunds_cents: 0, reconciliation_cents: 0
  }),
  saveBudget: async () => {},
  editCategory: async () => blank,
  exportBackupFile: async () => null,
  previewBackup: async () => ({ incoming_receipts: 0, existing_receipts: 0, duplicate_receipts: 0, conflicting_identifiers: 0, incoming_lots: 0, replace_will_create_restore_point: true }),
  applyBackup: async () => ({ outcome: "duplicate" as const, snapshot: blank }),
  undoLastReplace: async () => blank
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
    await mountApp(root, { ...projectionApi, importText, scanPhoto: vi.fn(), confirm, getSnapshot, action: vi.fn(async () => blank) });
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

  it("keeps manual entry available when offline photo OCR is unavailable", async () => {
    const confirm = vi.fn(async () => "accepted" as const);
    const scanPhoto = vi.fn(async () => { throw new Error("offline OCR is unavailable"); });
    const root = document.createElement("div");
    await mountApp(root, { ...projectionApi, importText: vi.fn(), scanPhoto, confirm, getSnapshot: async () => blank, action: vi.fn() });
    const input = root.querySelector<HTMLInputElement>('input[name="receipt-photo"]')!;
    const file = new File(["fake image"], "receipt.png", { type: "image/png" });
    file.arrayBuffer = async () => new Uint8Array([137, 80, 78, 71]).buffer;
    Object.defineProperty(input, "files", { value: [file] });
    input.dispatchEvent(new Event("change"));
    await vi.waitFor(() => expect(scanPhoto).toHaveBeenCalled());
    expect(root.querySelector("textarea[name=receipt-text]")).not.toBeNull();
    expect(confirm).not.toHaveBeenCalled();
    expect(root.textContent).toContain("offline OCR is unavailable");
  });

  it("shows scanned text for review without confirming it", async () => {
    const confirm = vi.fn(async () => "accepted" as const);
    const root = document.createElement("div");
    await mountApp(root, {
      ...projectionApi, importText: vi.fn(), scanPhoto: async () => ({ text: "SYNTHETIC OCR TEXT", draft }),
      confirm, getSnapshot: async () => blank, action: vi.fn()
    });
    const input = root.querySelector<HTMLInputElement>('input[name="receipt-photo"]')!;
    const file = new File(["fake image"], "receipt.png", { type: "image/png" });
    file.arrayBuffer = async () => new Uint8Array([137, 80, 78, 71]).buffer;
    Object.defineProperty(input, "files", { value: [file] });
    input.dispatchEvent(new Event("change"));
    await vi.waitFor(() => expect(root.querySelector("button[data-action=confirm]")).not.toBeNull());
    expect(root.textContent).toContain("SYNTHETIC OCR TEXT");
    expect(confirm).not.toHaveBeenCalled();
  });
});
