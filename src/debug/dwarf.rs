use ::serde::{Deserialize, Serialize};
use anyhow::Result;
use gimli::{DwarfSections, EndianRcSlice, LittleEndian, SectionId};
use std::collections::HashMap;
use std::rc::Rc;
use wasmparser::{Parser, Payload};

/// The reader type we use any time we interface with `gimli`.
type Reader = EndianRcSlice<LittleEndian>;

#[derive(Debug)]
pub struct Dwarf {
    /// Provides direct access to `gimli`
    inner: gimli::Dwarf<Reader>,
    /// DWARF sections maintained for serialization.
    /// References same memory as `inner`.
    sections: gimli::DwarfSections<Reader>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DieReference {
    unit_index: usize,
    #[serde(with = "serde::unit_offset")]
    unit_ofs: gimli::UnitOffset,
}

impl Clone for Dwarf {
    fn clone(&self) -> Self {
        let sections = gimli::DwarfSections {
            debug_abbrev: self.sections.debug_abbrev.clone(),
            debug_addr: self.sections.debug_addr.clone(),
            debug_aranges: self.sections.debug_aranges.clone(),
            debug_info: self.sections.debug_info.clone(),
            debug_line: self.sections.debug_line.clone(),
            debug_line_str: self.sections.debug_line_str.clone(),
            debug_macinfo: self.sections.debug_macinfo.clone(),
            debug_macro: self.sections.debug_macro.clone(),
            debug_names: self.sections.debug_names.clone(),
            debug_str: self.sections.debug_str.clone(),
            debug_str_offsets: self.sections.debug_str_offsets.clone(),
            debug_types: self.sections.debug_types.clone(),
            debug_loc: self.sections.debug_loc.clone(),
            debug_loclists: self.sections.debug_loclists.clone(),
            debug_ranges: self.sections.debug_ranges.clone(),
            debug_rnglists: self.sections.debug_rnglists.clone(),
        };

        let inner = sections.borrow(|s| s.clone());

        Self { inner, sections }
    }
}

impl Dwarf {
    /// Load Dwarf from section map
    pub fn from_sections(sections: &HashMap<&str, &[u8]>) -> Result<Self> {
        let sections = DwarfSections::load(|id: SectionId| -> Result<Reader, gimli::Error> {
            let data = sections.get(id.name()).copied().unwrap_or(&[]);
            Ok(EndianRcSlice::new(Rc::from(data), LittleEndian))
        })?;
        let inner = sections.borrow(|section| section.clone());
        Ok(Self { inner, sections })
    }
}

pub mod serde {
    use std::collections::HashMap;

    use super::{Dwarf, Reader};
    use gimli::Section;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    fn insert<S: Section<Reader>>(m: &mut HashMap<String, Vec<u8>>, section: &S) {
        let bytes = section.reader().bytes();
        if bytes.len() > 0 {
            m.insert(S::id().name().to_string(), bytes.to_vec());
        }
    }

    pub fn serialize<S>(dwarf: &Dwarf, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let sec = &dwarf.sections;
        let mut m = HashMap::new();
        insert(&mut m, &sec.debug_abbrev);
        insert(&mut m, &sec.debug_addr);
        insert(&mut m, &sec.debug_aranges);
        insert(&mut m, &sec.debug_info);
        insert(&mut m, &sec.debug_line);
        insert(&mut m, &sec.debug_line_str);
        insert(&mut m, &sec.debug_macinfo);
        insert(&mut m, &sec.debug_macro);
        insert(&mut m, &sec.debug_names);
        insert(&mut m, &sec.debug_str);
        insert(&mut m, &sec.debug_str_offsets);
        insert(&mut m, &sec.debug_types);
        insert(&mut m, &sec.debug_loc);
        insert(&mut m, &sec.debug_loclists);
        insert(&mut m, &sec.debug_ranges);
        insert(&mut m, &sec.debug_rnglists);
        m.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Dwarf, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: HashMap<String, Vec<u8>> = HashMap::deserialize(d)?;
        let map: HashMap<&str, &[u8]> = map
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        super::Dwarf::from_sections(&map).map_err(serde::de::Error::custom)
    }

    pub mod unit_offset {
        use gimli::UnitOffset;
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub fn serialize<S>(offset: &UnitOffset, s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            offset.0.serialize(s)
        }

        pub fn deserialize<'de, D>(d: D) -> Result<UnitOffset, D::Error>
        where
            D: Deserializer<'de>,
        {
            let n = usize::deserialize(d)?;
            Ok(UnitOffset(n))
        }
    }
}
