//! Small local display settings stored alongside budgets in the app state.

use crate::store::Store;

const THEMES: [&str; 3] = ["light", "dark", "system"];

/// The saved theme setting, or `None` when it was never changed (light).
pub fn ui_theme(store: &Store) -> Result<Option<String>, String> {
    Ok(store.load()?.ui_theme)
}

/// Save the theme setting. Accepts "light", "dark" or "system".
pub fn set_ui_theme(store: &mut Store, theme: &str) -> Result<(), String> {
    if !THEMES.contains(&theme) {
        return Err(format!("unknown theme: {theme}"));
    }
    let mut state = store.load()?;
    state.ui_theme = Some(theme.to_string());
    store.save(&state)
}
