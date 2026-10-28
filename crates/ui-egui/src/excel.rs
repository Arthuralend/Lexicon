//! Lexicon: the Excel tab, a small spreadsheet over the document's tables.
//!
//! While the tab is open, a cell whose text starts with `=` is a formula
//! ([`wordcraft_engine::sheet`]): once the caret leaves the cell, the cell shows the result, and
//! every result is recalculated after each edit. The formula bar under the ribbon shows the
//! caret cell's name (`B3`) and its formula, and edits it. Buttons add sums, averages and the like
//! for the numbers above or to the left (AutoSum), a totals row under a table, and rows and
//! columns. Leaving the tab turns formulas into plain text: the cells keep their last values.
//!
//! Formulas belong to top-level tables of the body, keyed by the table's block index, row and
//! cell; they are kept for the open document only.

use std::collections::HashMap;

use egui::{Ui, vec2};
use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, Pos, StoryRef};
use wordcraft_engine::sheet;

use crate::WordApp;

/// A cell: the table's block index in the body, the row, and the cell in the row.
type Key = (usize, usize, usize);

/// Most rows and cells per row read from one table.
const MAX_ROWS: usize = 5_000;
const MAX_CELLS: usize = 200;

struct Formula {
    /// The formula, with its `=`.
    text: String,
    /// What the cell showed after the last calculation (empty before the first).
    shown: String,
}

#[derive(Default)]
pub struct ExcelState {
    formulas: HashMap<Key, Formula>,
    /// The formula bar's text, and the cell it was filled from.
    bar: String,
    bar_for: Option<Key>,
    /// The document revision and document the formulas were last calculated for.
    seen: Option<(u64, u64)>,
}

impl ExcelState {
    /// Whether any formula is live.
    pub fn has_formulas(&self) -> bool {
        !self.formulas.is_empty()
    }
}

/// The cell holding the caret, when it is in a top-level table of the body.
fn caret_cell(app: &WordApp) -> Option<Key> {
    let f = &app.session.sel.focus;
    if f.story != StoryRef::Body {
        return None;
    }
    let (table, row, cell) = f.path.cell()?;
    match table.0.as_slice() {
        [b] => Some((*b as usize, row, cell)),
        _ => None,
    }
}

fn table_path(b: usize) -> Option<Path> {
    Some(Path(vec![u32::try_from(b).ok()?]))
}

/// A cell's text, its paragraphs joined with line breaks.
fn cell_text(doc: &Document, (b, r, c): Key) -> Option<String> {
    let t = doc.table(StoryRef::Body, &table_path(b)?)?;
    let cell = t.rows.get(r)?.cells.get(c)?;
    let paras: Vec<String> = cell.blocks.iter().filter_map(|bl| bl.as_para()).map(|p| p.plain_text()).collect();
    Some(paras.join("\n"))
}

/// Every cell's text of table `b` (bounded).
fn grid(doc: &Document, b: usize) -> Option<Vec<Vec<String>>> {
    let t = doc.table(StoryRef::Body, &table_path(b)?)?;
    Some(
        t.rows
            .iter()
            .take(MAX_ROWS)
            .enumerate()
            .map(|(r, row)| (0..row.cells.len().min(MAX_CELLS)).map(|c| cell_text(doc, (b, r, c)).unwrap_or_default()).collect())
            .collect(),
    )
}

/// The body's top-level tables, by block index.
fn tables(doc: &Document) -> Vec<usize> {
    doc.body.iter().enumerate().filter(|(_, bl)| bl.as_table().is_some()).map(|(i, _)| i).collect()
}

/// Replace a cell's text, keeping the formatting of its first character. `false` when the cell
/// isn't there or holds something other than paragraphs.
fn set_cell(doc: &mut Document, (b, r, c): Key, text: &str) -> bool {
    let Some(t) = table_path(b).and_then(|p| doc.table(StoryRef::Body, &p).cloned()) else {
        return false;
    };
    let Some(cell) = t.rows.get(r).and_then(|row| row.cells.get(c)) else {
        return false;
    };
    let paras: Vec<_> = cell.blocks.iter().filter_map(|bl| bl.as_para()).collect();
    let (Some(first), Some(last)) = (paras.first(), paras.last()) else {
        return false;
    };
    if paras.len() != cell.blocks.len() {
        return false;
    }
    let props = if first.text.is_empty() { first.mark.clone() } else { first.props_at(0).clone() };
    let (Ok(b), Ok(r), Ok(c), Ok(n)) = (u32::try_from(b), u32::try_from(r), u32::try_from(c), u32::try_from(paras.len() - 1)) else {
        return false;
    };
    let start = Pos { story: StoryRef::Body, path: Path(vec![b, r, c, 0]), off: 0 };
    let end = Pos { story: StoryRef::Body, path: Path(vec![b, r, c, n]), off: last.text.len() };
    doc.delete_range(&start, &end).is_ok() && doc.insert_text(&start, text, &props).is_ok()
}

/// Before each frame: leaving the tab turns formulas into text; on it, new formulas typed into
/// cells are picked up and every result is brought up to date.
pub(crate) fn update(app: &mut WordApp) {
    if app.ui.tab != "Excel" || !app.has_document() {
        app.excel.formulas.clear();
        app.excel.seen = None;
        app.excel.bar_for = None;
        return;
    }
    let now = (app.session.rev(), app.session.document_id());
    if app.excel.seen.is_some_and(|(_, d)| d != now.1) {
        app.excel.formulas.clear();
    }
    if app.excel.seen == Some(now) {
        return;
    }
    let caret = caret_cell(app);
    // A formula cell the user typed over is plain text again.
    app.excel.formulas.retain(|k, f| match cell_text(&app.session.doc, *k) {
        Some(text) => f.shown.is_empty() || text == f.shown || text.trim_start().starts_with('='),
        None => false,
    });
    // Formulas typed into cells, once the caret has left them.
    for b in tables(&app.session.doc) {
        let Some(g) = grid(&app.session.doc, b) else { continue };
        for (r, row) in g.iter().enumerate() {
            for (c, text) in row.iter().enumerate() {
                let t = text.trim();
                if t.len() > 1 && t.starts_with('=') && caret != Some((b, r, c)) {
                    app.excel.formulas.insert((b, r, c), Formula { text: t.to_string(), shown: String::new() });
                }
            }
        }
    }
    recalc(app);
    app.excel.seen = Some((app.session.rev(), app.session.document_id()));
}

/// Calculate every formula and write the results into their cells.
fn recalc(app: &mut WordApp) {
    let comma = app.session.region().decimal() == ',';
    let mut by_table: HashMap<usize, HashMap<(usize, usize), String>> = HashMap::new();
    for ((b, r, c), f) in &app.excel.formulas {
        by_table.entry(*b).or_default().insert((*r, *c), f.text.clone());
    }
    let mut changed = false;
    for (b, formulas) in by_table {
        let Some(g) = grid(&app.session.doc, b) else { continue };
        let results = sheet::Sheet { cells: &g, formulas: &formulas }.evaluate_all();
        for ((r, c), v) in results {
            let shown = match v {
                Ok(v) => sheet::format_number(v, comma),
                Err(e) => e.text().to_string(),
            };
            let key = (b, r, c);
            if cell_text(&app.session.doc, key).as_deref() != Some(shown.as_str()) && set_cell(&mut app.session.doc, key, &shown) {
                changed = true;
            }
            if let Some(f) = app.excel.formulas.get_mut(&key) {
                f.shown = shown;
            }
        }
    }
    if changed {
        let focus = app.session.doc.clamp(&app.session.sel.focus);
        let anchor = app.session.doc.clamp(&app.session.sel.anchor);
        app.session.sel = wordcraft_engine::Selection { anchor, focus };
        app.session.touch();
    }
}

/// Put `formula` (or plain text) into cell `key` and calculate.
fn enter(app: &mut WordApp, key: Key, text: &str) -> Result<Value, String> {
    let text = text.trim();
    app.session.checkpoint("Formula");
    if text.starts_with('=') && text.len() > 1 {
        app.excel.formulas.insert(key, Formula { text: text.to_string(), shown: String::new() });
    } else {
        app.excel.formulas.remove(&key);
        if !set_cell(&mut app.session.doc, key, text) {
            return Err(tl!("The caret isn't in a table cell.").into());
        }
        app.session.touch();
    }
    recalc(app);
    app.session.touch();
    app.excel.seen = None;
    app.excel.bar_for = None;
    Ok(json!({"cell": sheet::cell_name(key.1, key.2), "text": text}))
}

/// Whether cell `(r, c)` of `g` holds a number or a formula.
fn numeric(app: &WordApp, b: usize, g: &[Vec<String>], r: usize, c: usize) -> bool {
    app.excel.formulas.contains_key(&(b, r, c)) || g.get(r).and_then(|row| row.get(c)).is_some_and(|t| sheet::parse_number(t).is_some())
}

/// AutoSum's range for the cell at `(r, c)`: the numbers right above it, else right to its left.
fn auto_range(app: &WordApp, b: usize, g: &[Vec<String>], r: usize, c: usize) -> String {
    let up = (0..r).rev().take_while(|&i| numeric(app, b, g, i, c)).last();
    if let Some(top) = up {
        return format!("{}:{}", sheet::cell_name(top, c), sheet::cell_name(r - 1, c));
    }
    let left = (0..c).rev().take_while(|&i| numeric(app, b, g, r, i)).last();
    if let Some(first) = left {
        return format!("{}:{}", sheet::cell_name(r, first), sheet::cell_name(r, c - 1));
    }
    if r > 0 { "ABOVE".into() } else { "LEFT".into() }
}

/// The Excel tab's commands (`ui.excel.*`).
pub(crate) fn command(app: &mut WordApp, id: &str, p: &Value) -> Result<Value, String> {
    let need_cell = || tl!("Put the caret in a table cell first.").to_string();
    match id {
        "ui.excel.function" => {
            let f = p.get("name").and_then(Value::as_str).unwrap_or("SUM").to_ascii_uppercase();
            if !matches!(f.as_str(), "SUM" | "AVERAGE" | "COUNT" | "MAX" | "MIN") {
                return Err(format!("unknown function {f:?}"));
            }
            let key = caret_cell(app).ok_or_else(need_cell)?;
            let g = grid(&app.session.doc, key.0).unwrap_or_default();
            let range = auto_range(app, key.0, &g, key.1, key.2);
            enter(app, key, &format!("={f}({range})"))
        }
        "ui.excel.enter" => {
            let key = caret_cell(app).ok_or_else(need_cell)?;
            let text = p.get("text").and_then(Value::as_str).unwrap_or_default().to_string();
            enter(app, key, &text)
        }
        "ui.excel.totals" => totals(app),
        "ui.excel.insertSheet" => {
            let rows = p.get("rows").and_then(Value::as_u64).unwrap_or(6).clamp(1, 100);
            let cols = p.get("cols").and_then(Value::as_u64).unwrap_or(4).clamp(1, 26);
            app.run("insert.table", json!({"rows": rows, "cols": cols}))
        }
        "ui.excel.recalc" => {
            app.excel.seen = None;
            update(app);
            Ok(json!({"formulas": app.excel.formulas.len()}))
        }
        "ui.excel.toText" => {
            let n = app.excel.formulas.len();
            app.excel.formulas.clear();
            app.excel.bar_for = None;
            Ok(json!({"converted": n}))
        }
        _ => Err(format!("unknown command {id}")),
    }
}

/// A totals row under the caret's table: the sum of each column that has numbers.
fn totals(app: &mut WordApp) -> Result<Value, String> {
    let key = caret_cell(app).ok_or_else(|| tl!("Put the caret in a table cell first.").to_string())?;
    let b = key.0;
    let g = grid(&app.session.doc, b).unwrap_or_default();
    let rows = g.len();
    if rows == 0 {
        return Err(tl!("The caret isn't in a table cell.").into());
    }
    // Add the row under the last one.
    let last = u32::try_from(rows - 1).map_err(|e| e.to_string())?;
    let b32 = u32::try_from(b).map_err(|e| e.to_string())?;
    app.session.sel = wordcraft_engine::Selection::caret(Pos { story: StoryRef::Body, path: Path(vec![b32, last, 0, 0]), off: 0 });
    app.run("table.insertRowBelow", json!({}))?;
    let cols = g.iter().map(Vec::len).max().unwrap_or(0);
    let mut sums = 0;
    let mut labelled = false;
    for c in 0..cols {
        let has_numbers = (0..rows).any(|r| numeric(app, b, &g, r, c));
        let k = (b, rows, c);
        if has_numbers {
            app.excel
                .formulas
                .insert(k, Formula { text: format!("=SUM({}:{})", sheet::cell_name(0, c), sheet::cell_name(rows - 1, c)), shown: String::new() });
            sums += 1;
        } else if !labelled {
            set_cell(&mut app.session.doc, k, tl!("Total"));
            labelled = true;
        }
    }
    app.session.touch();
    recalc(app);
    app.excel.seen = None;
    Ok(json!({"sums": sums}))
}

/// The Excel tab's ribbon.
pub(crate) fn tab(app: &mut WordApp, ui: &mut Ui) {
    use crate::widgets::{big, group};
    group(ui, "Table", None, app, |ui, app| {
        big(ui, app, "table", "New Sheet", "ui.excel.insertSheet", json!({}), false);
        big(ui, app, "insertAbove", "Totals Row", "ui.excel.totals", json!({}), false);
    });
    group(ui, "AutoSum", None, app, |ui, app| {
        big(ui, app, "formula", "Sum", "ui.excel.function", json!({"name": "SUM"}), false);
        for (label, name) in [("Average", "AVERAGE"), ("Count", "COUNT"), ("Max", "MAX"), ("Min", "MIN")] {
            let r = ui.add_enabled(caret_cell(app).is_some(), egui::Button::new(tl!(label)).min_size(vec2(64.0, 22.0)));
            if r.clicked() {
                let r = command(app, "ui.excel.function", &json!({"name": name}));
                report(app, r);
            }
        }
    });
    group(ui, "Rows & Columns", None, app, |ui, app| {
        big(ui, app, "insertAbove", "Insert Below", "table.insertRowBelow", json!({}), false);
        big(ui, app, "plus", "Insert Right", "table.insertColumnRight", json!({}), false);
        big(ui, app, "deleteTable", "Delete Rows", "table.deleteRow", json!({}), false);
    });
    group(ui, "Calculation", None, app, |ui, app| {
        big(ui, app, "updateTable", "Calculate Now", "ui.excel.recalc", json!({}), false);
        big(ui, app, "toText", "Convert to Text", "ui.excel.toText", json!({}), false);
    });
}

fn report(app: &mut WordApp, r: Result<Value, String>) {
    if let Err(e) = r {
        app.status(e);
    }
}

/// The formula bar: the caret cell's name, `fx` and the formula.
pub(crate) fn formula_bar(app: &mut WordApp, ui: &mut Ui) {
    let key = caret_cell(app);
    if app.excel.bar_for != key {
        app.excel.bar_for = key;
        app.excel.bar = match key {
            Some(k) => app.excel.formulas.get(&k).map(|f| f.text.clone()).or_else(|| cell_text(&app.session.doc, k)).unwrap_or_default(),
            None => String::new(),
        };
    }
    ui.horizontal_centered(|ui| {
        let name = key.map(|(_, r, c)| sheet::cell_name(r, c)).unwrap_or_else(|| "—".into());
        ui.add_sized(vec2(64.0, 22.0), egui::Label::new(egui::RichText::new(name).strong()));
        ui.separator();
        ui.label(egui::RichText::new("fx").italics().strong());
        let hint = if key.is_some() { tl!("Type a formula, e.g. =SUM(B2:B5)") } else { tl!("Put the caret in a table cell first.") };
        let w = (ui.available_width() - 8.0).max(80.0);
        let r = ui.add_enabled(key.is_some(), egui::TextEdit::singleline(&mut app.excel.bar).hint_text(hint).desired_width(w));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let text = app.excel.bar.clone();
            let r = command(app, "ui.excel.enter", &json!({"text": text}));
            report(app, r);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_engine::Session;

    fn app() -> WordApp {
        WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default())
    }

    fn fill(a: &mut WordApp, rows: &[&[&str]]) {
        a.run("insert.table", json!({"rows": rows.len(), "cols": rows.first().map_or(1, |r| r.len())})).unwrap();
        let b = tables(&a.session.doc)[0];
        for (r, row) in rows.iter().enumerate() {
            for (c, text) in row.iter().enumerate() {
                assert!(set_cell(&mut a.session.doc, (b, r, c), text));
            }
        }
        a.session.touch();
    }

    #[test]
    fn formulas_calculate_on_the_tab_and_become_text_off_it() {
        let mut a = app();
        a.ui.tab = "Excel".into();
        fill(&mut a, &[&["Item", "Sum"], &["a", "2"], &["b", "3,5"], &["Total", "=SUM(B2:B3)"]]);
        update(&mut a);
        let b = tables(&a.session.doc)[0];
        let t = cell_text(&a.session.doc, (b, 3, 1)).unwrap();
        assert!(t == "5.5" || t == "5,5", "{t}");
        assert!(a.excel.has_formulas());
        // An edit above recalculates.
        assert!(set_cell(&mut a.session.doc, (b, 1, 1), "10"));
        a.session.touch();
        update(&mut a);
        let t = cell_text(&a.session.doc, (b, 3, 1)).unwrap();
        assert!(t == "13.5" || t == "13,5", "{t}");
        // Leaving the tab keeps the value as text.
        a.ui.tab = "Home".into();
        update(&mut a);
        assert!(!a.excel.has_formulas());
        assert!(set_cell(&mut a.session.doc, (b, 1, 1), "1"));
        a.session.touch();
        a.ui.tab = "Excel".into();
        update(&mut a);
        let t2 = cell_text(&a.session.doc, (b, 3, 1)).unwrap();
        assert_eq!(t2, t, "no longer a formula");
    }

    #[test]
    fn autosum_and_totals_row() {
        let mut a = app();
        a.ui.tab = "Excel".into();
        fill(&mut a, &[&["Item", "Qty"], &["a", "2"], &["b", "3"], &["", ""]]);
        let b = tables(&a.session.doc)[0];
        a.session.sel = wordcraft_engine::Selection::caret(Pos { story: StoryRef::Body, path: Path(vec![b as u32, 3, 1, 0]), off: 0 });
        let r = command(&mut a, "ui.excel.function", &json!({"name": "SUM"})).unwrap();
        assert_eq!(r["text"], "=SUM(B2:B3)");
        assert_eq!(cell_text(&a.session.doc, (b, 3, 1)).as_deref(), Some("5"));
        command(&mut a, "ui.excel.totals", &json!({})).unwrap();
        update(&mut a);
        assert_eq!(cell_text(&a.session.doc, (b, 4, 0)).as_deref(), Some(tl!("Total")));
        assert_eq!(cell_text(&a.session.doc, (b, 4, 1)).as_deref(), Some("10"), "the sum includes the AutoSum cell");
        // Outside a table the commands explain themselves.
        a.session.sel = wordcraft_engine::Selection::caret(a.session.doc.start_of(StoryRef::Body));
        if caret_cell(&a).is_none() {
            assert!(command(&mut a, "ui.excel.function", &json!({})).is_err());
        }
        assert!(command(&mut a, "ui.excel.nope", &json!({})).is_err());
    }
}
