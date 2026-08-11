//! Rain (vertical) and flat (horizontal) source forms — port of forms.py.

use crate::lex::{Axis, Cell, LoadError};

const GUTTER: usize = 2;

pub struct Program {
    pub boot_cells: Option<Vec<Cell>>,
    pub strands: Vec<(String, Vec<Cell>)>,
    pub axis: Axis,
}

type LResult<T> = Result<T, LoadError>;

pub fn parse_source(text: &str) -> LResult<Program> {
    if let Some(idx) = text.find('\t') {
        let row = text[..idx].matches('\n').count() as u32 + 1;
        return Err(LoadError::new(
            "tab characters break the grid — use spaces",
            Some((row, 1)),
        ));
    }
    let lines: Vec<&str> = text.lines().collect();
    if !lines.is_empty() && lines[0].trim() == "⇓" {
        parse_rain(&lines)
    } else {
        parse_flat(&lines)
    }
}

fn line_cells(line: &str, row: u32, start_col: usize) -> Vec<Cell> {
    line.chars()
        .enumerate()
        .filter(|(c, _)| *c >= start_col)
        .map(|(c, ch)| Cell { ch, row, col: c as u32 + 1 })
        .collect()
}

fn parse_flat(lines: &[&str]) -> LResult<Program> {
    let mut sections: [Vec<Vec<Cell>>; 2] = [Vec::new(), Vec::new()];
    let has_divider = lines.iter().any(|l| l.trim() == "⇊");
    let mut section = if has_divider { 0 } else { 1 };
    for (i, line) in lines.iter().enumerate() {
        let row = i as u32 + 1;
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if stripped == "⇊" {
            if section == 1 && !sections[0].is_empty() {
                return Err(LoadError::new("second ⇊ divider", Some((row, 1))));
            }
            section = 1;
            continue;
        }
        if stripped.starts_with('⋮') {
            if sections[section].is_empty() {
                return Err(LoadError::new(
                    "⋮ continuation with nothing to continue",
                    Some((row, 1)),
                ));
            }
            let start = line.chars().position(|c| c == '⋮').unwrap() + 1;
            let bucket = sections[section].last_mut().unwrap();
            bucket.push(Cell { ch: ' ', row, col: start as u32 });
            bucket.extend(line_cells(line, row, start));
            continue;
        }
        sections[section].push(line_cells(line, row, 0));
    }
    let boot = if sections[0].is_empty() {
        None
    } else {
        let mut boot: Vec<Cell> = Vec::new();
        for cells in &sections[0] {
            if !boot.is_empty() {
                boot.push(Cell { ch: ' ', row: cells[0].row, col: 0 });
            }
            boot.extend(cells.iter().copied());
        }
        Some(boot)
    };
    let strands = sections[1]
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| (format!("row {}", c[0].row), c.clone()))
        .collect();
    Ok(Program { boot_cells: boot, strands, axis: Axis::Row })
}

fn parse_rain(lines: &[&str]) -> LResult<Program> {
    let rows: Vec<Vec<char>> = lines[1..].iter().map(|l| l.chars().collect()).collect();
    let row0 = 2u32; // file row number of the first grid row
    let divider = rows.iter().position(|r| {
        r.iter().find(|c| **c != ' ').map(|c| *c == '⇊').unwrap_or(false)
    });
    let (pre_end, main_start) = match divider {
        Some(d) => (d, d + 1),
        None => (0, 0),
    };
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let cell = |r: usize, c: usize| Cell {
        ch: *rows[r].get(c).unwrap_or(&' '),
        row: r as u32 + row0,
        col: c as u32 + 1,
    };

    let mut boot: Option<Vec<Cell>> = None;
    if divider.is_some() {
        let mut acc: Vec<Cell> = Vec::new();
        for c in 0..width {
            let col: Vec<Cell> = (0..pre_end).map(|r| cell(r, c)).collect();
            if col.iter().any(|x| x.ch != ' ') {
                if !acc.is_empty() {
                    acc.push(Cell { ch: ' ', row: row0, col: c as u32 + 1 });
                }
                acc.extend(col);
            }
        }
        if !acc.is_empty() {
            boot = Some(acc);
        }
    }

    let mut strands = Vec::new();
    for c in 0..width {
        let col: Vec<Cell> = (main_start..rows.len()).map(|r| cell(r, c)).collect();
        if col.iter().any(|x| x.ch != ' ') {
            strands.push((format!("col {}", c + 1), col));
        }
    }
    Ok(Program { boot_cells: boot, strands, axis: Axis::Col })
}

// ── renderers ──────────────────────────────────────────────────────────
fn strand_strings(prog: &Program) -> (Option<String>, Vec<String>) {
    let boot = prog.boot_cells.as_ref().map(|cells| {
        cells.iter().map(|c| c.ch).collect::<String>().trim().to_string()
    });
    let strands = prog
        .strands
        .iter()
        .map(|(_, cells)| {
            cells.iter().map(|c| c.ch).collect::<String>().trim_end().to_string()
        })
        .collect();
    (boot, strands)
}

pub fn to_rain(text: &str) -> LResult<String> {
    let prog = parse_source(text)?;
    if prog.axis == Axis::Col {
        return Err(LoadError::new("already in rain form", None));
    }
    let (boot, strands) = strand_strings(&prog);
    let mut out: Vec<String> = vec!["⇓".into()];
    if let Some(b) = boot {
        if !b.is_empty() {
            out.extend(b.chars().map(|c| c.to_string()));
            out.push("⇊".into());
        }
    }
    let cols: Vec<Vec<char>> = strands.iter().map(|s| s.chars().collect()).collect();
    let height = cols.iter().map(|s| s.len()).max().unwrap_or(0);
    for r in 0..height {
        let mut row = String::new();
        for s in &cols {
            row.push(*s.get(r).unwrap_or(&' '));
            row.push_str(&" ".repeat(GUTTER));
        }
        out.push(row.trim_end().to_string());
    }
    Ok(out.join("\n") + "\n")
}

pub fn to_flat(text: &str) -> LResult<String> {
    let prog = parse_source(text)?;
    if prog.axis == Axis::Row {
        return Err(LoadError::new("already in flat form", None));
    }
    let (boot, strands) = strand_strings(&prog);
    let mut out: Vec<String> = Vec::new();
    if let Some(b) = boot {
        out.push(b.split_whitespace().collect::<Vec<_>>().join(" "));
        out.push("⇊".into());
    }
    out.extend(strands);
    Ok(out.join("\n") + "\n")
}
