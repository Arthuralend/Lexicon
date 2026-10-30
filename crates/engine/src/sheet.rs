//! Lexicon: spreadsheet formulas over a table's cells (the Excel tab).
//!
//! A cell is addressed as in a spreadsheet: columns are letters (A, B, … Z, AA…), rows numbers
//! from 1, so `B3` is the third row's second cell. A formula starts with `=` and has numbers,
//! cell references, ranges (`A1:B3`), the operators `+ - * / ^`, a postfix `%`, comparisons
//! (`= <> < > <= >=`, giving 1 or 0), parentheses and functions: `SUM`, `AVERAGE`, `MIN`, `MAX`,
//! `COUNT`, `PRODUCT`, `ROUND`, `ABS`, `SQRT`, `INT`, `IF`. Function names are also accepted
//! in Russian (`СУММ`, `СРЗНАЧ`, `МИН`, `МАКС`, `СЧЁТ`, `ПРОИЗВЕД`, `ОКРУГЛ`, `ЕСЛИ`), and Word's
//! table directions `ABOVE`, `LEFT`, `BELOW` and `RIGHT` stand for the cells in that direction.
//! Arguments are separated by `;` or `,`.
//!
//! Everything here is input-derived: lengths, recursion and references are bounded, and no
//! operation panics.

use std::collections::HashMap;

/// A formula's error, shown in its cell as Excel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetError {
    /// Division by zero (`#DIV/0!`).
    DivZero,
    /// A reference outside the table (`#REF!`).
    Ref,
    /// An unknown function or name (`#NAME?`).
    Name,
    /// A malformed formula or a wrong argument (`#VALUE!`).
    Value,
    /// The formula depends on itself (`#CYCLE!`).
    Cycle,
}

impl SheetError {
    pub fn text(self) -> &'static str {
        match self {
            SheetError::DivZero => "#DIV/0!",
            SheetError::Ref => "#REF!",
            SheetError::Name => "#NAME?",
            SheetError::Value => "#VALUE!",
            SheetError::Cycle => "#CYCLE!",
        }
    }
}

/// Longest formula read, in characters.
const MAX_FORMULA: usize = 2000;
/// Deepest nesting of parentheses and of formulas referring to formulas.
const MAX_DEPTH: usize = 64;
/// Most cells one range may name.
const MAX_RANGE: usize = 100_000;

/// The name of the cell at `row`, `col` (from 0): `A1`, `B3`, `AA10`.
pub fn cell_name(row: usize, col: usize) -> String {
    let mut letters = Vec::new();
    let mut c = col.saturating_add(1);
    while c > 0 && letters.len() < 8 {
        let rem = (c - 1) % 26;
        letters.push(char::from(b'A' + rem as u8));
        c = (c - 1) / 26;
    }
    letters.reverse();
    format!("{}{}", letters.into_iter().collect::<String>(), row.saturating_add(1))
}

/// A cell's text read as a number: `12`, `-3.5`, `1 234,56`, `12%`, `$40`, `40 ₽`. `None` for
/// empty or non-numeric text.
pub fn parse_number(text: &str) -> Option<f64> {
    let t: String =
        text.chars().filter(|c| !c.is_whitespace() && !matches!(c, '\u{00A0}' | '\u{202F}' | '$' | '€' | '£' | '₽' | '֏' | '₾' | '₼')).collect();
    if t.is_empty() {
        return None;
    }
    let (t, percent) = match t.strip_suffix('%') {
        Some(rest) => (rest.to_string(), true),
        None => (t, false),
    };
    // One comma and no dot is a decimal comma (1234,5); commas beside a dot group thousands.
    let t = if t.contains('.') {
        t.replace(',', "")
    } else if t.matches(',').count() == 1 {
        t.replace(',', ".")
    } else {
        t.replace(',', "")
    };
    let t = t.replace('−', "-");
    if !t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')) || !t.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let v: f64 = t.parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    Some(if percent { v / 100.0 } else { v })
}

/// A result as a cell shows it: integers without decimals, other numbers with up to six
/// decimals, with a decimal comma when `comma` is set.
pub fn format_number(v: f64, comma: bool) -> String {
    if !v.is_finite() {
        return SheetError::Value.text().into();
    }
    let rounded = (v * 1e6).round() / 1e6;
    let mut s = if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        format!("{rounded:.0}")
    } else {
        let s = format!("{rounded:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    if s == "-0" {
        s = "0".into();
    }
    if comma { s.replace('.', ",") } else { s }
}

/// The table being calculated: each cell's text, and the formulas of the cells that have one
/// (keyed by row and column, from 0, without the leading `=`).
pub struct Sheet<'a> {
    pub cells: &'a [Vec<String>],
    pub formulas: &'a HashMap<(usize, usize), String>,
}

impl Sheet<'_> {
    /// Every formula's result.
    pub fn evaluate_all(&self) -> HashMap<(usize, usize), Result<f64, SheetError>> {
        let mut memo = HashMap::new();
        let keys: Vec<(usize, usize)> = self.formulas.keys().copied().collect();
        for k in keys {
            let mut stack = Vec::new();
            let _ = self.cell_value(k, &mut memo, &mut stack);
        }
        memo.into_iter().map(|(k, v)| (k, v.map(|v| v.unwrap_or(0.0)))).collect()
    }

    /// The value of cell `k`: its formula's result, its number, or `None` when it holds text.
    fn cell_value(
        &self,
        k: (usize, usize),
        memo: &mut HashMap<(usize, usize), Result<Option<f64>, SheetError>>,
        stack: &mut Vec<(usize, usize)>,
    ) -> Result<Option<f64>, SheetError> {
        let Some(formula) = self.formulas.get(&k) else {
            if self.cells.get(k.0).and_then(|r| r.get(k.1)).is_none() {
                return Err(SheetError::Ref);
            }
            return Ok(self.cells.get(k.0).and_then(|r| r.get(k.1)).and_then(|t| parse_number(t)));
        };
        if let Some(v) = memo.get(&k) {
            return *v;
        }
        if stack.contains(&k) {
            return Err(SheetError::Cycle);
        }
        if stack.len() >= MAX_DEPTH {
            return Err(SheetError::Cycle);
        }
        stack.push(k);
        let r = self.eval_formula(formula, k, memo, stack).map(Some);
        stack.pop();
        memo.insert(k, r);
        r
    }

    fn eval_formula(
        &self,
        formula: &str,
        at: (usize, usize),
        memo: &mut HashMap<(usize, usize), Result<Option<f64>, SheetError>>,
        stack: &mut Vec<(usize, usize)>,
    ) -> Result<f64, SheetError> {
        let src: Vec<char> = formula.trim().trim_start_matches('=').chars().take(MAX_FORMULA).collect();
        let tokens = tokenize(&src)?;
        let mut p = Parser { tokens: &tokens, pos: 0, depth: 0 };
        let expr = p.expr()?;
        if p.pos != tokens.len() {
            return Err(SheetError::Value);
        }
        let mut ev = Eval { sheet: self, at, memo, stack };
        ev.value(&expr)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Cell(usize, usize),
    Name(String),
    Op(char),
    Cmp(&'static str),
    LParen,
    RParen,
    Sep,
    Colon,
}

fn tokenize(src: &[char]) -> Result<Vec<Tok>, SheetError> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = src.get(i) {
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && src.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            while src.get(i).is_some_and(|d| d.is_ascii_digit() || *d == '.') {
                i += 1;
            }
            // A decimal comma right after digits (1,5) when followed by a digit and no separator use.
            let s: String = src.get(start..i).unwrap_or_default().iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| SheetError::Value)?));
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while src.get(i).is_some_and(|d| d.is_alphanumeric() || *d == '_' || *d == '$' || *d == '.') {
                i += 1;
            }
            let word: String = src.get(start..i).unwrap_or_default().iter().filter(|c| **c != '$').collect();
            match parse_ref(&word) {
                Some((r, c)) => out.push(Tok::Cell(r, c)),
                None => out.push(Tok::Name(word.to_uppercase())),
            }
        } else {
            i += 1;
            let next = src.get(i).copied();
            out.push(match c {
                '+' | '-' | '*' | '/' | '^' | '%' | '−' | '×' | '÷' => Tok::Op(match c {
                    '−' => '-',
                    '×' => '*',
                    '÷' => '/',
                    other => other,
                }),
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                ';' | ',' => Tok::Sep,
                ':' => Tok::Colon,
                '=' => Tok::Cmp("="),
                '<' if next == Some('=') => {
                    i += 1;
                    Tok::Cmp("<=")
                }
                '<' if next == Some('>') => {
                    i += 1;
                    Tok::Cmp("<>")
                }
                '<' => Tok::Cmp("<"),
                '>' if next == Some('=') => {
                    i += 1;
                    Tok::Cmp(">=")
                }
                '>' => Tok::Cmp(">"),
                _ => return Err(SheetError::Value),
            });
        }
    }
    Ok(out)
}

/// `B3` → (2, 1). Up to three letters and seven digits.
fn parse_ref(word: &str) -> Option<(usize, usize)> {
    let letters: String = word.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let digits = word.get(letters.len()..)?;
    if letters.is_empty() || letters.len() > 3 || digits.is_empty() || digits.len() > 7 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut col = 0usize;
    for ch in letters.to_ascii_uppercase().bytes() {
        col = col.checked_mul(26)?.checked_add(usize::from(ch - b'A') + 1)?;
    }
    let row: usize = digits.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, col - 1))
}

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Cell(usize, usize),
    Range((usize, usize), (usize, usize)),
    Dir(String),
    Neg(Box<Expr>),
    Percent(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Cmp(&'static str, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

struct Parser<'a> {
    tokens: &'a [Tok],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expr(&mut self) -> Result<Expr, SheetError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(SheetError::Value);
        }
        let left = self.additive()?;
        let r = if let Some(Tok::Cmp(op)) = self.peek().cloned() {
            self.pos += 1;
            let right = self.additive()?;
            Ok(Expr::Cmp(op, Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        };
        self.depth -= 1;
        r
    }

    fn additive(&mut self) -> Result<Expr, SheetError> {
        let mut left = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let right = self.term()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, SheetError> {
        let mut left = self.power()?;
        while let Some(Tok::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.pos += 1;
            let right = self.power()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn power(&mut self) -> Result<Expr, SheetError> {
        let base = self.unary()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.pos += 1;
            let exp = self.power()?;
            return Ok(Expr::Bin('^', Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn unary(&mut self) -> Result<Expr, SheetError> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(SheetError::Value);
                }
                let e = self.unary()?;
                self.depth -= 1;
                Ok(Expr::Neg(Box::new(e)))
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Result<Expr, SheetError> {
        let mut e = self.primary()?;
        while let Some(Tok::Op('%')) = self.peek() {
            self.pos += 1;
            e = Expr::Percent(Box::new(e));
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, SheetError> {
        match self.next() {
            Some(Tok::Num(v)) => Ok(Expr::Num(v)),
            Some(Tok::Cell(r, c)) => {
                if let Some(Tok::Colon) = self.peek() {
                    self.pos += 1;
                    match self.next() {
                        Some(Tok::Cell(r2, c2)) => Ok(Expr::Range((r, c), (r2, c2))),
                        _ => Err(SheetError::Value),
                    }
                } else {
                    Ok(Expr::Cell(r, c))
                }
            }
            Some(Tok::LParen) => {
                let e = self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err(SheetError::Value),
                }
            }
            Some(Tok::Name(name)) => {
                if let Some(Tok::LParen) = self.peek() {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if let Some(Tok::RParen) = self.peek() {
                        self.pos += 1;
                        return Ok(Expr::Call(name, args));
                    }
                    loop {
                        args.push(self.expr()?);
                        match self.next() {
                            Some(Tok::Sep) => continue,
                            Some(Tok::RParen) => break,
                            _ => return Err(SheetError::Value),
                        }
                    }
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Dir(name))
                }
            }
            _ => Err(SheetError::Value),
        }
    }
}

struct Eval<'a, 'b> {
    sheet: &'a Sheet<'b>,
    at: (usize, usize),
    memo: &'a mut HashMap<(usize, usize), Result<Option<f64>, SheetError>>,
    stack: &'a mut Vec<(usize, usize)>,
}

impl Eval<'_, '_> {
    fn value(&mut self, e: &Expr) -> Result<f64, SheetError> {
        let v = match e {
            Expr::Num(v) => *v,
            Expr::Cell(r, c) => self.sheet.cell_value((*r, *c), self.memo, self.stack)?.unwrap_or(0.0),
            Expr::Range(..) | Expr::Dir(_) => {
                // A range alone (=A1:A3) sums, as a convenience.
                let vals = self.values(e)?;
                vals.iter().sum()
            }
            Expr::Neg(x) => -self.value(x)?,
            Expr::Percent(x) => self.value(x)? / 100.0,
            Expr::Bin(op, a, b) => {
                let (a, b) = (self.value(a)?, self.value(b)?);
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' if b == 0.0 => return Err(SheetError::DivZero),
                    '/' => a / b,
                    '^' => a.powf(b),
                    _ => return Err(SheetError::Value),
                }
            }
            Expr::Cmp(op, a, b) => {
                let (a, b) = (self.value(a)?, self.value(b)?);
                let t = match *op {
                    "=" => (a - b).abs() < 1e-9,
                    "<>" => (a - b).abs() >= 1e-9,
                    "<" => a < b,
                    ">" => a > b,
                    "<=" => a <= b,
                    ">=" => a >= b,
                    _ => return Err(SheetError::Value),
                };
                if t { 1.0 } else { 0.0 }
            }
            Expr::Call(name, args) => self.call(name, args)?,
        };
        if v.is_finite() { Ok(v) } else { Err(SheetError::Value) }
    }

    /// The numbers an argument stands for: a range's or direction's numeric cells, or one value.
    fn values(&mut self, e: &Expr) -> Result<Vec<f64>, SheetError> {
        let cells: Vec<(usize, usize)> = match e {
            Expr::Range((r1, c1), (r2, c2)) => {
                let (r1, r2) = ((*r1).min(*r2), (*r1).max(*r2));
                let (c1, c2) = ((*c1).min(*c2), (*c1).max(*c2));
                let rows = self.sheet.cells.len();
                let r2 = r2.min(rows.saturating_sub(1));
                let mut v = Vec::new();
                for r in r1..=r2 {
                    let cols = self.sheet.cells.get(r).map_or(0, Vec::len);
                    for c in c1..=c2.min(cols.saturating_sub(1)) {
                        if v.len() >= MAX_RANGE {
                            return Err(SheetError::Ref);
                        }
                        v.push((r, c));
                    }
                }
                v
            }
            Expr::Dir(d) => {
                let (r, c) = self.at;
                let rows = self.sheet.cells.len();
                let cols = self.sheet.cells.get(r).map_or(0, Vec::len);
                match d.as_str() {
                    "ABOVE" | "ВЫШЕ" => (0..r).rev().map(|r| (r, c)).take_while(|k| self.numeric_or_formula(*k)).collect(),
                    "BELOW" | "НИЖЕ" => (r + 1..rows).map(|r| (r, c)).take_while(|k| self.numeric_or_formula(*k)).collect(),
                    "LEFT" | "ЛЕВЕЕ" => (0..c).rev().map(|c| (r, c)).take_while(|k| self.numeric_or_formula(*k)).collect(),
                    "RIGHT" | "ПРАВЕЕ" => (c + 1..cols).map(|c| (r, c)).take_while(|k| self.numeric_or_formula(*k)).collect(),
                    _ => return Err(SheetError::Name),
                }
            }
            other => return Ok(vec![self.value(other)?]),
        };
        let mut out = Vec::new();
        for k in cells {
            if let Some(v) = self.sheet.cell_value(k, self.memo, self.stack)? {
                out.push(v);
            }
        }
        Ok(out)
    }

    /// Word's ABOVE and LEFT stop at the first cell that is neither a number nor a formula
    /// (a header), skipping empty cells next to the formula.
    fn numeric_or_formula(&self, k: (usize, usize)) -> bool {
        self.sheet.formulas.contains_key(&k)
            || self.sheet.cells.get(k.0).and_then(|r| r.get(k.1)).is_some_and(|t| t.trim().is_empty() || parse_number(t).is_some())
    }

    fn all(&mut self, args: &[Expr]) -> Result<Vec<f64>, SheetError> {
        let mut out = Vec::new();
        for a in args {
            out.extend(self.values(a)?);
        }
        Ok(out)
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<f64, SheetError> {
        let arg = |this: &mut Self, i: usize| -> Result<f64, SheetError> { this.value(args.get(i).ok_or(SheetError::Value)?) };
        Ok(match name {
            "SUM" | "СУММ" => self.all(args)?.iter().sum(),
            "AVERAGE" | "СРЗНАЧ" => {
                let v = self.all(args)?;
                if v.is_empty() {
                    return Err(SheetError::DivZero);
                }
                v.iter().sum::<f64>() / v.len() as f64
            }
            "MIN" | "МИН" => self.all(args)?.into_iter().reduce(f64::min).unwrap_or(0.0),
            "MAX" | "МАКС" => self.all(args)?.into_iter().reduce(f64::max).unwrap_or(0.0),
            "COUNT" | "СЧЁТ" | "СЧЕТ" => self.all(args)?.len() as f64,
            "PRODUCT" | "ПРОИЗВЕД" => self.all(args)?.iter().product(),
            "ABS" => arg(self, 0)?.abs(),
            "SQRT" | "КОРЕНЬ" => {
                let v = arg(self, 0)?;
                if v < 0.0 {
                    return Err(SheetError::Value);
                }
                v.sqrt()
            }
            "INT" | "ЦЕЛОЕ" => arg(self, 0)?.floor(),
            "ROUND" | "ОКРУГЛ" => {
                let v = arg(self, 0)?;
                let n = if args.len() > 1 { arg(self, 1)?.clamp(-10.0, 10.0).round() } else { 0.0 };
                let f = 10f64.powi(n as i32);
                (v * f).round() / f
            }
            "IF" | "ЕСЛИ" => {
                if arg(self, 0)? != 0.0 {
                    arg(self, 1)?
                } else if args.len() > 2 {
                    arg(self, 2)?
                } else {
                    0.0
                }
            }
            _ => return Err(SheetError::Name),
        })
    }
}

/// A formula (with or without its `=`) in Excel's syntax: English function names and `,` between
/// arguments. `None` when Excel has no equivalent (Word's `ABOVE`, `LEFT`…), so the value is saved
/// instead.
pub fn to_excel(formula: &str) -> Option<String> {
    let src = formula.trim().trim_start_matches('=');
    if src.is_empty() || src.chars().count() > MAX_FORMULA {
        return None;
    }
    let mut out = String::with_capacity(src.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| -> Option<()> {
        if word.is_empty() {
            return Some(());
        }
        let up = word.to_uppercase();
        let mapped = match up.as_str() {
            "СУММ" => "SUM",
            "СРЗНАЧ" => "AVERAGE",
            "МИН" => "MIN",
            "МАКС" => "MAX",
            "СЧЁТ" | "СЧЕТ" => "COUNT",
            "ПРОИЗВЕД" => "PRODUCT",
            "ОКРУГЛ" => "ROUND",
            "ЕСЛИ" => "IF",
            "КОРЕНЬ" => "SQRT",
            "ЦЕЛОЕ" => "INT",
            "ABOVE" | "BELOW" | "LEFT" | "RIGHT" | "ВЫШЕ" | "НИЖЕ" | "ЛЕВЕЕ" | "ПРАВЕЕ" => return None,
            _ => "",
        };
        out.push_str(if mapped.is_empty() { word.as_str() } else { mapped });
        word.clear();
        Some(())
    };
    for c in src.chars() {
        if c.is_alphanumeric() || c == '_' || c == '$' || c == '.' {
            word.push(c);
            continue;
        }
        flush(&mut word, &mut out)?;
        out.push(match c {
            ';' => ',',
            '−' => '-',
            '×' => '*',
            '÷' => '/',
            other => other,
        });
    }
    flush(&mut word, &mut out)?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect()
    }

    fn eval(cells: &[Vec<String>], at: (usize, usize), f: &str) -> Result<f64, SheetError> {
        let mut formulas = HashMap::new();
        formulas.insert(at, f.to_string());
        let sheet = Sheet { cells, formulas: &formulas };
        sheet.evaluate_all().get(&at).copied().unwrap_or(Err(SheetError::Value))
    }

    #[test]
    fn formulas_in_excel_syntax() {
        assert_eq!(to_excel("=СУММ(B2:B5)").as_deref(), Some("SUM(B2:B5)"));
        assert_eq!(to_excel("=ОКРУГЛ(A1/3; 2)").as_deref(), Some("ROUND(A1/3, 2)"));
        assert_eq!(to_excel("=SUM(ABOVE)"), None);
        assert_eq!(to_excel("=B2*C2").as_deref(), Some("B2*C2"));
    }

    #[test]
    fn names_and_numbers() {
        assert_eq!(cell_name(0, 0), "A1");
        assert_eq!(cell_name(2, 1), "B3");
        assert_eq!(cell_name(9, 26), "AA10");
        assert_eq!(parse_ref("B3"), Some((2, 1)));
        assert_eq!(parse_ref("AA10"), Some((9, 26)));
        assert_eq!(parse_ref("A0"), None);
        assert_eq!(parse_number("1 234,5"), Some(1234.5));
        assert_eq!(parse_number("1,234.5"), Some(1234.5));
        assert_eq!(parse_number("12%"), Some(0.12));
        assert_eq!(parse_number("40 ₽"), Some(40.0));
        assert_eq!(parse_number("Итого"), None);
        assert_eq!(format_number(3.0, false), "3");
        assert_eq!(format_number(2.5, true), "2,5");
        assert_eq!(format_number(1.0 / 3.0, false), "0.333333");
    }

    #[test]
    fn sums_ranges_directions_and_functions() {
        let g = grid(&[&["Item", "Qty", "Price"], &["a", "2", "10"], &["b", "3", "1,5"], &["Total", "", ""]]);
        assert_eq!(eval(&g, (3, 1), "=SUM(B2:B3)"), Ok(5.0));
        assert_eq!(eval(&g, (3, 1), "=SUM(ABOVE)"), Ok(5.0));
        assert_eq!(eval(&g, (3, 2), "=СУММ(ВЫШЕ)"), Ok(11.5));
        assert_eq!(eval(&g, (3, 2), "=B2*C2+B3*C3"), Ok(24.5));
        assert_eq!(eval(&g, (3, 2), "=AVERAGE(C2:C3)"), Ok(5.75));
        assert_eq!(eval(&g, (3, 2), "=ROUND(10/3; 2)"), Ok(3.33));
        assert_eq!(eval(&g, (3, 2), "=IF(B2>1, 100, 0)"), Ok(100.0));
        assert_eq!(eval(&g, (3, 2), "=MAX(B2:C3)-MIN(B2:C3)"), Ok(8.5));
        assert_eq!(eval(&g, (3, 2), "=2^3*50%"), Ok(4.0));
        assert_eq!(eval(&g, (1, 0), "=SUM(LEFT)"), Ok(0.0));
        assert_eq!(eval(&g, (3, 0), "=COUNT(B2:C3)"), Ok(4.0));
    }

    #[test]
    fn errors_never_panic() {
        let g = grid(&[&["1", "0"]]);
        assert_eq!(eval(&g, (0, 0), "=A1+1"), Err(SheetError::Cycle));
        assert_eq!(eval(&g, (0, 1), "=A1/B1"), Err(SheetError::Cycle));
        assert_eq!(eval(&g, (0, 0), "=1/0"), Err(SheetError::DivZero));
        assert_eq!(eval(&g, (0, 0), "=FOO(1)"), Err(SheetError::Name));
        assert_eq!(eval(&g, (0, 0), "=Z99"), Err(SheetError::Ref));
        for junk in ["=", "=(((", "=1+", "=SUM(", "=A1:", "=)", "=#", &"(".repeat(5000), &"-".repeat(5000), "=ZZZZZZZZ1:A1", "=1e999"] {
            let _ = eval(&g, (0, 0), junk);
        }
    }
}
