//! Lexicon's Calculations: write a grid of cells as an Excel workbook (`.xlsx`, ECMA-376
//! SpreadsheetML) with one sheet. Numbers are numbers, formulas keep their formula with the last
//! calculated value cached (Excel recalculates on open), and everything else is text.

use std::io::{Cursor, Write};

/// One cell to write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    /// The text shown (for text cells, and the cached value of a formula that gives text).
    pub text: String,
    /// The number, for number cells and formulas that give one.
    pub number: Option<f64>,
    /// The formula without its `=`, in Excel's syntax (English names, `,` between arguments).
    pub formula: Option<String>,
}

/// Most rows and columns written.
const MAX_ROWS: usize = 1_048_576;
const MAX_COLS: usize = 16_384;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            // Control characters other than tab and newlines aren't allowed in XML 1.0.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => o.push(c),
        }
    }
    o
}

/// `A`, `B`, … `Z`, `AA` for a column from 0.
fn col_name(col: usize) -> String {
    let mut letters = Vec::new();
    let mut c = col.saturating_add(1);
    while c > 0 && letters.len() < 4 {
        letters.push(char::from(b'A' + ((c - 1) % 26) as u8));
        c = (c - 1) / 26;
    }
    letters.iter().rev().collect()
}

fn sheet_xml(rows: &[Vec<Cell>]) -> String {
    let mut o = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    );
    for (r, row) in rows.iter().enumerate().take(MAX_ROWS) {
        if row.iter().all(|c| c.text.is_empty() && c.number.is_none() && c.formula.is_none()) {
            continue;
        }
        o.push_str(&format!("<row r=\"{}\">", r + 1));
        for (c, cell) in row.iter().enumerate().take(MAX_COLS) {
            let at = format!("{}{}", col_name(c), r + 1);
            match (&cell.formula, cell.number) {
                (Some(f), Some(v)) if v.is_finite() => o.push_str(&format!("<c r=\"{at}\"><f>{}</f><v>{v}</v></c>", esc(f))),
                (Some(f), _) => o.push_str(&format!("<c r=\"{at}\" t=\"str\"><f>{}</f><v>{}</v></c>", esc(f), esc(&cell.text))),
                (None, Some(v)) if v.is_finite() => o.push_str(&format!("<c r=\"{at}\"><v>{v}</v></c>")),
                _ if cell.text.is_empty() => {}
                _ => o.push_str(&format!("<c r=\"{at}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", esc(&cell.text))),
            }
        }
        o.push_str("</row>");
    }
    o.push_str("</sheetData></worksheet>");
    o
}

/// The workbook's bytes.
pub fn write(sheet_name: &str, rows: &[Vec<Cell>]) -> Result<Vec<u8>, String> {
    let name: String = sheet_name.chars().filter(|c| !matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']')).take(31).collect();
    let name = if name.trim().is_empty() { "Sheet1".to_string() } else { name };
    let parts: [(&str, String); 6] = [
        (
            "[Content_Types].xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>".into(),
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".into(),
        ),
        (
            "xl/workbook.xml",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"{}\" sheetId=\"1\" r:id=\"rId1\"/></sheets><calcPr fullCalcOnLoad=\"1\"/></workbook>",
                esc(&name)
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>".into(),
        ),
        (
            "xl/styles.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Arial\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>".into(),
        ),
        ("xl/worksheets/sheet1.xml", sheet_xml(rows)),
    ];
    let err = |e: zip::result::ZipError| format!("XLSX: {e}");
    let ioerr = |e: std::io::Error| format!("XLSX: {e}");
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (path, data) in &parts {
        z.start_file(*path, deflated).map_err(err)?;
        z.write_all(data.as_bytes()).map_err(ioerr)?;
    }
    Ok(z.finish().map_err(err)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_workbooks_read_back_with_formulas() {
        let t = |s: &str| Cell { text: s.into(), ..Default::default() };
        let n = |v: f64| Cell { text: v.to_string(), number: Some(v), formula: None };
        let rows = vec![
            vec![t("Item"), t("Qty <&>")],
            vec![t("a"), n(2.0)],
            vec![],
            vec![t("Total"), Cell { text: "2".into(), number: Some(2.0), formula: Some("SUM(B2:B3)".into()) }],
        ];
        let bytes = write("Расчёт", &rows).unwrap();
        let grid = crate::sheet::read_grid(&bytes).unwrap();
        assert_eq!(grid[0], vec!["Item", "Qty <&>"]);
        assert_eq!(grid[1], vec!["a", "2"]);
        assert!(grid[2].is_empty(), "the blank row stays");
        assert_eq!(grid[3], vec!["Total", "=SUM(B2:B3)"]);
        assert_eq!(crate::sheet::sheet_names(&bytes).unwrap(), vec!["Расчёт"]);
        assert_eq!(col_name(0), "A");
        assert_eq!(col_name(27), "AB");
    }
}
