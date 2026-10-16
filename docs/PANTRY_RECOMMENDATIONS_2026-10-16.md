# Pantry recipe recommendations

HungerStrike bundles ten original starter recipes in `assets/recipes.json`. They use local confirmed stock across the fridge, freezer, and pantry; no recipe search service, live price feed, or paid API is involved. Four additions broaden the combinations around pantry staples and common produce: a bean and rice bowl, spinach and rice bowl, tomato and egg skillet, and banana oat bowl.

For each recipe, the planner first allocates usable matching lots by known expiry date. A Plan next recipe is ranked by how many ingredients still need groceries, then by the share of its required quantities available from stock. The percentage is the average of each ingredient's stocked fraction, rounded down so an incomplete recipe never appears as fully stocked. The card lists the first two missing quantities; the details show every ingredient and let the user add the shortage to the local shopping list.

Names match through a short explicit alias list. Counted canned black or kidney beans may match a canned-beans recipe; `can` and `tin` are compatible with `each` only when the ingredient name also matches. Dry beans are not treated as canned beans. Recipe steps are original, short prompts rather than a cooking safety assessment. User-entered dates are planning cues; inspect food condition and cook safely. Missing grocery prices, nutrition, and dietary suitability are not inferred.

The [Windows workflow](../.github/workflows/pantry-recommendations.yml) checks formatting and tests the core and desktop workspace. No installer, food image, or personal inventory fixture is bundled for this review.
