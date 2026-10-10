import type { Lot } from "./inventory-list";
import { invoke } from "@tauri-apps/api/core";

export type DraftItemDto = {
  line_id: string;
  raw: string;
  name: string;
  quantity: string | null;
  unit: string | null;
  gross_cents: number | null;
  discount_cents: number;
  net_cents: number | null;
  category: string;
  expiry_date: string | null;
  uncertainty: string[];
};

export type RefundDto = {
  name: string;
  category: string;
  quantity: string | null;
  unit: string | null;
  amount_cents: number;
  original_line_id: string | null;
  inventory_quantity_to_reverse: string | null;
};

export type ReceiptDraftDto = {
  id: string;
  purchase_date: string | null;
  source_sha256: string;
  items: DraftItemDto[];
  refunds: RefundDto[];
  unallocated_adjustments: string[];
  known_subtotal_cents: number;
  printed_subtotal_cents: number | null;
  printed_tax_cents: number | null;
  printed_total_cents: number | null;
  status: "pending_confirmation" | "needs_resolution";
  uncertainty: string[];
};

export type SnapshotDto = {
  receipt_count: number;
  cash_flow_cents: number;
  food_spending_cents: number;
  household_goods_cents: number;
  tax_cents: number;
  consumed_cost_cents: number;
  discarded_cost_cents: number;
  remaining_food_cost_cents: number;
  refund_without_stock_reversal_cents: number;
  lots: Array<Lot & {
    source_receipt_id: string;
    category: string;
    expiry_date: string | null;
    purchase_date: string;
  }>;
  receipt_items: Array<{ receipt_id: string; line_id: string; name: string; category: string; net_cents: number }>;
  category_edits: Array<{ sequence: number; receipt_id: string; line_id: string; from: string; to: string }>;
};

export type MealDto = {
  id: string; name: string; servings: number; steps: string[]; available: boolean;
  estimate_cents: number | null; estimate_status: string;
  ingredients: Array<{ name: string; required_quantity: string; unit: string; stock_used_quantity: string; estimated_cost_cents: number; missing_quantity: string }>;
  soon_expiring: string[]; expiry_unknown: boolean;
};

export type PeriodDto = "week" | "month";
export type BudgetDto = {
  period: PeriodDto; start_date: string; end_date: string; receipt_count: number;
  budget_cents: number | null; remaining_budget_cents: number | null;
  cash_flow_cents: number; food_spending_cents: number; household_goods_cents: number;
  tax_cents: number; gross_items_cents: number; discounts_cents: number;
  refunds_cents: number; reconciliation_cents: number;
};

export type InventoryActionDto =
  | { kind: "move"; lot_id: string; zone: string }
  | { kind: "consume" | "discard"; lot_id: string; quantity: string };

export interface AppApi {
  importText(raw: string): Promise<ReceiptDraftDto>;
  scanPhoto(bytes: number[]): Promise<{ text: string; draft: ReceiptDraftDto }>;
  confirm(draft: ReceiptDraftDto): Promise<"accepted" | "duplicate">;
  getSnapshot(): Promise<SnapshotDto>;
  action(action: InventoryActionDto): Promise<SnapshotDto>;
  meals(today: string): Promise<MealDto[]>;
  budget(referenceDate: string, period: PeriodDto, weekStart: number): Promise<BudgetDto>;
  saveBudget(period: PeriodDto, cents: number | null): Promise<void>;
  editCategory(lineId: string, category: string): Promise<SnapshotDto>;
}

export const tauriApi: AppApi = {
  importText: (raw) => invoke("import_text", { raw }),
  scanPhoto: (bytes) => invoke("scan_photo", { bytes }),
  confirm: (reviewed) => invoke("confirm_reviewed", { reviewed }),
  getSnapshot: () => invoke("get_snapshot"),
  action: (action) => invoke("inventory_action", { action })
  ,meals: (today) => invoke("get_meals", { today })
  ,budget: (referenceDate, period, weekStart) => invoke("get_budget", { referenceDate, period, weekStart })
  ,saveBudget: (period, cents) => invoke("save_budget", { period, cents })
  ,editCategory: (lineId, category) => invoke("edit_category", { lineId, category })
};
