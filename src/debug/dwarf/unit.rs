use std::{num::NonZeroU64, path::PathBuf};

use super::R;
use anyhow::Result;
use gimli::Reader;

#[derive(Debug)]
pub struct Unit {
    /// Provides direct access to `gimli`
    unit: gimli::Unit<R>,
    files: Vec<PathBuf>,
    /// Information about the lines in this unit.
    /// Each of these is theoretically a breakable program statement
    /// (whether it is actually depends on if istrumentation code was generated for it)
    lines: Vec<LineRow>,
}

#[derive(PartialEq, Debug, Clone)]
#[repr(Rust, packed)]
pub struct LineRow {
    /// PC address within code segment
    address: u64,
    /// Index of corresponding file within this unit
    file_index: u64,
    /// Line number within file (one-indexed)
    line: u64,
    /// Column number (0 is left edge)
    column: u64,
}

impl Unit {
    pub fn clone(&self, dwarf: &gimli::Dwarf<R>) -> Self {
        let unit = {
            dwarf
                .unit(self.unit.header.clone())
                .expect("clone unit should not fail")
        };

        Self {
            unit,
            files: self.files.clone(),
            lines: self.lines.clone(),
        }
    }

    pub fn new(dwarf: &gimli::Dwarf<R>, unit: gimli::Unit<R>) -> Result<Unit> {
        let mut files = vec![];
        let mut lines = vec![];
        if let Some(ref lp) = unit.line_program {
            let mut rows = lp.clone().rows();
            lines = parse_lines(&mut rows)?;
            files = parse_files(dwarf, &unit, &rows)?;
        }

        Ok(Unit { unit, files, lines })
    }

    pub fn unit(&self) -> &gimli::Unit<R> {
        &self.unit
    }
}

fn parse_lines(
    rows: &mut gimli::LineRows<R, gimli::IncompleteLineProgram<R>>,
) -> gimli::Result<Vec<LineRow>> {
    let mut lines = vec![];
    while let Some((_, line_row)) = rows.next_row()? {
        let column = match line_row.column() {
            gimli::ColumnType::LeftEdge => 0,
            gimli::ColumnType::Column(x) => x.get(),
        };

        if !line_row.is_stmt() {
            continue;
        }

        lines.push(LineRow {
            address: line_row.address(),
            file_index: line_row.file_index(),
            line: line_row.line().map(NonZeroU64::get).unwrap_or(0),
            column,
        })
    }

    lines.shrink_to_fit();
    Ok(lines)
}

fn parse_files(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R>,
    rows: &gimli::LineRows<R, gimli::IncompleteLineProgram<R>>,
) -> gimli::Result<Vec<PathBuf>> {
    let mut files = vec![];
    let header = rows.header();
    match header.file(0) {
        Some(file) => files.push(render_file_path(unit, file, header, dwarf)?),
        None => files.push(PathBuf::default()),
    }
    let mut index = 1;
    while let Some(file) = header.file(index) {
        files.push(render_file_path(unit, file, header, dwarf)?);
        index += 1;
    }

    files.shrink_to_fit();
    Ok(files)
}

fn render_file_path(
    dw_unit: &gimli::Unit<R>,
    file: &gimli::FileEntry<R>,
    header: &gimli::LineProgramHeader<R>,
    sections: &gimli::Dwarf<R>,
) -> gimli::Result<PathBuf> {
    let mut path = if let Some(ref comp_dir) = dw_unit.comp_dir {
        PathBuf::from(comp_dir.to_string_lossy()?.as_ref())
    } else {
        PathBuf::new()
    };

    if file.directory_index() != 0
        && let Some(directory) = file.directory(header)
    {
        path.push(
            sections
                .attr_string(dw_unit, directory)?
                .to_string_lossy()?
                .as_ref(),
        );
    }

    path.push(
        sections
            .attr_string(dw_unit, file.path_name())?
            .to_string_lossy()?
            .as_ref(),
    );

    Ok(path)
}
