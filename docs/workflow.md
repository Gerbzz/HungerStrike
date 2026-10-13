# HungerStrike workflow

HungerStrike is a Windows source preview. This walkthrough uses the repository's **synthetic test receipts**; their prices and tax are invented fixture values, not current store prices or tax guidance. General receipts require human review.

## 1. Review a purchase before it changes stock

Open [R001-purchase.txt](../core/tests/fixtures/receipts/R001-purchase.txt), paste its contents into **Receipt**, and select **Parse text**. Review the date, each item, quantity, unit, category, price, discount, subtotal, tax, and total. Inventory changes only after **Confirm reviewed purchase**. **Cancel and clear** discards the temporary draft. A purchase date never becomes an expiry date.

For a photo, **Open photo · offline OCR** accepts PNG/JPEG under 8 MB and 40 megapixels through a separately installed local English Tesseract at `C:\Program Files\Tesseract-OCR\tesseract.exe`. The engine and language model are not bundled. A stalled engine is stopped after 15 seconds; **Cancel and clear** stops an active job. OCR text remains a temporary draft and clears on confirmation or cancellation. Manual text entry works without Tesseract.

## 2. Handle refunds and corrections deliberately

Open [R002-refund.txt](../core/tests/fixtures/receipts/R002-refund.txt) and parse it. The milk refund can be linked to original lot `R001-L02` with one gallon of stock reversal, or left unlinked. Refunds affect spending either way; unlinked amounts appear separately in **Spending**.

Exact repeat imports do not charge again. If matching source text represents a truly separate purchase, give it a unique reviewed receipt ID and line IDs, then choose **Confirm separate purchase**. That decision is audited. Use **Correct existing receipt** for corrections. Name, date, and expiry corrections can follow stock use; cost or quantity corrections after stock use, and corrections after linked refunds, remain blocked to protect historical allocations.

## 3. Check the kitchen and plan meals

Select the fridge, freezer, or pantry in the 3D kitchen to filter stock. The list offers the same storage zones and inventory actions for keyboard use. Arrow keys and scene buttons orbit and zoom the scene. **Meals** uses confirmed stock quantities and shows ingredients and estimated remaining-lot cost. Unknown costs or expiry dates remain marked unknown.

## 4. Understand spending

**Spending** separates cash paid, item allocations, refunds, consumed stock cost, and discarded stock cost so stock use is not charged twice. Inspect weekly and monthly budgets, discounts, refunds, reconciliation, and discarded food cost. Unknown item costs require explicit reconciliation.

## 5. Back up local data

On **Data**, export a local JSON backup, preview an import, then choose reconciliation or replacement explicitly. A local restore point is available. Confirmed inventory is stored in SQLite at `%LOCALAPPDATA%\HungerStrike\inventory.sqlite`; `HUNGERSTRIKE_DATA_DIR` selects a separate parent directory for an isolated test run.

Backups and SQLite omit raw receipt lines, photo bytes, and OCR transcripts. Reviewed fields and sanitized audit metadata remain. See [launch notes](launch.md) and the [verification record](../verification.md) for tested behavior and open native interaction gates.
