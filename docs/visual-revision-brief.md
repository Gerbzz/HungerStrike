# HungerStrike visual revision - reference-based working brief

**Status:** Local implementation preview ready for review, 2026-10-11. I inspected all three user-supplied dishwasher frames (`image(20261010-202929).png`, `image(20261010-202930).png`, `image(20261010-202931).png`). This is a quality reference, not a request for dishwasher simulation. The preview package and its verification frames remain in the original local worktree. This brief records that visual direction; the [recovered source branch](verification/LIGHT_UI_SOURCE_RECOVERY.md) requires a new build and capture.

## What the references show

The central object has fine physical construction: rack lattices, rails, plate prongs, recognizable dishes, a mechanical frame and open door, and a translucent shell that exposes the interior. Soft neutral light and careful camera framing make those details readable. The interface is a bright off-white workspace with a restrained dark header, compact settings at left, a large viewport and results at right. These are filmed screen frames, so their washed colors are not reliable palette samples.

The [older public-version capture](SCREENSHOT_PROVENANCE.md) shows the starting gap: fridge, pantry and food are plain cuboids; doors jump as flat slabs; the room and hard wall shadow dominate; the dark egui controls feel detached. Receipt, Meals and Spending leave large blank regions or present dense raw forms.

## Proposed visual direction

- **Detailed focal scene:** Build one curated, proportionate kitchen around the fridge and nearby counter/pantry. The fridge needs door thickness, hinges, shaped handles, seams, rubber seal, liner, shelf rails, glass shelves, door bins and crisper drawers; the freezer needs a believable sliding drawer. Pantry shelves, carcass, trim and counter edge should share the same physical care. A selected-zone cutaway or ghosted shell may expose stock without sacrificing a credible closed state.
- **Material and light:** Distinguish brushed metal, enamel, glass, translucent plastic, rubber, wood and packaging through surface response and subtle local texture. Use soft key/fill and interior task light, controlled shadows and an intentional camera angle. Flat tinted boxes are not the finish line.
- **Stock and motion:** Replace generic cubes with recognizable, scale-aware proxies for fixture foods and products: bananas, milk, eggs, pasta, greens, soap, apples and yogurt. Each proxy maps to an actual confirmed lot. Hinged doors, drawer rails, camera focus and selection highlights should ease smoothly. Keep equivalent keyboard/list controls and never imply an unknown quantity or expiry through a graphic.
- **Unified interface:** Use a light three-part Kitchen layout: compact storage/filter rail, dominant scene, contextual lot details/actions. Use consistent typography, spacing, dividers and clear primary actions under a restrained dark header. Give receipt lines and reconciliation balanced room; use the full width for Meals and Spending summaries. Preserve explicit review-before-write, uncertainty, refunds, unknown costs and synthetic-data labeling.

## Code boundary and alignment

`desktop/src/scene.rs` is the main geometry, material, light, camera and interaction change point; it may split into appliance, prop and animation modules. `desktop/src/ui.rs` owns the theme and five screens; shared styles and screen modules may replace its monolithic layout. `desktop/src/main.rs` should gain only presentation/animation state. Local authored models or textures, if chosen, belong in `assets/` with provenance. `core/` accounting, OCR safeguards and persistence stay unchanged. No web UI, paid assets, cloud services, room reconstruction or Android worktree edits.

The current preview uses source-authored procedural Bevy geometry and materials. The reference's finer material finish remains an open quality gap. The app-generated open, selected-stock, Receipt, Meals and Spending frames were reviewed at 1600×900, with Kitchen and the empty state also reviewed at 1280×720. Native interaction remains a separate gate before calling the revision polished.
