import type { ReceiptDraftDto } from "./api";
import type { Lot } from "./inventory-list";

export function renderReview(
  root: HTMLElement,
  source: ReceiptDraftDto,
  raw: string,
  lots: Lot[],
  onConfirm: (draft: ReceiptDraftDto) => void,
  onCancel: () => void
): void {
  const draft = structuredClone(source);
  root.replaceChildren();
  const heading = document.createElement("h2");
  heading.textContent = `Review receipt ${draft.id || "(identifier needed)"}`;
  root.append(heading);
  const note = document.createElement("p");
  note.textContent = "Nothing enters inventory or spending until you confirm this review.";
  root.append(note);
  if (draft.uncertainty.length) {
    const warning = document.createElement("p");
    warning.className = "warning";
    warning.textContent = `Needs resolution: ${draft.uncertainty.join(", ")}`;
    root.append(warning);
  }
  const columns = document.createElement("div");
  columns.className = "review-columns";
  const sourceBox = document.createElement("pre");
  sourceBox.className = "receipt-source";
  sourceBox.textContent = raw;
  columns.append(sourceBox);
  const fields = document.createElement("div");
  fields.className = "receipt-fields";
  const addField = (
    parent: HTMLElement, labelText: string, name: string, value: string,
    update: (value: string) => void, type = "text"
  ) => {
    const label = document.createElement("label");
    label.textContent = labelText;
    const input = document.createElement("input");
    input.name = name;
    input.type = type;
    input.value = value;
    input.addEventListener("change", () => update(input.value));
    label.append(input);
    parent.append(label);
  };
  addField(fields, "Receipt ID", "receipt-id", draft.id, (value) => {
    const old = draft.id;
    draft.id = value;
    for (const item of draft.items) {
      if (item.line_id.startsWith(`${old}-`)) item.line_id = `${value}${item.line_id.slice(old.length)}`;
    }
  });
  addField(fields, "Purchase date", "purchase-date", draft.purchase_date ?? "", (value) => {
    draft.purchase_date = value || null;
  }, "date");
  for (const item of draft.items) {
    const row = document.createElement("fieldset");
    row.dataset.lineId = item.line_id;
    const legend = document.createElement("legend");
    legend.textContent = item.raw;
    row.append(legend);
    addField(row, "Item", "name", item.name, (value) => { item.name = value; });
    addField(row, "Quantity", "quantity", item.quantity ?? "", (value) => {
      item.quantity = value || null;
    });
    addField(row, "Unit", "unit", item.unit ?? "", (value) => { item.unit = value || null; });
    addField(row, "Gross cents", "gross_cents", String(item.gross_cents ?? ""), (value) => {
      item.gross_cents = value === "" ? null : Number(value);
      item.net_cents = item.gross_cents === null ? null : item.gross_cents + item.discount_cents;
    }, "number");
    addField(row, "Discount cents", "discount_cents", String(item.discount_cents), (value) => {
      item.discount_cents = Number(value);
      item.net_cents = item.gross_cents === null ? null : item.gross_cents + item.discount_cents;
    }, "number");
    addField(row, "Category", "category", item.category, (value) => { item.category = value; });
    addField(row, "Expiry (optional)", "expiry_date", item.expiry_date ?? "", (value) => {
      item.expiry_date = value || null;
    }, "date");
    fields.append(row);
  }
  draft.refunds.forEach((refund, index) => {
    const row = document.createElement("fieldset");
    const legend = document.createElement("legend");
    legend.textContent = `Refund: ${refund.name}, ${refund.amount_cents} cents`;
    row.append(legend);
    addField(row, "Refund category", `refund-category-${index}`, refund.category,
      (value) => { refund.category = value; });
    const label = document.createElement("label");
    label.textContent = "Original inventory lot";
    const select = document.createElement("select");
    select.name = `refund-link-${index}`;
    const empty = document.createElement("option");
    empty.value = "";
    empty.textContent = "No linked stock";
    select.append(empty);
    for (const lot of lots.filter((lot) => lot.name === refund.name)) {
      const option = document.createElement("option");
      option.value = lot.id;
      option.textContent = `${lot.name}: ${lot.quantity} ${lot.unit} available`;
      select.append(option);
    }
    select.value = refund.original_line_id ?? "";
    select.addEventListener("change", () => {
      refund.original_line_id = select.value || null;
    });
    label.append(select);
    row.append(label);
    addField(row, "Quantity to remove from stock", `refund-reverse-${index}`,
      refund.inventory_quantity_to_reverse ?? "0",
      (value) => { refund.inventory_quantity_to_reverse = value || "0"; }, "number");
    fields.append(row);
  });
  addField(fields, "Printed subtotal, cents", "printed-subtotal",
    String(draft.printed_subtotal_cents ?? ""), (value) => {
      draft.printed_subtotal_cents = value === "" ? null : Number(value);
    }, "number");
  addField(fields, "Printed tax, cents", "printed-tax",
    String(draft.printed_tax_cents ?? ""), (value) => {
      draft.printed_tax_cents = value === "" ? null : Number(value);
    }, "number");
  addField(fields, "Printed total, cents", "printed-total",
    String(draft.printed_total_cents ?? ""), (value) => {
      draft.printed_total_cents = value === "" ? null : Number(value);
    }, "number");
  columns.append(fields);
  root.append(columns);
  const actions = document.createElement("div");
  actions.className = "actions";
  const confirm = document.createElement("button");
  confirm.type = "button";
  confirm.dataset.action = "confirm";
  confirm.textContent = "Confirm receipt and update inventory";
  confirm.addEventListener("click", () => {
    draft.known_subtotal_cents = draft.items.reduce(
      (sum, item) => sum + (item.net_cents ?? 0), 0
    ) + draft.refunds.reduce((sum, item) => sum + item.amount_cents, 0);
    onConfirm(structuredClone(draft));
  });
  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.dataset.action = "cancel";
  cancel.textContent = "Cancel import";
  cancel.addEventListener("click", onCancel);
  actions.append(confirm, cancel);
  root.append(actions);
}
