import type { AppApi, ReceiptDraftDto, SnapshotDto } from "./api";
import { renderBackup } from "./backup";
import { renderInventoryList } from "./inventory-list";
import { renderKitchen } from "./kitchen";
import { renderMeals } from "./meals";
import { renderReview } from "./review";
import { renderSpending } from "./spending";

const dollars = (cents: number) => `$${(cents / 100).toFixed(2)}`;

export async function mountApp(root: HTMLElement, api: AppApi): Promise<void> {
  root.innerHTML = `
    <header><div><p class="eyebrow">LOCAL KITCHEN</p><h1>HungerStrike</h1></div><p class="header-note">Your receipts and inventory stay on this device.</p></header>
    <main>
      <section class="panel intro"><h2>Receipt intake</h2><p>Paste receipt text, inspect every line, then confirm before any inventory or spending changes.</p>
        <label for="receipt-text">Receipt text</label><textarea id="receipt-text" name="receipt-text" rows="7" placeholder="Paste receipt text here"></textarea>
        <div class="actions"><button type="button" data-action="import">Review receipt</button><label class="file-label">Open a text file<input type="file" accept=".txt,text/plain" name="receipt-file"></label><label class="file-label">Scan a photo locally<input type="file" accept="image/png,image/jpeg" name="receipt-photo"></label></div>
        <div id="photo-preview"></div><p class="hint">Photo text is a draft. Correct OCR errors before confirming. If offline OCR is unavailable, enter receipt text above.</p></section>
      <p id="app-status" class="status" role="status" aria-live="polite"></p>
      <section id="review-panel" class="panel" hidden></section>
      <section class="panel"><div class="section-heading"><div><p class="eyebrow">INVENTORY</p><h2>Kitchen</h2></div><span id="receipt-count"></span></div><div id="kitchen-view"></div><label>Search inventory <input type="search" name="inventory-search" placeholder="Item name"></label><div id="inventory-view"></div></section>
      <section class="panel"><p class="eyebrow">FROM CONFIRMED STOCK</p><h2>Meal ideas</h2><div id="meals-view"></div></section>
      <section class="panel"><p class="eyebrow">CONFIRMED RECEIPTS</p><h2>Spending</h2>
        <div class="budget-controls"><label>View date <input type="date" name="view-date"></label><label>Week starts <select name="week-start"><option value="0">Monday</option><option value="1">Tuesday</option><option value="2">Wednesday</option><option value="3">Thursday</option><option value="4">Friday</option><option value="5">Saturday</option><option value="6">Sunday</option></select></label></div>
        <div id="spending-view"></div><div class="budget-grid"><div id="week-view"></div><div id="month-view"></div></div>
        <h3>Receipt item categories</h3><p class="hint">Category edits are recorded. Moving a used food item to non-food is blocked to protect inventory history.</p><div id="category-view"></div>
      </section>
      <section class="panel"><p class="eyebrow">LOCAL RECOVERY</p><h2>Backup and restore</h2><div id="backup-view"></div></section>
    </main>`;
  const textarea = root.querySelector<HTMLTextAreaElement>("textarea[name=receipt-text]")!;
  const reviewPanel = root.querySelector<HTMLElement>("#review-panel")!;
  const status = root.querySelector<HTMLElement>("#app-status")!;
  const listRoot = root.querySelector<HTMLElement>("#inventory-view")!;
  const kitchenRoot = root.querySelector<HTMLElement>("#kitchen-view")!;
  const inventorySearch = root.querySelector<HTMLInputElement>('input[name="inventory-search"]')!;
  const spendingRoot = root.querySelector<HTMLElement>("#spending-view")!;
  const mealsRoot = root.querySelector<HTMLElement>("#meals-view")!;
  const weekRoot = root.querySelector<HTMLElement>("#week-view")!;
  const monthRoot = root.querySelector<HTMLElement>("#month-view")!;
  const categoriesRoot = root.querySelector<HTMLElement>("#category-view")!;
  const backupRoot = root.querySelector<HTMLElement>("#backup-view")!;
  const viewDate = root.querySelector<HTMLInputElement>('input[name="view-date"]')!;
  const weekStart = root.querySelector<HTMLSelectElement>('select[name="week-start"]')!;
  const current = new Date();
  viewDate.value = `${current.getFullYear()}-${String(current.getMonth() + 1).padStart(2, "0")}-${String(current.getDate()).padStart(2, "0")}`;
  let snapshot: SnapshotDto = await api.getSnapshot();
  let activeZone: string | null = null;

  const showError = (error: unknown) => { status.textContent = String(error); status.className = "status error"; };
  const loadProjections = async () => {
    try {
      const [meals, week, month] = await Promise.all([
        api.meals(viewDate.value), api.budget(viewDate.value, "week", Number(weekStart.value)),
        api.budget(viewDate.value, "month", Number(weekStart.value))
      ]);
      renderMeals(mealsRoot, meals);
      renderSpending(weekRoot, week, async (cents) => {
        try { await api.saveBudget("week", cents); status.textContent = "Weekly budget saved locally."; await loadProjections(); }
        catch (error) { showError(error); }
      });
      renderSpending(monthRoot, month, async (cents) => {
        try { await api.saveBudget("month", cents); status.textContent = "Monthly budget saved locally."; await loadProjections(); }
        catch (error) { showError(error); }
      });
    } catch (error) { showError(error); }
  };
  const paint = () => {
    root.querySelector<HTMLElement>("#receipt-count")!.textContent = `${snapshot.receipt_count} confirmed receipt${snapshot.receipt_count === 1 ? "" : "s"}`;
    const query = inventorySearch.value.trim().toLocaleLowerCase();
    const visible = snapshot.lots.filter((lot) => Number(lot.quantity) > 0 && (!activeZone || lot.zone === activeZone) && lot.name.toLocaleLowerCase().includes(query));
    renderKitchen(kitchenRoot, snapshot.lots, (zone) => { activeZone = activeZone === zone ? null : zone; paint(); }, typeof WebGLRenderingContext !== "undefined");
    const filter = document.createElement("p");
    filter.textContent = activeZone ? `Showing ${activeZone}. ` : "Showing all zones. ";
    if (activeZone) {
      const clear = document.createElement("button");
      clear.type = "button"; clear.textContent = "Show all";
      clear.addEventListener("click", () => { activeZone = null; paint(); });
      filter.append(clear);
    }
    listRoot.replaceChildren(filter);
    const list = document.createElement("div"); listRoot.append(list);
    renderInventoryList(list, visible, async (id, zone) => {
      try { snapshot = await api.action({ kind: "move", lot_id: id, zone }); status.textContent = "Inventory location updated."; paint(); }
      catch (error) { showError(error); }
    }, async (id, kind, quantity) => {
      try { snapshot = await api.action({ kind, lot_id: id, quantity }); status.textContent = kind === "consume" ? "Food use recorded." : "Discarded food cost recorded separately."; paint(); await loadProjections(); }
      catch (error) { showError(error); }
    });
    spendingRoot.replaceChildren();
    for (const [label, value] of [
      ["Receipt cash flow", snapshot.cash_flow_cents], ["Food purchases and refunds", snapshot.food_spending_cents],
      ["Household goods", snapshot.household_goods_cents], ["Tax", snapshot.tax_cents],
      ["Food still in inventory", snapshot.remaining_food_cost_cents], ["Discarded food cost", snapshot.discarded_cost_cents]
    ] as Array<[string, number]>) {
      const row = document.createElement("div"); row.className = "spending-row";
      const name = document.createElement("span"); name.textContent = label;
      const amount = document.createElement("strong"); amount.textContent = dollars(value);
      row.append(name, amount); spendingRoot.append(row);
    }
    categoriesRoot.replaceChildren();
    for (const item of snapshot.receipt_items) {
      const row = document.createElement("div"); row.className = "category-row";
      const name = document.createElement("span"); name.textContent = `${item.name} (${item.receipt_id}, ${dollars(item.net_cents)})`;
      const input = document.createElement("input"); input.value = item.category;
      input.setAttribute("aria-label", `Category for ${item.name} from ${item.receipt_id}`);
      const save = document.createElement("button"); save.type = "button"; save.textContent = "Save category";
      save.addEventListener("click", async () => {
        try { snapshot = await api.editCategory(item.line_id, input.value); status.textContent = "Category saved in audit history."; paint(); await loadProjections(); }
        catch (error) { showError(error); }
      });
      row.append(name, input, save); categoriesRoot.append(row);
    }
    const audit = document.createElement("p"); audit.className = "hint";
    audit.textContent = `${snapshot.category_edits.length} category edit${snapshot.category_edits.length === 1 ? "" : "s"} recorded.`;
    categoriesRoot.append(audit);
    if (snapshot.category_edits.length) {
      const history = document.createElement("ol");
      history.setAttribute("aria-label", "Category edit history");
      for (const edit of snapshot.category_edits) {
        const row = document.createElement("li");
        row.textContent = `${edit.receipt_id} ${edit.line_id}: ${edit.from} → ${edit.to}`;
        history.append(row);
      }
      categoriesRoot.append(history);
    }
  };
  paint();
  inventorySearch.addEventListener("input", paint);
  renderBackup(backupRoot, api, async (restored) => { snapshot = restored; paint(); await loadProjections(); });
  await loadProjections();
  viewDate.addEventListener("change", loadProjections);
  weekStart.addEventListener("change", loadProjections);
  const openReview = (draft: ReceiptDraftDto, raw: string) => {
      reviewPanel.hidden = false;
      status.textContent = "Draft ready. Review and confirm to save it.";
      renderReview(reviewPanel, draft, raw, snapshot.lots, async (edited) => {
        try {
          const outcome = await api.confirm(edited);
          status.textContent = outcome === "duplicate" ? "This exact receipt was already imported; nothing changed." : "Receipt confirmed and inventory updated.";
          reviewPanel.hidden = true; reviewPanel.replaceChildren();
          snapshot = await api.getSnapshot(); paint(); await loadProjections();
        } catch (error) { showError(error); }
      }, () => { reviewPanel.hidden = true; reviewPanel.replaceChildren(); status.textContent = "Import cancelled. Nothing changed."; });
  };
  const startReview = async () => {
    const raw = textarea.value;
    if (!raw.trim()) { status.textContent = "Enter receipt text first."; return; }
    try {
      const draft = await api.importText(raw);
      openReview(draft, raw);
    } catch (error) { showError(error); }
  };
  root.querySelector<HTMLButtonElement>("button[data-action=import]")!.addEventListener("click", startReview);
  root.querySelector<HTMLInputElement>("input[name=receipt-file]")!.addEventListener("change", async (event) => {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (file) { textarea.value = await file.text(); await startReview(); }
  });
  let previewUrl: string | null = null;
  root.querySelector<HTMLInputElement>("input[name=receipt-photo]")!.addEventListener("change", async (event) => {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    const preview = root.querySelector<HTMLElement>("#photo-preview")!;
    preview.replaceChildren();
    if (previewUrl) URL.revokeObjectURL(previewUrl);
    if (typeof URL.createObjectURL === "function") {
      previewUrl = URL.createObjectURL(file);
      const image = document.createElement("img");
      image.src = previewUrl;
      image.alt = "Selected local receipt preview";
      image.className = "photo-preview";
      preview.append(image);
    } else {
      preview.textContent = `Selected photo: ${file.name}`;
    }
    if (file.size > 8 * 1024 * 1024 || !["image/png", "image/jpeg"].includes(file.type)) {
      status.textContent = "Choose a PNG or JPEG photo under 8 MB.";
      return;
    }
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const result = await api.scanPhoto(bytes);
      textarea.value = result.text;
      openReview(result.draft, result.text);
    } catch (error) {
      showError(error);
      status.textContent += " Enter the receipt text manually above.";
    }
  });
}
