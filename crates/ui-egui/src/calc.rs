//! Lexicon: Calculations, a spreadsheet inside the word processor.
//!
//! The Calculations tab replaces the page with a grid: column letters across the top, row
//! numbers down the side, a name box and formula bar above. A cell holds text, a number or a
//! formula (`=SUM(B2:B5)`, [`wordcraft_engine::sheet`]), recalculated after every change. The
//! sheet is a file of its own: it opens `.xlsx`, `.xls`, `.ods` and `.csv` files and saves Excel
//! workbooks (`.xlsx`) and CSV, apart from the document. A selection copies to the clipboard as a
//! table (tab-separated text and HTML) and goes into the document as a Word table; a table at the
//! document's caret comes into the sheet.

use std::collections::{BTreeMap, HashMap};

use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};
use wordcraft_engine::sheet;

use crate::WordApp;
use crate::file_dialogs::{AfterPick, FileDialogRequest};
use crate::theme::{Tokens, regular};

/// A cell: row and column from 0.
pub type Cell = (usize, usize);

const ROW_H: f32 = 22.0;
const COL_W: f32 = 96.0;
const HEAD_W: f32 = 48.0;
const HEAD_H: f32 = 22.0;
/// Rows and columns shown beyond the last one used.
const SPARE_ROWS: usize = 60;
const SPARE_COLS: usize = 12;
/// Limits on what a sheet holds (hostile files, runaway pastes).
const MAX_ROWS: usize = 100_000;
const MAX_COLS: usize = 702;
const MAX_CELLS: usize = 500_000;
const MAX_UNDO: usize = 100;

/// The open sheet.
#[derive(Default)]
pub struct Workbook {
    /// What was typed into each cell (formulas with their `=`).
    cells: BTreeMap<Cell, String>,
    /// What each formula cell shows.
    shown: HashMap<Cell, String>,
    /// Each formula's number, for saving.
    numbers: HashMap<Cell, f64>,
    pub path: Option<String>,
    pub dirty: bool,
    anchor: Cell,
    sel: Cell,
    /// The in-cell editor's text, while a cell is being edited.
    editing: Option<String>,
    edit_started: bool,
    /// The formula bar's text and the cell it shows.
    bar: String,
    bar_for: Option<Cell>,
    undo: Vec<BTreeMap<Cell, String>>,
    redo: Vec<BTreeMap<Cell, String>>,
    widths: BTreeMap<usize, f32>,
    /// Bring the active cell into view on the next frame.
    reveal: bool,
    /// A pointer drag extending the selection.
    dragging: bool,
}

impl Workbook {
    /// The text typed into a cell.
    pub fn raw(&self, c: Cell) -> &str {
        self.cells.get(&c).map(String::as_str).unwrap_or("")
    }

    /// What a cell shows: a formula's result, else its text.
    pub fn display(&self, c: Cell) -> String {
        match self.shown.get(&c) {
            Some(v) => v.clone(),
            None => self.raw(c).to_string(),
        }
    }

    /// The last used row and column (0 when empty).
    fn used(&self) -> (usize, usize) {
        let rows = self.cells.keys().map(|(r, _)| r + 1).max().unwrap_or(0);
        let cols = self.cells.keys().map(|(_, c)| c + 1).max().unwrap_or(0);
        (rows, cols)
    }

    fn snapshot(&mut self) {
        self.undo.push(self.cells.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Set a cell (empty text clears it). The caller takes the undo snapshot.
    fn put(&mut self, c: Cell, text: &str) {
        if c.0 >= MAX_ROWS || c.1 >= MAX_COLS {
            return;
        }
        if text.is_empty() {
            self.cells.remove(&c);
        } else if self.cells.len() < MAX_CELLS || self.cells.contains_key(&c) {
            self.cells.insert(c, text.to_string());
        }
        self.dirty = true;
    }

    /// Recalculate every formula.
    pub fn recalc(&mut self, comma: bool) {
        self.shown.clear();
        self.numbers.clear();
        let formulas: HashMap<Cell, String> =
            self.cells.iter().filter(|(_, v)| v.starts_with('=') && v.len() > 1).map(|(k, v)| (*k, v.clone())).collect();
        if formulas.is_empty() {
            return;
        }
        let (rows, cols) = self.used();
        let mut grid = vec![vec![String::new(); cols]; rows];
        for ((r, c), v) in &self.cells {
            if !v.starts_with('=')
                && let Some(slot) = grid.get_mut(*r).and_then(|row| row.get_mut(*c))
            {
                slot.clone_from(v);
            }
        }
        let results = sheet::Sheet { cells: &grid, formulas: &formulas }.evaluate_all();
        for (k, v) in results {
            match v {
                Ok(n) => {
                    self.shown.insert(k, sheet::format_number(n, comma));
                    self.numbers.insert(k, n);
                }
                Err(e) => {
                    self.shown.insert(k, e.text().to_string());
                }
            }
        }
    }

    fn range(&self) -> (Cell, Cell) {
        ((self.anchor.0.min(self.sel.0), self.anchor.1.min(self.sel.1)), (self.anchor.0.max(self.sel.0), self.anchor.1.max(self.sel.1)))
    }

    fn selected(&self, c: Cell) -> bool {
        let ((r0, c0), (r1, c1)) = self.range();
        (r0..=r1).contains(&c.0) && (c0..=c1).contains(&c.1)
    }

    fn width(&self, col: usize) -> f32 {
        self.widths.get(&col).copied().unwrap_or(COL_W)
    }

    /// The selection's shown values, one row per line, cells separated by tabs.
    fn selection_rows(&self) -> Vec<Vec<String>> {
        let ((r0, c0), (r1, c1)) = self.range();
        let (rows, cols) = self.used();
        let r1 = r1.min(rows.max(r0 + 1) - 1);
        let c1 = c1.min(cols.max(c0 + 1) - 1);
        (r0..=r1).map(|r| (c0..=c1).map(|c| self.display((r, c))).collect()).collect()
    }

    /// Load rows of cell texts.
    pub fn load(&mut self, rows: Vec<Vec<String>>, comma: bool) {
        self.cells.clear();
        for (r, row) in rows.into_iter().enumerate().take(MAX_ROWS) {
            for (c, text) in row.into_iter().enumerate().take(MAX_COLS) {
                if !text.is_empty() && self.cells.len() < MAX_CELLS {
                    self.cells.insert((r, c), text);
                }
            }
        }
        self.undo.clear();
        self.redo.clear();
        self.anchor = (0, 0);
        self.sel = (0, 0);
        self.editing = None;
        self.bar_for = None;
        self.dirty = false;
        self.recalc(comma);
    }

    /// The sheet as an Excel workbook.
    pub fn to_xlsx(&self) -> Result<Vec<u8>, String> {
        let (rows, cols) = self.used();
        let mut out = vec![vec![wordcraft_formats::xlsx_write::Cell::default(); cols]; rows];
        for ((r, c), raw) in &self.cells {
            let Some(slot) = out.get_mut(*r).and_then(|row| row.get_mut(*c)) else { continue };
            if raw.starts_with('=') {
                slot.formula = sheet::to_excel(raw);
                slot.number = self.numbers.get(&(*r, *c)).copied();
                slot.text = self.display((*r, *c));
            } else {
                slot.number = sheet::parse_number(raw).filter(|_| !raw.trim().ends_with('%'));
                slot.text.clone_from(raw);
            }
        }
        wordcraft_formats::xlsx_write::write(crate::tl!("Sheet1"), &out)
    }

    /// The sheet as CSV (shown values; `;` between cells with a decimal comma, as Excel does).
    pub fn to_csv(&self, comma: bool) -> String {
        let sep = if comma { ';' } else { ',' };
        let (rows, cols) = self.used();
        let mut o = String::new();
        for r in 0..rows {
            let line: Vec<String> = (0..cols)
                .map(|c| {
                    let v = self.display((r, c));
                    if v.contains(sep) || v.contains('"') || v.contains('\n') { format!("\"{}\"", v.replace('"', "\"\"")) } else { v }
                })
                .collect();
            o.push_str(&line.join(&sep.to_string()));
            o.push_str("\r\n");
        }
        o
    }
}

fn comma(app: &WordApp) -> bool {
    app.session.region().decimal() == ','
}

/// Whether the Calculations view is showing.
pub fn active(app: &WordApp) -> bool {
    app.ui.tab == "Calculations" && !app.ui.backstage
}

/// The column letters of a column from 0.
fn col_letters(col: usize) -> String {
    let name = sheet::cell_name(0, col);
    name.trim_end_matches('1').to_string()
}

fn html_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The selection as an HTML table and as tab-separated text.
fn selection_clip(wb: &Workbook) -> (String, String) {
    let rows = wb.selection_rows();
    let tsv = rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n");
    let mut html = String::from("<table border=\"1\" style=\"border-collapse:collapse\">");
    for r in &rows {
        html.push_str("<tr>");
        for c in r {
            html.push_str(&format!("<td>{}</td>", html_esc(c)));
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");
    (tsv, html)
}

/// The Calculations commands (`ui.calc.*`).
pub(crate) fn command(app: &mut WordApp, id: &str, p: &Value) -> Result<Value, String> {
    let comma = comma(app);
    match id {
        "ui.calc.new" => {
            app.calc = Workbook::default();
            Ok(json!({"new": true}))
        }
        "ui.calc.open" => match p.get("path").and_then(Value::as_str) {
            Some(path) => open_path(app, path),
            None => {
                let _ = app.ask_file(FileDialogRequest::Open { purpose: "spreadsheet".into() }, AfterPick::OpenSheet);
                Ok(json!({"pending": "open"}))
            }
        },
        "ui.calc.save" | "ui.calc.saveAs" | "ui.calc.exportCsv" => {
            let csv = id == "ui.calc.exportCsv";
            if let Some(path) = p.get("path").and_then(Value::as_str) {
                return save_path(app, path);
            }
            if id == "ui.calc.save"
                && let Some(path) = app.calc.path.clone()
            {
                return save_path(app, &path);
            }
            let stem = app
                .calc
                .path
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_stem())
                .map_or_else(|| crate::tl!("Sheet1").to_string(), |s| s.to_string_lossy().into_owned());
            let name = format!("{stem}.{}", if csv { "csv" } else { "xlsx" });
            let _ = app.ask_file(FileDialogRequest::Save { name }, AfterPick::SaveSheet);
            Ok(json!({"pending": "save"}))
        }
        "ui.calc.copy" | "ui.calc.cut" => {
            let (tsv, html) = selection_clip(&app.calc);
            if let Some(ctx) = app.ctx.clone() {
                crate::system_clipboard::put(app, &ctx, &tsv, &html, "");
            }
            if id == "ui.calc.cut" {
                clear_selection(app);
            }
            Ok(json!({"text": tsv}))
        }
        "ui.calc.paste" => {
            let text = match p.get("text").and_then(Value::as_str) {
                Some(t) => t.to_string(),
                None => app.services.clipboard_text.as_ref().and_then(|f| f()).unwrap_or_default(),
            };
            paste(app, &text);
            Ok(json!({"pasted": true}))
        }
        "ui.calc.clear" => {
            clear_selection(app);
            Ok(json!({}))
        }
        "ui.calc.function" => {
            let f = p.get("name").and_then(Value::as_str).unwrap_or("SUM").to_ascii_uppercase();
            if !matches!(f.as_str(), "SUM" | "AVERAGE" | "COUNT" | "MAX" | "MIN") {
                return Err(format!("unknown function {f:?}"));
            }
            let at = app.calc.sel;
            let range = auto_range(&app.calc, at);
            app.calc.snapshot();
            app.calc.put(at, &format!("={f}({range})"));
            app.calc.recalc(comma);
            app.calc.bar_for = None;
            Ok(json!({"cell": sheet::cell_name(at.0, at.1), "formula": app.calc.raw(at)}))
        }
        "ui.calc.insertRow" | "ui.calc.deleteRow" | "ui.calc.insertColumn" | "ui.calc.deleteColumn" => {
            let ((r0, c0), (r1, c1)) = app.calc.range();
            app.calc.snapshot();
            let old = std::mem::take(&mut app.calc.cells);
            let (n_r, n_c) = (r1 - r0 + 1, c1 - c0 + 1);
            for ((r, c), v) in old {
                let to = match id {
                    "ui.calc.insertRow" => Some(if r >= r0 { (r + n_r, c) } else { (r, c) }),
                    "ui.calc.deleteRow" if (r0..=r1).contains(&r) => None,
                    "ui.calc.deleteRow" => Some(if r > r1 { (r - n_r, c) } else { (r, c) }),
                    "ui.calc.insertColumn" => Some(if c >= c0 { (r, c + n_c) } else { (r, c) }),
                    "ui.calc.deleteColumn" if (c0..=c1).contains(&c) => None,
                    _ => Some(if c > c1 { (r, c - n_c) } else { (r, c) }),
                };
                if let Some(k) = to.filter(|k| k.0 < MAX_ROWS && k.1 < MAX_COLS) {
                    app.calc.cells.insert(k, v);
                }
            }
            app.calc.dirty = true;
            app.calc.recalc(comma);
            app.calc.bar_for = None;
            Ok(json!({}))
        }
        "ui.calc.sortAscending" | "ui.calc.sortDescending" => {
            sort(app, id == "ui.calc.sortDescending");
            Ok(json!({}))
        }
        "ui.calc.undo" | "ui.calc.redo" => {
            let (from, to) = if id == "ui.calc.undo" { (&mut app.calc.undo, &mut app.calc.redo) } else { (&mut app.calc.redo, &mut app.calc.undo) };
            if let Some(prev) = from.pop() {
                to.push(std::mem::replace(&mut app.calc.cells, prev));
                app.calc.dirty = true;
                app.calc.recalc(comma);
                app.calc.bar_for = None;
            }
            Ok(json!({}))
        }
        "ui.calc.toDocument" => to_document(app),
        "ui.calc.fromDocument" => from_document(app),
        _ => Err(format!("unknown command {id}")),
    }
}

fn clear_selection(app: &mut WordApp) {
    let ((r0, c0), (r1, c1)) = app.calc.range();
    app.calc.snapshot();
    app.calc.cells.retain(|(r, c), _| !((r0..=r1).contains(r) && (c0..=c1).contains(c)));
    app.calc.dirty = true;
    let comma = comma(app);
    app.calc.recalc(comma);
    app.calc.bar_for = None;
}

/// Tab-separated text (from Excel, Word or Lexicon) into the cells from the active one.
fn paste(app: &mut WordApp, text: &str) {
    if text.is_empty() {
        return;
    }
    let (r0, c0) = app.calc.range().0;
    app.calc.snapshot();
    for (dr, line) in text.trim_end_matches(['\n', '\r']).split('\n').enumerate().take(MAX_ROWS) {
        for (dc, cell) in line.trim_end_matches('\r').split('\t').enumerate().take(MAX_COLS) {
            app.calc.put((r0 + dr, c0 + dc), cell.trim());
        }
    }
    let comma = comma(app);
    app.calc.recalc(comma);
    app.calc.bar_for = None;
}

/// Whether a cell holds a number or a formula.
fn numeric(wb: &Workbook, c: Cell) -> bool {
    let raw = wb.raw(c);
    raw.starts_with('=') || sheet::parse_number(raw).is_some()
}

/// AutoSum's range for cell `at`: the numbers right above it, else right to its left.
fn auto_range(wb: &Workbook, (r, c): Cell) -> String {
    if let Some(top) = (0..r).rev().take_while(|&i| numeric(wb, (i, c))).last() {
        return format!("{}:{}", sheet::cell_name(top, c), sheet::cell_name(r - 1, c));
    }
    if let Some(first) = (0..c).rev().take_while(|&i| numeric(wb, (r, i))).last() {
        return format!("{}:{}", sheet::cell_name(r, first), sheet::cell_name(r, c - 1));
    }
    if r > 0 { format!("{}:{}", sheet::cell_name(0, c), sheet::cell_name(r - 1, c)) } else { sheet::cell_name(0, 0) }
}

/// Sort the used rows (all but a header row of text) by the active cell's column.
fn sort(app: &mut WordApp, descending: bool) {
    let (rows, _) = app.calc.used();
    if rows < 2 {
        return;
    }
    let col = app.calc.sel.1;
    // A first row of text over numbers is a header and stays on top.
    let (_, cols) = app.calc.used();
    let header = (0..cols).all(|c| !numeric(&app.calc, (0, c))) && (1..rows).any(|r| (0..cols).any(|c| numeric(&app.calc, (r, c))));
    let start = usize::from(header);
    let mut lines: Vec<BTreeMap<usize, String>> =
        (start..rows).map(|r| app.calc.cells.range((r, 0)..(r, MAX_COLS)).map(|((_, c), v)| (*c, v.clone())).collect()).collect();
    let key = |l: &BTreeMap<usize, String>| l.get(&col).cloned().unwrap_or_default();
    lines.sort_by(|a, b| {
        let (ka, kb) = (key(a), key(b));
        // Empty keys go last either way.
        if ka.is_empty() != kb.is_empty() {
            return if ka.is_empty() { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less };
        }
        let o = match (sheet::parse_number(&ka), sheet::parse_number(&kb)) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => ka.to_lowercase().cmp(&kb.to_lowercase()),
        };
        if descending { o.reverse() } else { o }
    });
    app.calc.snapshot();
    app.calc.cells.retain(|(r, _), _| *r < start);
    for (i, line) in lines.into_iter().enumerate() {
        for (c, v) in line {
            app.calc.cells.insert((start + i, c), v);
        }
    }
    app.calc.dirty = true;
    let comma = comma(app);
    app.calc.recalc(comma);
    app.calc.bar_for = None;
}

/// The selection (or the whole sheet when one cell is selected) into the document at the caret
/// as a Word table; the formulas' results go in as text.
fn to_document(app: &mut WordApp) -> Result<Value, String> {
    let saved = (app.calc.anchor, app.calc.sel);
    if app.calc.anchor == app.calc.sel {
        let (rows, cols) = app.calc.used();
        if rows == 0 {
            return Err(crate::tl!("The sheet is empty.").into());
        }
        app.calc.anchor = (0, 0);
        app.calc.sel = (rows - 1, cols.max(1) - 1);
    }
    let (tsv, html) = selection_clip(&app.calc);
    (app.calc.anchor, app.calc.sel) = saved;
    let r = app.run("edit.paste", json!({"html": html, "text": tsv}));
    if r.is_ok() {
        app.ui.tab = "Home".into();
        app.status(crate::tl!("The table is in the document."));
    }
    r
}

/// The document's table at the caret into a new sheet.
fn from_document(app: &mut WordApp) -> Result<Value, String> {
    let f = &app.session.sel.focus;
    let Some((tpath, _, _)) = f.path.cell() else {
        return Err(crate::tl!("Put the caret in a table in the document first.").into());
    };
    let Some(t) = app.session.doc.table(f.story, &tpath) else {
        return Err(crate::tl!("Put the caret in a table in the document first.").into());
    };
    let rows: Vec<Vec<String>> = t
        .rows
        .iter()
        .take(MAX_ROWS)
        .map(|row| {
            row.cells
                .iter()
                .take(MAX_COLS)
                .map(|cell| cell.blocks.iter().filter_map(|b| b.as_para()).map(|p| p.plain_text()).collect::<Vec<_>>().join(" ").trim().to_string())
                .collect()
        })
        .collect();
    let comma = comma(app);
    app.calc = Workbook::default();
    app.calc.load(rows, comma);
    app.calc.dirty = true;
    Ok(json!({"rows": app.calc.used().0}))
}

/// Open a spreadsheet file into the sheet.
pub(crate) fn open_path(app: &mut WordApp, path: &str) -> Result<Value, String> {
    let bytes = wordcraft_engine::io::read_file(path)?;
    let lower = path.to_ascii_lowercase();
    let rows = if lower.ends_with(".csv") || lower.ends_with(".tsv") || lower.ends_with(".txt") {
        wordcraft_engine::cmd::mailings::parse_csv(&String::from_utf8_lossy(&bytes))
    } else {
        wordcraft_formats::sheet::read_grid(&bytes)?
    };
    let comma = comma(app);
    app.calc = Workbook::default();
    app.calc.load(rows, comma);
    // Only an .xlsx is saved back in place; other formats are saved as a new workbook.
    app.calc.path = lower.ends_with(".xlsx").then(|| path.to_string());
    app.ui.tab = "Calculations".into();
    Ok(json!({"opened": path}))
}

/// Save the sheet: `.csv` as CSV, anything else as an Excel workbook.
pub(crate) fn save_path(app: &mut WordApp, path: &str) -> Result<Value, String> {
    let csv = path.to_ascii_lowercase().ends_with(".csv");
    let path = if csv || path.to_ascii_lowercase().ends_with(".xlsx") { path.to_string() } else { format!("{path}.xlsx") };
    let bytes = if csv { app.calc.to_csv(comma(app)).into_bytes() } else { app.calc.to_xlsx()? };
    wordcraft_engine::io::write_file(&path, &bytes)?;
    if !csv {
        app.calc.path = Some(path.clone());
        app.calc.dirty = false;
    }
    let name = std::path::Path::new(&path).file_name().map_or(path.clone(), |n| n.to_string_lossy().into_owned());
    app.status(crate::i18n::fmt(crate::tl!("Saved {name}"), &[("name", &name)]));
    Ok(json!({"saved": path}))
}

/// The Calculations tab's ribbon.
pub(crate) fn tab(app: &mut WordApp, ui: &mut Ui) {
    use crate::widgets::{big, group, small};
    group(ui, "Sheet", None, app, |ui, app| {
        big(ui, app, "file", "New Sheet", "ui.calc.new", json!({}), false);
        big(ui, app, "open", "Open", "ui.calc.open", json!({}), false);
        big(ui, app, "save", "Save", "ui.calc.save", json!({}), false);
        ui.vertical(|ui| {
            small(ui, app, "saveAs", Some("Save As Excel"), "Save As Excel", "ui.calc.saveAs", json!({}), false);
            small(ui, app, "export", Some("Save As CSV"), "Save As CSV", "ui.calc.exportCsv", json!({}), false);
        });
    });
    group(ui, "Clipboard", None, app, |ui, app| {
        big(ui, app, "paste", "Paste", "ui.calc.paste", json!({}), false);
        ui.vertical(|ui| {
            small(ui, app, "cut", Some("Cut"), "Cut", "ui.calc.cut", json!({}), false);
            small(ui, app, "copy", Some("Copy"), "Copy", "ui.calc.copy", json!({}), false);
            small(ui, app, "clear", Some("Clear"), "Clear", "ui.calc.clear", json!({}), false);
        });
    });
    group(ui, "AutoSum", None, app, |ui, app| {
        big(ui, app, "formula", "Sum", "ui.calc.function", json!({"name": "SUM"}), false);
        ui.vertical(|ui| {
            small(ui, app, "formula", Some("Average"), "Average", "ui.calc.function", json!({"name": "AVERAGE"}), false);
            small(ui, app, "formula", Some("Count"), "Count", "ui.calc.function", json!({"name": "COUNT"}), false);
        });
        ui.vertical(|ui| {
            small(ui, app, "formula", Some("Max"), "Max", "ui.calc.function", json!({"name": "MAX"}), false);
            small(ui, app, "formula", Some("Min"), "Min", "ui.calc.function", json!({"name": "MIN"}), false);
        });
    });
    group(ui, "Rows & Columns", None, app, |ui, app| {
        ui.vertical(|ui| {
            small(ui, app, "insertAbove", Some("Insert Rows"), "Insert Rows", "ui.calc.insertRow", json!({}), false);
            small(ui, app, "deleteTable", Some("Delete Rows"), "Delete Rows", "ui.calc.deleteRow", json!({}), false);
        });
        ui.vertical(|ui| {
            small(ui, app, "plus", Some("Insert Columns"), "Insert Columns", "ui.calc.insertColumn", json!({}), false);
            small(ui, app, "minus", Some("Delete Columns"), "Delete Columns", "ui.calc.deleteColumn", json!({}), false);
        });
    });
    group(ui, "Sort", None, app, |ui, app| {
        ui.vertical(|ui| {
            small(ui, app, "sort", Some("Sort A to Z"), "Sort A to Z", "ui.calc.sortAscending", json!({}), false);
            small(ui, app, "sort", Some("Sort Z to A"), "Sort Z to A", "ui.calc.sortDescending", json!({}), false);
        });
    });
    group(ui, "Document", None, app, |ui, app| {
        big(ui, app, "table", "Insert into Document", "ui.calc.toDocument", json!({}), false);
        big(ui, app, "updateTable", "Table from Document", "ui.calc.fromDocument", json!({}), false);
    });
}

/// Commit the in-cell editor.
fn commit_edit(app: &mut WordApp) {
    if let Some(text) = app.calc.editing.take() {
        let at = app.calc.sel;
        if text != app.calc.raw(at) {
            app.calc.snapshot();
            app.calc.put(at, text.trim());
            let comma = comma(app);
            app.calc.recalc(comma);
        }
        app.calc.bar_for = None;
    }
}

fn move_sel(app: &mut WordApp, dr: isize, dc: isize, extend: bool) {
    let (r, c) = app.calc.sel;
    let r = r.saturating_add_signed(dr).min(MAX_ROWS - 1);
    let c = c.saturating_add_signed(dc).min(MAX_COLS - 1);
    app.calc.sel = (r, c);
    if !extend {
        app.calc.anchor = (r, c);
    }
    app.calc.reveal = true;
}

/// The keyboard while the grid has the focus (no text field does).
fn keys(app: &mut WordApp, ctx: &egui::Context) {
    let events = ctx.input(|i| i.events.clone());
    for e in events {
        match e {
            egui::Event::Copy => {
                let _ = command(app, "ui.calc.copy", &json!({}));
            }
            egui::Event::Cut => {
                let _ = command(app, "ui.calc.cut", &json!({}));
            }
            egui::Event::Paste(t) => paste(app, &t),
            egui::Event::Text(t) | egui::Event::Ime(egui::ImeEvent::Commit(t)) if !t.is_empty() => {
                app.calc.editing = Some(t);
                app.calc.edit_started = true;
            }
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                let shift = modifiers.shift;
                match key {
                    egui::Key::ArrowUp => move_sel(app, -1, 0, shift),
                    egui::Key::ArrowDown => move_sel(app, 1, 0, shift),
                    egui::Key::ArrowLeft => move_sel(app, 0, -1, shift),
                    egui::Key::ArrowRight => move_sel(app, 0, 1, shift),
                    egui::Key::Enter => move_sel(app, if shift { -1 } else { 1 }, 0, false),
                    egui::Key::Tab => move_sel(app, 0, if shift { -1 } else { 1 }, false),
                    egui::Key::PageDown => move_sel(app, 20, 0, shift),
                    egui::Key::PageUp => move_sel(app, -20, 0, shift),
                    egui::Key::Delete | egui::Key::Backspace => clear_selection(app),
                    egui::Key::F2 => {
                        app.calc.editing = Some(app.calc.raw(app.calc.sel).to_string());
                        app.calc.edit_started = true;
                    }
                    egui::Key::Z if modifiers.command => {
                        let _ = command(app, if shift { "ui.calc.redo" } else { "ui.calc.undo" }, &json!({}));
                    }
                    egui::Key::Y if modifiers.command => {
                        let _ = command(app, "ui.calc.redo", &json!({}));
                    }
                    egui::Key::S if modifiers.command => {
                        let _ = command(app, if shift { "ui.calc.saveAs" } else { "ui.calc.save" }, &json!({}));
                    }
                    egui::Key::O if modifiers.command => {
                        let _ = command(app, "ui.calc.open", &json!({}));
                    }
                    egui::Key::A if modifiers.command => {
                        let (rows, cols) = app.calc.used();
                        app.calc.anchor = (0, 0);
                        app.calc.sel = (rows.max(1) - 1, cols.max(1) - 1);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// The formula bar: the active cell's name and what it holds.
fn formula_bar(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
    let at = app.calc.sel;
    if app.calc.bar_for != Some(at) {
        app.calc.bar_for = Some(at);
        app.calc.bar = app.calc.raw(at).to_string();
    }
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        let (r0c0, r1c1) = app.calc.range();
        let name = if r0c0 == r1c1 {
            sheet::cell_name(at.0, at.1)
        } else {
            format!("{}:{}", sheet::cell_name(r0c0.0, r0c0.1), sheet::cell_name(r1c1.0, r1c1.1))
        };
        egui::Frame::NONE.fill(t.input).stroke(Stroke::new(1.0, t.input_border)).inner_margin(egui::Margin::symmetric(6, 3)).show(ui, |ui| {
            ui.set_width(84.0);
            ui.label(egui::RichText::new(name).strong());
        });
        ui.label(egui::RichText::new("fx").italics().strong().color(t.text_dim));
        let w = (ui.available_width() - 8.0).max(80.0);
        let r = ui.add(egui::TextEdit::singleline(&mut app.calc.bar).hint_text(crate::tl!("Type a formula, e.g. =SUM(B2:B5)")).desired_width(w));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let text = app.calc.bar.trim().to_string();
            app.calc.snapshot();
            app.calc.put(at, &text);
            let comma = comma(app);
            app.calc.recalc(comma);
            app.calc.bar_for = None;
            move_sel(app, 1, 0, false);
        }
    });
}

/// The Calculations view: formula bar and grid, in place of the page.
pub(crate) fn show(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    app.canvas.focused = false;
    egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin::symmetric(10, 4)).show(ui, |ui| formula_bar(app, ui, &t));
    let (rows_used, cols_used) = app.calc.used();
    let rows = (rows_used.max(app.calc.sel.0 + 1) + SPARE_ROWS).min(MAX_ROWS);
    let cols = (cols_used.max(app.calc.sel.1 + 1) + SPARE_COLS).min(MAX_COLS);
    // Column edges from the left of the first column.
    let mut edges = Vec::with_capacity(cols + 1);
    let mut x = 0.0;
    edges.push(0.0);
    for c in 0..cols {
        x += app.calc.width(c);
        edges.push(x);
    }
    let total = vec2(HEAD_W + x, HEAD_H + rows as f32 * ROW_H);
    let grid_focus = ui.ctx().memory(|m| m.focused().is_none());
    if grid_focus && app.calc.editing.is_none() && app.dialog.is_none() {
        let ctx = ui.ctx().clone();
        keys(app, &ctx);
    }
    let reveal = std::mem::take(&mut app.calc.reveal);
    let sel = app.calc.sel;
    let mut scroll = egui::ScrollArea::both().auto_shrink(false).id_salt("calc_grid");
    if reveal {
        // Keep the active cell in view after keyboard moves.
        scroll = scroll.scroll_offset(vec2(0.0, 0.0)).vertical_scroll_offset((sel.0 as f32 * ROW_H - 200.0).max(0.0));
    }
    let edges_ref = &edges;
    scroll.show_viewport(ui, |ui, vp| {
        let (rect, resp) = ui.allocate_exact_size(total, Sense::click_and_drag());
        let origin = rect.min;
        let painter = ui.painter_at(Rect::from_min_size(origin + vp.min.to_vec2(), vp.size()));
        let col_at = |x: f32| -> usize { edges_ref.partition_point(|e| *e <= x).saturating_sub(1).min(cols.saturating_sub(1)) };
        let row_at = |y: f32| -> usize { ((y / ROW_H).max(0.0) as usize).min(rows.saturating_sub(1)) };
        let cell_rect = |r: usize, c: usize| -> Rect {
            let x0 = origin.x + HEAD_W + edges_ref.get(c).copied().unwrap_or(0.0);
            let x1 = origin.x + HEAD_W + edges_ref.get(c + 1).copied().unwrap_or(0.0);
            let y0 = origin.y + HEAD_H + r as f32 * ROW_H;
            Rect::from_min_max(pos2(x0, y0), pos2(x1, y0 + ROW_H))
        };
        let r_first = row_at((vp.min.y - HEAD_H).max(0.0));
        let r_last = row_at(vp.max.y);
        let c_first = col_at((vp.min.x - HEAD_W).max(0.0));
        let c_last = col_at(vp.max.x);
        painter.rect_filled(Rect::from_min_size(origin + vp.min.to_vec2(), vp.size()), 0.0, t.input);
        let font = regular(12.5);
        for r in r_first..=r_last {
            for c in c_first..=c_last {
                let cr = cell_rect(r, c);
                if app.calc.selected((r, c)) {
                    painter.rect_filled(cr, 0.0, t.selection);
                }
                painter.rect_stroke(cr, 0.0, Stroke::new(0.5, t.border), egui::StrokeKind::Inside);
                let text = app.calc.display((r, c));
                if !text.is_empty() {
                    let number = sheet::parse_number(&text).is_some() || text.starts_with('#');
                    let (anchor, x) = if number { (Align2::RIGHT_CENTER, cr.max.x - 4.0) } else { (Align2::LEFT_CENTER, cr.min.x + 4.0) };
                    painter.with_clip_rect(cr.intersect(painter.clip_rect())).text(pos2(x, cr.center().y), anchor, text, font.clone(), t.text);
                }
            }
        }
        // The active cell.
        let active = cell_rect(sel.0, sel.1);
        painter.rect_stroke(active, 0.0, Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
        // Headers stay at the top and left of the view.
        let top = origin.y + vp.min.y;
        let left = origin.x + vp.min.x;
        let ((sr0, sc0), (sr1, sc1)) = app.calc.range();
        for c in c_first..=c_last {
            let cr = cell_rect(0, c);
            let hr = Rect::from_min_max(pos2(cr.min.x, top), pos2(cr.max.x, top + HEAD_H));
            painter.rect_filled(hr, 0.0, if (sc0..=sc1).contains(&c) { t.checked } else { t.panel });
            painter.rect_stroke(hr, 0.0, Stroke::new(0.5, t.border_strong), egui::StrokeKind::Inside);
            painter.text(hr.center(), Align2::CENTER_CENTER, col_letters(c), font.clone(), t.text_dim);
        }
        for r in r_first..=r_last {
            let cr = cell_rect(r, 0);
            let hr = Rect::from_min_max(pos2(left, cr.min.y), pos2(left + HEAD_W, cr.max.y));
            painter.rect_filled(hr, 0.0, if (sr0..=sr1).contains(&r) { t.checked } else { t.panel });
            painter.rect_stroke(hr, 0.0, Stroke::new(0.5, t.border_strong), egui::StrokeKind::Inside);
            painter.text(hr.center(), Align2::CENTER_CENTER, (r + 1).to_string(), font.clone(), t.text_dim);
        }
        painter.rect_filled(Rect::from_min_size(pos2(left, top), vec2(HEAD_W, HEAD_H)), 0.0, t.panel);

        // Pointer: click selects (Shift extends), drag extends, double-click edits; a header
        // selects its whole column or row.
        let hit = |p: Pos2| -> Option<(Option<usize>, Option<usize>)> {
            let local = p - origin;
            let in_col_head = p.y < top + HEAD_H;
            let in_row_head = p.x < left + HEAD_W;
            let r = (!in_col_head).then(|| row_at(local.y - HEAD_H));
            let c = (!in_row_head).then(|| col_at(local.x - HEAD_W));
            Some((r, c))
        };
        if let Some(p) = resp.interact_pointer_pos()
            && (resp.clicked() || resp.drag_started() || (resp.dragged() && app.calc.dragging))
            && let Some((r, c)) = hit(p)
        {
            if app.calc.editing.is_some() {
                commit_edit(app);
            }
            let shift = ui.input(|i| i.modifiers.shift);
            let (rows_u, cols_u) = app.calc.used();
            match (r, c) {
                (Some(r), Some(c)) => {
                    if resp.drag_started() {
                        app.calc.dragging = true;
                    }
                    if !(shift || (resp.dragged() && !resp.drag_started())) {
                        app.calc.anchor = (r, c);
                    }
                    app.calc.sel = (r, c);
                }
                (None, Some(c)) => {
                    app.calc.anchor = (0, c);
                    app.calc.sel = (rows_u.max(1) - 1, c);
                }
                (Some(r), None) => {
                    app.calc.anchor = (r, 0);
                    app.calc.sel = (r, cols_u.max(1) - 1);
                }
                (None, None) => {
                    app.calc.anchor = (0, 0);
                    app.calc.sel = (rows_u.max(1) - 1, cols_u.max(1) - 1);
                }
            }
            ui.ctx().memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        if resp.drag_stopped() {
            app.calc.dragging = false;
        }
        if resp.double_clicked() {
            app.calc.editing = Some(app.calc.raw(app.calc.sel).to_string());
            app.calc.edit_started = true;
        }
        // The in-cell editor.
        if app.calc.editing.is_some() {
            let cr = cell_rect(app.calc.sel.0, app.calc.sel.1).expand(1.0);
            let w = cr.width().max(160.0);
            let er = Rect::from_min_size(cr.min, vec2(w, cr.height()));
            painter.rect_filled(er, 0.0, t.input);
            let mut text = app.calc.editing.take().unwrap_or_default();
            let id = ui.id().with("calc_cell_editor");
            let out = ui.put(er, egui::TextEdit::singleline(&mut text).id(id).margin(vec2(3.0, 2.0)).desired_width(w));
            if std::mem::take(&mut app.calc.edit_started) {
                out.request_focus();
                // The typed character starts the text: put the cursor after it.
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let end = egui::text::CCursor::new(text.chars().count());
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                    state.store(ui.ctx(), id);
                }
            }
            let (enter, tab, esc, shift) =
                ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Tab), i.key_pressed(egui::Key::Escape), i.modifiers.shift));
            if esc {
                app.calc.editing = None;
            } else if out.lost_focus() {
                app.calc.editing = Some(text);
                commit_edit(app);
                if enter {
                    move_sel(app, if shift { -1 } else { 1 }, 0, false);
                } else if tab {
                    move_sel(app, 0, if shift { -1 } else { 1 }, false);
                }
            } else {
                app.calc.editing = Some(text);
            }
        }
        let _ = Color32::TRANSPARENT;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_engine::Session;

    fn app() -> WordApp {
        WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default())
    }

    fn grid(a: &mut WordApp, rows: &[&[&str]]) {
        a.calc.load(rows.iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect(), false);
    }

    #[test]
    fn formulas_autosum_sort_and_rows() {
        let mut a = app();
        grid(&mut a, &[&["Item", "Qty"], &["b", "3"], &["a", "2"], &["", ""]]);
        a.calc.sel = (3, 1);
        a.calc.anchor = (3, 1);
        let r = command(&mut a, "ui.calc.function", &json!({"name": "SUM"})).unwrap();
        assert_eq!(r["formula"], "=SUM(B2:B3)");
        assert_eq!(a.calc.display((3, 1)), "5");
        a.calc.sel = (1, 0);
        a.calc.anchor = (1, 0);
        command(&mut a, "ui.calc.sortAscending", &json!({})).unwrap();
        assert_eq!(a.calc.raw((1, 0)), "a", "sorted under the header");
        assert_eq!(a.calc.raw((0, 0)), "Item");
        command(&mut a, "ui.calc.insertRow", &json!({})).unwrap();
        assert_eq!(a.calc.raw((2, 0)), "a");
        command(&mut a, "ui.calc.undo", &json!({})).unwrap();
        assert_eq!(a.calc.raw((1, 0)), "a");
    }

    #[test]
    fn copy_paste_and_files() {
        let mut a = app();
        grid(&mut a, &[&["x", "1"], &["y", "=B1*2"]]);
        a.calc.anchor = (0, 0);
        a.calc.sel = (1, 1);
        let (tsv, html) = selection_clip(&a.calc);
        assert_eq!(tsv, "x\t1\ny\t2");
        assert!(html.contains("<td>y</td><td>2</td>"));
        a.calc.anchor = (5, 0);
        a.calc.sel = (5, 0);
        command(&mut a, "ui.calc.paste", &json!({"text": "p\t7\nq\t8"})).unwrap();
        assert_eq!(a.calc.raw((6, 1)), "8");
        // An Excel workbook round trip keeps the formula.
        let dir = std::env::temp_dir().join(format!("lexicon-calc-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.xlsx").to_string_lossy().into_owned();
        save_path(&mut a, &path).unwrap();
        let mut b = app();
        open_path(&mut b, &path).unwrap();
        assert_eq!(b.calc.raw((1, 1)), "=B1*2");
        assert_eq!(b.calc.display((1, 1)), "2");
        assert_eq!(b.ui.tab, "Calculations");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sheets_go_into_the_document_and_come_back() {
        let mut a = app();
        grid(&mut a, &[&["Name", "Sum"], &["a", "=2+3"]]);
        command(&mut a, "ui.calc.toDocument", &json!({})).unwrap();
        assert!(a.session.doc.body.iter().any(|b| b.as_table().is_some()), "a Word table");
        let text = a.session.doc.plain_text(wordcraft_doc::StoryRef::Body);
        assert!(text.contains('5') && text.contains("Name"), "{text}");
        // Back: the caret into the table, then import.
        let b = a.session.doc.body.iter().position(|b| b.as_table().is_some()).unwrap() as u32;
        a.session.sel = wordcraft_engine::Selection::caret(wordcraft_doc::Pos {
            story: wordcraft_doc::StoryRef::Body,
            path: wordcraft_doc::Path(vec![b, 1, 1, 0]),
            off: 0,
        });
        command(&mut a, "ui.calc.fromDocument", &json!({})).unwrap();
        assert_eq!(a.calc.raw((1, 1)), "5");
        assert!(command(&mut a, "ui.calc.nope", &json!({})).is_err());
    }
}
