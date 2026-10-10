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
    await mountApp(root, { importText, scanPhoto: vi.fn(), confirm, getSnapshot, action: vi.fn(async () => blank) });
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
    await mountApp(root, { importText: vi.fn(), scanPhoto, confirm, getSnapshot: async () => blank, action: vi.fn() });
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
      importText: vi.fn(), scanPhoto: async () => ({ text: "SYNTHETIC OCR TEXT", draft }),
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
