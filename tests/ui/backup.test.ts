// @vitest-environment jsdom
import { expect, it, vi } from "vitest";
import { renderBackup } from "../../src/backup";

it("previews a selected backup before any replace action", async () => {
  const root = document.createElement("div");
  const preview = vi.fn(async () => ({
    incoming_receipts: 2, existing_receipts: 1, duplicate_receipts: 1,
    conflicting_identifiers: 0, incoming_lots: 5, replace_will_create_restore_point: true
  }));
  const apply = vi.fn(async () => ({ outcome: "imported" as const, snapshot: {} as never }));
  renderBackup(root, {
    exportBackupFile: vi.fn(), previewBackup: preview, applyBackup: apply, undoLastReplace: vi.fn()
  }, () => {});
  const input = root.querySelector<HTMLInputElement>('input[name="backup-file"]')!;
  const file = new File(["backup"], "backup.json", { type: "application/json" });
  file.arrayBuffer = async () => new Uint8Array([1, 2, 3]).buffer;
  Object.defineProperty(input, "files", { value: [file] });
  input.dispatchEvent(new Event("change"));
  await vi.waitFor(() => expect(preview).toHaveBeenCalled());
  expect(root.textContent).toContain("2 incoming receipts");
  expect(apply).not.toHaveBeenCalled();
  root.querySelector<HTMLButtonElement>('button[data-action="replace-backup"]')!.click();
  await vi.waitFor(() => expect(apply).toHaveBeenCalledWith([1, 2, 3], "replace"));
});
