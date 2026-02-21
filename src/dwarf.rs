use crate::types::{BreakpointRange, LocationInfo};
use gimli::{EndianSlice, LittleEndian, Reader};
use object::{Object, ObjectSection};
use std::borrow::Cow;
use std::collections::HashMap;
use wasmer_wasix::virtual_fs::{AsyncReadExt, FileSystem, mem_fs};

/// ============================================================================
/// HELPERS
/// ============================================================================

/// Get the WASM bytes from the filesystem.
/// Returns the WASM bytes or an error if the file does not exist.
pub async fn get_wasm_bytes(
    fs: &mem_fs::FileSystem,
    path: &str,
) -> Result<Vec<u8>, std::io::Error> {
    let mut file = fs
        .new_open_options()
        .read(true)
        .open(path)
        .expect(&format!("{} exists", path));

    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)
        .await
        .expect("Read main.wasm");

    Ok(wasm_bytes)
}

/// Build a filename from a file entry, handling directory prefixes.
fn build_filename<R: Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R>,
    file_entry: &gimli::FileEntry<R>,
) -> Result<String, gimli::Error> {
    let mut path = String::new();

    // Add directory if present
    if let Some(dir) = file_entry.directory(unit.line_program.as_ref().unwrap().header()) {
        let dir_str = dwarf.attr_string(unit, dir)?;
        let dir_str = dir_str.to_string_lossy()?;
        if !dir_str.is_empty() && dir_str != "." {
            path.push_str(&dir_str);
            if !path.ends_with('/') {
                path.push('/');
            }
        }
    }

    // Add filename
    let name = dwarf.attr_string(unit, file_entry.path_name())?;
    path.push_str(&name.to_string_lossy()?);

    Ok(path)
}

/// ============================================================================
/// DWARF PARSING
/// ============================================================================

/// Parse DWARF debug info from WASM bytes to extract breakpoint locations.
///
/// Returns (locations, files) where:
/// - locations: All possible breakpoint locations (file index, line, col)
/// - files: Deduplicated list of source filenames
pub fn parse_dwarf_info(wasm_bytes: &[u8]) -> (Vec<LocationInfo>, Vec<String>) {
    match parse_dwarf_inner(wasm_bytes) {
        Ok(result) => result,
        Err(e) => {
            web_sys::console::error_1(&format!("DWARF parsing error: {:?}", e).into());
            (vec![], vec![])
        }
    }
}

fn parse_dwarf_inner(wasm_bytes: &[u8]) -> Result<(Vec<LocationInfo>, Vec<String>), gimli::Error> {
    // Parse the WASM file
    let object = match object::File::parse(wasm_bytes) {
        Ok(obj) => obj,
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to parse WASM: {:?}", e).into());
            return Ok((vec![], vec![]));
        }
    };

    // Load DWARF sections from the WASM file
    let load_section = |id: gimli::SectionId| -> Result<Cow<'_, [u8]>, gimli::Error> {
        Ok(object
            .section_by_name(id.name())
            .and_then(|s| s.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[])))
    };

    let dwarf_sections = gimli::DwarfSections::load(load_section)?;
    let dwarf =
        dwarf_sections.borrow(|section| EndianSlice::new(Cow::as_ref(section), LittleEndian));

    let mut locations = Vec::new();
    let mut files: Vec<String> = Vec::new();
    let mut file_map: HashMap<String, u32> = HashMap::new();

    // Iterate over compilation units
    let mut units = dwarf.units();
    while let Some(header) = units.next()? {
        let unit = dwarf.unit(header)?;

        // Get the line program for this unit
        let Some(program) = unit.line_program.clone() else {
            continue;
        };

        // Execute the line program to get all rows
        let mut rows = program.rows();
        while let Some((header, row)) = rows.next_row()? {
            // Skip rows that aren't statement beginnings (not useful for breakpoints)
            if !row.is_stmt() {
                continue;
            }

            // Get the file entry
            let Some(file_entry) = row.file(header) else {
                continue;
            };

            // Build the filename
            let filename = build_filename(&dwarf, &unit, file_entry)?;

            // Get or insert file index
            let file_idx = if let Some(&idx) = file_map.get(&filename) {
                idx
            } else {
                let idx = files.len() as u32;
                files.push(filename.clone());
                file_map.insert(filename, idx);
                idx
            };

            let line = row.line().map(|l| l.get()).unwrap_or(0) as u32;
            let col = match row.column() {
                gimli::ColumnType::LeftEdge => 0,
                gimli::ColumnType::Column(c) => c.get() as u32,
            };

            locations.push(LocationInfo {
                file: file_idx,
                line,
                col,
                address: row.address(),
            });
        }
    }

    Ok((locations, files))
}

/// ============================================================================
/// WASM INSTRUMENTATION
/// ============================================================================
use std::collections::{BTreeMap, BTreeSet};

/// Instrument a WASM binary by inserting `bkpt` calls at line boundaries.
///
/// Only inserts breakpoints at addresses from DWARF line info (line boundaries),
/// NOT at every WASM instruction. Multiple WASM instructions from the same
/// source line will share a single breakpoint.
///
/// Adds import: `(import "debug" "bkpt" (func (param i32)))`
/// The i32 param is the breakpoint index (1-based, 0 is sentinel).
///
/// Returns:
/// - instrumented wasm bytes
/// - list of breakpoint indices that were actually inserted
pub fn instrument_binary(
    wasm_bytes: &[u8],
    locations: &[LocationInfo],
) -> Result<(Vec<u8>, Vec<LocationInfo>, Vec<BreakpointRange>), String> {
    use walrus::ir::*;
    use walrus::*;
    use std::collections::{HashMap, HashSet};

    // One candidate per (file, line), selected directly from DWARF rows.
    // This gives line-level breakpoints rather than instruction-level breakpoints.
    let mut line_candidates: BTreeMap<(u32, u32), LocationInfo> = BTreeMap::new();
    for loc in locations {
        if loc.line == 0 {
            continue;
        }
        let key = (loc.file, loc.line);
        line_candidates
            .entry(key)
            .and_modify(|existing| {
                if loc.address < existing.address {
                    *existing = loc.clone();
                }
            })
            .or_insert_with(|| loc.clone());
    }
    let candidates: Vec<LocationInfo> = line_candidates.into_values().collect();

    let mut module = ModuleConfig::new()
        .parse(wasm_bytes)
        .map_err(|e| format!("Failed to parse WASM: {:?}", e))?;

    // Add import: (import "debug" "bkpt" (func (param i32)))
    let bkpt_type = module.types.add(&[ValType::I32], &[]);
    let (bkpt_func_id, _) = module.add_import_func("debug", "bkpt", bkpt_type);

    let func_ids: Vec<FunctionId> = module
        .funcs
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Local(_) => Some(f.id()),
            _ => None,
        })
        .collect();

    // Bridge from walrus location ids to concrete insertion sites.
    let mut instrloc_to_site: HashMap<(FunctionId, u32), (InstrSeqId, usize)> = HashMap::new();
    let mut abs_offsets: Vec<(u64, FunctionId, u32)> = Vec::new();
    let mut rel_offsets: Vec<(u64, FunctionId, u32)> = Vec::new();

    #[derive(Default)]
    struct Collector {
        stack: Vec<(InstrSeqId, usize)>,
        pairs: Vec<(u32, InstrSeqId, usize)>,
    }
    impl<'instr> Visitor<'instr> for Collector {
        fn start_instr_seq(&mut self, seq: &'instr InstrSeq) {
            self.stack.push((seq.id(), 0));
        }
        fn end_instr_seq(&mut self, _seq: &'instr InstrSeq) {
            let _ = self.stack.pop();
        }
        fn visit_instr(&mut self, _instr: &'instr Instr, loc: &InstrLocId) {
            if let Some((seq, idx)) = self.stack.last_mut() {
                self.pairs.push((loc.data(), *seq, *idx));
                *idx += 1;
            }
        }
    }

    for func_id in &func_ids {
        let func = module.funcs.get(*func_id);
        let FunctionKind::Local(local_func) = &func.kind else {
            continue;
        };

        let mut collector = Collector::default();
        dfs_in_order(&mut collector, local_func, local_func.entry_block());
        for (loc_data, seq, idx) in collector.pairs {
            instrloc_to_site.insert((*func_id, loc_data), (seq, idx));
        }

        let mut first_abs: Option<u64> = None;
        for (off, loc_id) in &local_func.instruction_mapping {
            let abs = *off as u64;
            first_abs.get_or_insert(abs);
            abs_offsets.push((abs, *func_id, loc_id.data()));
            if let Some(base) = first_abs {
                rel_offsets.push((abs.saturating_sub(base), *func_id, loc_id.data()));
            }
        }
    }

    abs_offsets.sort_by_key(|(off, _, _)| *off);
    rel_offsets.sort_by_key(|(off, _, _)| *off);

    let resolve_site = |addr: u64,
                        table: &[(u64, FunctionId, u32)],
                        instrloc_to_site: &HashMap<(FunctionId, u32), (InstrSeqId, usize)>|
     -> Option<(FunctionId, InstrSeqId, usize)> {
        let pos = table.partition_point(|(off, _, _)| *off <= addr);
        if pos == 0 {
            return None;
        }
        let (_, func_id, loc_data) = table[pos - 1];
        instrloc_to_site
            .get(&(func_id, loc_data))
            .copied()
            .map(|(seq, idx)| (func_id, seq, idx))
    };

    #[derive(Clone)]
    struct Resolved {
        location: LocationInfo,
        func_id: FunctionId,
        seq_id: InstrSeqId,
        instr_idx: usize,
    }

    let mut resolved: Vec<Resolved> = Vec::new();
    let mut seen_sites: HashSet<(FunctionId, InstrSeqId, usize)> = HashSet::new();
    let mut unmatched_addresses = 0usize;

    for loc in candidates {
        let maybe_site = resolve_site(loc.address, &abs_offsets, &instrloc_to_site)
            .or_else(|| resolve_site(loc.address, &rel_offsets, &instrloc_to_site));
        if let Some((func_id, seq_id, instr_idx)) = maybe_site {
            if seen_sites.insert((func_id, seq_id, instr_idx)) {
                resolved.push(Resolved {
                    location: loc,
                    func_id,
                    seq_id,
                    instr_idx,
                });
            }
        } else {
            unmatched_addresses += 1;
        }
    }

    resolved.sort_by_key(|r| (r.location.file, r.location.line, r.location.col, r.location.address));

    let inserted_locations: Vec<LocationInfo> = resolved.iter().map(|r| r.location.clone()).collect();

    // Build insertion plan from resolved sites with final 1-based bkpt indices.
    let mut plan: HashMap<FunctionId, HashMap<InstrSeqId, Vec<(usize, u32)>>> = HashMap::new();
    for (i, item) in resolved.iter().enumerate() {
        let bkpt = (i + 1) as u32;
        plan.entry(item.func_id)
            .or_default()
            .entry(item.seq_id)
            .or_default()
            .push((item.instr_idx, bkpt));
    }

    let mut inserted_breakpoints: BTreeSet<u32> = BTreeSet::new();
    let mut skipped_out_of_bounds = 0usize;

    for func_id in func_ids {
        let Some(by_seq) = plan.remove(&func_id) else {
            continue;
        };
        let func = module.funcs.get_mut(func_id);
        let FunctionKind::Local(local_func) = &mut func.kind else {
            continue;
        };
        let builder = local_func.builder_mut();

        for (seq_id, mut inserts) in by_seq {
            inserts.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

            let mut seq = builder.instr_seq(seq_id);
            let instrs = seq.instrs_mut();
            for (idx, bkpt) in inserts {
                if idx > instrs.len() {
                    skipped_out_of_bounds += 1;
                    continue;
                }
                instrs.insert(
                    idx,
                    (
                        Instr::Const(Const {
                            value: Value::I32(bkpt as i32),
                        }),
                        Default::default(),
                    ),
                );
                instrs.insert(
                    idx + 1,
                    (Instr::Call(Call { func: bkpt_func_id }), Default::default()),
                );
                inserted_breakpoints.insert(bkpt);
            }
        }
    }

    // Build file-local line ranges that resolve to each inserted bkpt index.
    let mut per_file_lines: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new(); // file -> [(line, bkpt)]
    for (i, loc) in inserted_locations.iter().enumerate() {
        let bkpt = (i + 1) as u32;
        per_file_lines
            .entry(loc.file)
            .or_default()
            .push((loc.line, bkpt));
    }

    let mut ranges = Vec::new();
    for (file, mut entries) in per_file_lines {
        entries.sort_by_key(|(line, _)| *line);
        entries.dedup_by_key(|(line, _)| *line);
        for i in 0..entries.len() {
            let (start_line, bkpt) = entries[i];
            let end_line = if i + 1 < entries.len() {
                let next_start = entries[i + 1].0;
                if next_start > start_line {
                    next_start - 1
                } else {
                    start_line
                }
            } else {
                start_line
            };
            ranges.push(BreakpointRange {
                file,
                start_line,
                end_line,
                bkpt,
            });
        }
    }

    web_sys::console::log_1(
        &format!(
            "instrumentation summary: candidates={}, inserted={}, unmatched_dwarf_addresses={}, skipped_out_of_bounds={}",
            inserted_locations.len() + unmatched_addresses,
            inserted_breakpoints.len(),
            unmatched_addresses,
            skipped_out_of_bounds
        )
        .into(),
    );

    Ok((module.emit_wasm(), inserted_locations, ranges))
}
