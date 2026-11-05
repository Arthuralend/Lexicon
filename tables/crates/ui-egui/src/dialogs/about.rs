use super::Dialog;

/// Help ▸ About GridCraft: the app, its contributors and the AI models that helped. The credits
/// are compiled in (`crate::credits`, docs/contributors.md).
pub(super) fn about(ui: &mut egui::Ui, d: &mut Dialog) {
    // Lusoft's Таблицы: the program and its developer; the licence texts ship with the app.
    let _ = &d.tab;
    ui.heading("Таблицы");
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
    ui.add_space(4.0);
    ui.label("Spreadsheets for Документы. Developer: Lusoft.");
    ui.label(egui::RichText::new("Free software under the MIT or Apache-2.0 licence.").small().weak());
}
