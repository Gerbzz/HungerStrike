import type { BackupApi, BackupMode, SnapshotDto } from "./api";

export function renderBackup(root: HTMLElement, api: BackupApi, onChanged: (snapshot: SnapshotDto) => void): void {
  root.innerHTML = `<p>Export a versioned JSON backup to a file you choose. Import requires a preview and an explicit action.</p>
    <div class="actions"><button type="button" data-action="export-backup">Export backup</button>
      <label class="file-label">Choose backup to import<input type="file" accept=".json,application/json" name="backup-file"></label>
      <button type="button" data-action="undo-replace">Undo last replace</button></div>
    <p class="hint">Backups omit receipt photos, OCR transcripts, and raw receipt lines. Replacing stores a local restore point in the app database.</p>
    <p id="backup-status" role="status" aria-live="polite"></p><div id="backup-preview" hidden></div>`;
  const status = root.querySelector<HTMLElement>("#backup-status")!;
  const previewBox = root.querySelector<HTMLElement>("#backup-preview")!;
  let pending: number[] | null = null;
  const fail = (error: unknown) => { status.textContent = String(error); status.className = "error"; };
  root.querySelector<HTMLButtonElement>('button[data-action="export-backup"]')!.addEventListener("click", async () => {
    try {
      const path = await api.exportBackupFile();
      status.textContent = path ? `Backup saved to ${path}` : "Backup export cancelled.";
    } catch (error) { fail(error); }
  });
  root.querySelector<HTMLInputElement>('input[name="backup-file"]')!.addEventListener("change", async (event) => {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    pending = null; previewBox.hidden = true; previewBox.replaceChildren();
    if (!file) return;
    if (file.size > 20 * 1024 * 1024) { fail("Backup file is too large."); return; }
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const preview = await api.previewBackup(bytes);
      pending = bytes;
      previewBox.hidden = false;
      const summary = document.createElement("p");
      summary.textContent = `${preview.incoming_receipts} incoming receipts, ${preview.existing_receipts} already stored, ${preview.duplicate_receipts} exact duplicates, ${preview.conflicting_identifiers} identifier conflicts, ${preview.incoming_lots} incoming lots.`;
      previewBox.append(summary);
      const detail = document.createElement("p");
      detail.textContent = "Reconcile adds a fully disjoint backup. Replace overwrites current app data and creates an undo point.";
      previewBox.append(detail);
      for (const mode of ["reconcile", "replace"] as const) {
        const button = document.createElement("button"); button.type = "button";
        button.dataset.action = `${mode}-backup`;
        button.textContent = mode === "replace" ? "Replace with this backup" : "Reconcile this backup";
        button.addEventListener("click", async () => {
          if (!pending) return;
          try {
            const result = await api.applyBackup(pending, mode as BackupMode);
            onChanged(result.snapshot);
            status.textContent = result.outcome === "duplicate" ? "Backup receipts already exist; nothing changed." : "Backup applied locally.";
            previewBox.hidden = true; pending = null;
          } catch (error) { fail(error); }
        });
        previewBox.append(button);
      }
      status.textContent = "Backup validated. Choose reconcile or replace to apply it.";
    } catch (error) { fail(error); }
  });
  root.querySelector<HTMLButtonElement>('button[data-action="undo-replace"]')!.addEventListener("click", async () => {
    try { onChanged(await api.undoLastReplace()); status.textContent = "Previous local state restored."; }
    catch (error) { fail(error); }
  });
}
