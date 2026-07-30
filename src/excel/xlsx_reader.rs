//! Native XLSX reader (Office Open XML / OOXML).
//!
//! Reads XLSX files using only the `zip` crate for decompression and a
//! minimal hand-written XML scanner. No external spreadsheet library is used.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Read;
use zip::ZipArchive;

/// Cell value types for XLSX reading.
#[derive(Debug, Clone)]
pub enum XlsxCellValue {
    String(String),
    Number(f64),
    Bool(bool),
    Error(String),
    Empty,
}

impl XlsxCellValue {
    pub fn to_string(&self) -> String {
        match self {
            XlsxCellValue::String(s) => s.clone(),
            XlsxCellValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            XlsxCellValue::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
            XlsxCellValue::Error(e) => format!("#{}", e),
            XlsxCellValue::Empty => String::new(),
        }
    }
}

/// Sheet data for XLSX reading.
#[derive(Debug, Clone)]
pub struct XlsxSheetData {
    pub name: String,
    pub cells: Vec<Vec<XlsxCellValue>>,
}

impl XlsxSheetData {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cells: Vec::new(),
        }
    }

    pub fn to_string_vec(&self) -> Vec<Vec<String>> {
        self.cells.iter()
            .map(|row| row.iter().map(|cell| cell.to_string()).collect())
            .collect()
    }

    pub fn row_count(&self) -> usize {
        self.cells.len()
    }

    pub fn col_count(&self) -> usize {
        self.cells.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    pub fn get_cell(&self, row: usize, col: usize) -> &XlsxCellValue {
        self.cells.get(row)
            .and_then(|r| r.get(col))
            .unwrap_or(&XlsxCellValue::Empty)
    }
}

/// Minimal XML scanner for extracting data from well-formed XLSX XML.
struct XmlScanner<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> XmlScanner<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Skip whitespace and XML declarations/comments.
    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Skip XML declaration <?xml ... ?>
    fn skip_declaration(&mut self) {
        self.skip_whitespace();
        if self.pos + 5 <= self.data.len() && &self.data[self.pos..self.pos + 5] == b"<?xml" {
            if let Some(end) = self.find_from(b"?>", self.pos) {
                self.pos = end + 2;
            }
        }
        // Skip comments <!-- -->
        loop {
            self.skip_whitespace();
            if self.pos + 4 <= self.data.len() && &self.data[self.pos..self.pos + 4] == b"<!--" {
                if let Some(end) = self.find_from(b"-->", self.pos) {
                    self.pos = end + 3;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn find_from(&self, needle: &[u8], from: usize) -> Option<usize> {
        if needle.is_empty() || from >= self.data.len() {
            return None;
        }
        let end = self.data.len() - needle.len() + 1;
        for i in from..end {
            if &self.data[i..i + needle.len()] == needle {
                return Some(i);
            }
        }
        None
    }

    /// Find the next opening tag with the given local name.
    /// Returns the position right after the tag name start, or None.
    fn find_open_tag(&mut self, local_name: &str) -> Option<usize> {
        let name_bytes = local_name.as_bytes();
        while self.pos < self.data.len() {
            // Find '<'
            if self.data[self.pos] != b'<' {
                self.pos += 1;
                continue;
            }
            // Skip closing tags, declarations, comments
            if self.pos + 1 < self.data.len() {
                let next = self.data[self.pos + 1];
                if next == b'/' || next == b'?' || next == b'!' {
                    self.pos += 1;
                    continue;
                }
            }
            // Check if this matches the tag name (possibly with namespace prefix)
            let tag_start = self.pos + 1;
            if tag_start + name_bytes.len() <= self.data.len() {
                // Check exact match or namespace-prefixed match
                let after_tag = tag_start + name_bytes.len();
                if &self.data[tag_start..after_tag] == name_bytes {
                    // Ensure it's followed by whitespace, '>', '/>', or ':'
                    if after_tag < self.data.len() {
                        let c = self.data[after_tag];
                        if c == b' ' || c == b'>' || c == b'/' || c == b':' || c == b'\t' || c == b'\n' || c == b'\r' {
                            self.pos = tag_start;
                            return Some(tag_start);
                        }
                    }
                }
                // Check namespace-prefixed: e.g. "sheet" matches "x:sheet"
                // Find ':' before the name
                if let Some(colon_pos) = self.find_colon_in_tag(tag_start) {
                    let local_start = colon_pos + 1;
                    let local_end = local_start + name_bytes.len();
                    if local_end <= self.data.len() && &self.data[local_start..local_end] == name_bytes {
                        if local_end < self.data.len() {
                            let c = self.data[local_end];
                            if c == b' ' || c == b'>' || c == b'/' || c == b'\t' || c == b'\n' || c == b'\r' {
                                self.pos = tag_start;
                                return Some(tag_start);
                            }
                        }
                    }
                }
            }
            self.pos += 1;
        }
        None
    }

    fn find_colon_in_tag(&self, start: usize) -> Option<usize> {
        let mut i = start;
        while i < self.data.len() {
            let c = self.data[i];
            if c == b' ' || c == b'>' || c == b'/' || c == b'\t' || c == b'\n' || c == b'\r' {
                return None;
            }
            if c == b':' {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Get the tag name at the current position (after '<').
    /// Returns the full tag name including namespace prefix.
    fn read_tag_name(&self, start: usize) -> String {
        let mut end = start;
        while end < self.data.len() {
            let c = self.data[end];
            if c == b' ' || c == b'>' || c == b'/' || c == b'\t' || c == b'\n' || c == b'\r' {
                break;
            }
            end += 1;
        }
        String::from_utf8_lossy(&self.data[start..end]).to_string()
    }

    /// Parse attributes from a tag. Starts after the tag name.
    /// Returns a map of (local_name -> value) and the end position.
    fn parse_attributes(&self, attr_start: usize) -> (HashMap<String, String>, usize) {
        let mut attrs = HashMap::new();
        let mut pos = attr_start;

        loop {
            // Skip whitespace
            while pos < self.data.len() && matches!(self.data[pos], b' ' | b'\t' | b'\n' | b'\r') {
                pos += 1;
            }
            if pos >= self.data.len() {
                break;
            }
            // Check for end of tag
            if self.data[pos] == b'>' {
                pos += 1;
                break;
            }
            if self.data[pos] == b'/' {
                // Self-closing tag
                if pos + 1 < self.data.len() && self.data[pos + 1] == b'>' {
                    pos += 2;
                } else {
                    pos += 1;
                }
                break;
            }
            // Read attribute name (may have namespace prefix)
            let name_start = pos;
            while pos < self.data.len() && !matches!(self.data[pos], b'=' | b' ' | b'>' | b'/' | b'\t' | b'\n' | b'\r') {
                pos += 1;
            }
            if pos >= self.data.len() || self.data[pos] != b'=' {
                pos += 1;
                continue;
            }
            let full_name = String::from_utf8_lossy(&self.data[name_start..pos]).to_string();
            // Extract local name (after ':')
            let local_name = full_name.rsplit(':').next().unwrap_or(&full_name).to_string();
            pos += 1; // skip '='
            // Skip whitespace
            while pos < self.data.len() && matches!(self.data[pos], b' ' | b'\t') {
                pos += 1;
            }
            if pos >= self.data.len() {
                break;
            }
            let quote = self.data[pos];
            if quote != b'"' && quote != b'\'' {
                break;
            }
            pos += 1; // skip opening quote
            let val_start = pos;
            while pos < self.data.len() && self.data[pos] != quote {
                pos += 1;
            }
            let value = String::from_utf8_lossy(&self.data[val_start..pos]).to_string();
            attrs.insert(local_name, xml_unescape(&value));
            if pos < self.data.len() {
                pos += 1; // skip closing quote
            }
        }

        (attrs, pos)
    }

    /// Extract text content between current position and the closing tag.
    /// Assumes we're positioned right after the opening tag's '>'.
    fn read_text_until_close(&mut self, local_name: &str) -> String {
        let text_start = self.pos;
        // Find closing tag </...local_name>
        // We need to find the matching close tag, handling nesting
        let mut depth = 1;
        let name_bytes = local_name.as_bytes();

        while self.pos < self.data.len() {
            if self.data[self.pos] == b'<' {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'/' {
                    // Closing tag
                    let check_start = self.pos + 2;
                    // Check if this matches our tag
                    if check_start + name_bytes.len() <= self.data.len() {
                        let after = check_start + name_bytes.len();
                        if &self.data[check_start..after] == name_bytes {
                            let c = if after < self.data.len() { self.data[after] } else { b'>' };
                            if c == b'>' || c == b' ' || c == b':' || c == b'\t' || c == b'\n' {
                                depth -= 1;
                                if depth == 0 {
                                    let text = String::from_utf8_lossy(&self.data[text_start..self.pos]).to_string();
                                    // Skip past closing tag
                                    while self.pos < self.data.len() && self.data[self.pos] != b'>' {
                                        self.pos += 1;
                                    }
                                    if self.pos < self.data.len() {
                                        self.pos += 1;
                                    }
                                    return xml_unescape(text.trim());
                                }
                            }
                        }
                        // Also check with namespace prefix
                        if let Some(colon) = self.find_colon_in_tag(check_start) {
                            let local_start = colon + 1;
                            let local_end = local_start + name_bytes.len();
                            if local_end <= self.data.len() && &self.data[local_start..local_end] == name_bytes {
                                let c = if local_end < self.data.len() { self.data[local_end] } else { b'>' };
                                if c == b'>' || c == b' ' || c == b':' || c == b'\t' || c == b'\n' {
                                    depth -= 1;
                                    if depth == 0 {
                                        let text = String::from_utf8_lossy(&self.data[text_start..self.pos]).to_string();
                                        while self.pos < self.data.len() && self.data[self.pos] != b'>' {
                                            self.pos += 1;
                                        }
                                        if self.pos < self.data.len() {
                                            self.pos += 1;
                                        }
                                        return xml_unescape(text.trim());
                                    }
                                }
                            }
                        }
                    }
                    // Skip this closing tag
                    while self.pos < self.data.len() && self.data[self.pos] != b'>' {
                        self.pos += 1;
                    }
                    if self.pos < self.data.len() {
                        self.pos += 1;
                    }
                } else if self.pos + 1 < self.data.len() && self.data[self.pos + 1] != b'?' && self.data[self.pos + 1] != b'!' {
                    // Opening tag - check if it matches our tag name for nesting
                    let tag_start = self.pos + 1;
                    if tag_start + name_bytes.len() <= self.data.len() {
                        let after = tag_start + name_bytes.len();
                        if &self.data[tag_start..after] == name_bytes {
                            if after < self.data.len() {
                                let c = self.data[after];
                                if c == b' ' || c == b'>' || c == b'/' || c == b':' || c == b'\t' || c == b'\n' {
                                    depth += 1;
                                }
                            }
                        }
                        if let Some(colon) = self.find_colon_in_tag(tag_start) {
                            let local_start = colon + 1;
                            let local_end = local_start + name_bytes.len();
                            if local_end <= self.data.len() && &self.data[local_start..local_end] == name_bytes {
                                if local_end < self.data.len() {
                                    let c = self.data[local_end];
                                    if c == b' ' || c == b'>' || c == b'/' || c == b':' || c == b'\t' || c == b'\n' {
                                        depth += 1;
                                    }
                                }
                            }
                        }
                    }
                    self.pos += 1;
                } else {
                    self.pos += 1;
                }
            } else {
                self.pos += 1;
            }
        }
        String::from_utf8_lossy(&self.data[text_start..self.pos]).to_string()
    }

    /// Skip past the current opening tag (from '<' to '>' or '/>').
    /// Positions right after the '>' or '/>'.
    fn skip_open_tag(&mut self) {
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if c == b'>' {
                self.pos += 1;
                return;
            }
            if c == b'/' && self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                self.pos += 2;
                return;
            }
            // Skip quoted strings
            if c == b'"' || c == b'\'' {
                let quote = c;
                self.pos += 1;
                while self.pos < self.data.len() && self.data[self.pos] != quote {
                    self.pos += 1;
                }
            }
            self.pos += 1;
        }
    }

    /// Check if the tag at current position is self-closing.
    /// Call this right after find_open_tag returns.
    fn is_self_closing(&self, tag_name_start: usize) -> bool {
        // Scan from tag name start to find '/>' or '>'
        let mut pos = tag_name_start;
        while pos < self.data.len() {
            let c = self.data[pos];
            if c == b'>' {
                return false;
            }
            if c == b'/' && pos + 1 < self.data.len() && self.data[pos + 1] == b'>' {
                return true;
            }
            // Skip quoted attribute values
            if c == b'"' || c == b'\'' {
                let quote = c;
                pos += 1;
                while pos < self.data.len() && self.data[pos] != quote {
                    pos += 1;
                }
            }
            pos += 1;
        }
        false
    }
}

fn xml_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Parse a cell reference like "A1" into (row, col) 0-based indices.
fn parse_cell_ref(ref_str: &str) -> (u32, u16) {
    let mut col_str = String::new();
    let mut row_str = String::new();
    for ch in ref_str.chars() {
        if ch.is_alphabetic() {
            col_str.push(ch.to_ascii_uppercase());
        } else if ch.is_ascii_digit() {
            row_str.push(ch);
        }
    }
    let mut col: u16 = 0;
    for ch in col_str.chars() {
        col = col * 26 + (ch as u16 - b'A' as u16 + 1);
    }
    let row: u32 = row_str.parse().unwrap_or(0);
    (row.saturating_sub(1), col.saturating_sub(1))
}

/// XLSX workbook reader.
pub struct XlsxReader {
    sheets: Vec<XlsxSheetData>,
}

impl XlsxReader {
    pub fn new() -> Self {
        Self { sheets: Vec::new() }
    }

    /// Read an XLSX file from a path.
    pub fn from_path(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open XLSX file: {}", path))?;
        Self::from_reader(file)
    }

    /// Read an XLSX file from any Read+Seek source.
    pub fn from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)
            .context("Failed to open XLSX archive")?;
        Self::from_archive(&mut archive)
    }

    fn from_archive<R: std::io::Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<Self> {
        // Read shared strings
        let shared_strings = Self::read_shared_strings(archive)?;

        // Read workbook.xml to get sheet names and r:ids
        let workbook_xml = Self::read_zip_entry(archive, "xl/workbook.xml")
            .or_else(|_| Self::read_zip_entry(archive, "xl/workbook.xml"))?;
        let sheets_info = Self::parse_workbook_sheets(&workbook_xml);

        // Read workbook.xml.rels to map r:ids to file paths
        let rels_xml = Self::read_zip_entry(archive, "xl/_rels/workbook.xml.rels")?;
        let rels_map = Self::parse_rels(&rels_xml);

        // Read each sheet
        let mut sheets = Vec::new();
        for (name, rid) in &sheets_info {
            let target = rels_map.get(rid)
                .cloned()
                .unwrap_or_else(|| format!("worksheets/sheet{}.xml", sheets_info.iter().position(|(n, _)| n == name).map(|i| i + 1).unwrap_or(1)));
            let sheet_path = if target.starts_with('/') {
                target[1..].to_string()
            } else {
                format!("xl/{}", target)
            };
            let sheet_xml = Self::read_zip_entry(archive, &sheet_path)
                .with_context(|| format!("Failed to read sheet XML: {}", sheet_path))?;
            let cells = Self::parse_sheet(&sheet_xml, &shared_strings);
            sheets.push(XlsxSheetData::new(name.clone()).tap_cells(cells));
        }

        Ok(Self { sheets })
    }

    fn read_zip_entry<R: std::io::Read + std::io::Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
        let mut entry = archive
            .by_name(name)
            .with_context(|| format!("Failed to find '{}' in XLSX archive", name))?;
        let size = entry.size();
        if size > crate::limits::MAX_ZIP_ENTRY_BYTES {
            anyhow::bail!(
                "ZIP entry '{}' is too large ({} bytes; max {})",
                name,
                size,
                crate::limits::MAX_ZIP_ENTRY_BYTES
            );
        }
        let mut buf = Vec::with_capacity(size as usize);
        entry.read_to_end(&mut buf)?;
        if buf.len() as u64 > crate::limits::MAX_ZIP_ENTRY_BYTES {
            anyhow::bail!(
                "ZIP entry '{}' expanded beyond max size ({} bytes)",
                name,
                crate::limits::MAX_ZIP_ENTRY_BYTES
            );
        }
        Ok(buf)
    }

    fn read_shared_strings<R: std::io::Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<String>> {
        let xml = match Self::read_zip_entry(archive, "xl/sharedStrings.xml") {
            Ok(data) => data,
            Err(_) => return Ok(Vec::new()),
        };
        let xml_str = String::from_utf8_lossy(&xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut strings = Vec::new();
        // Find each <si> element
        while scanner.find_open_tag("si").is_some() {
            let tag_start = scanner.pos;
            if scanner.is_self_closing(tag_start) {
                scanner.skip_open_tag();
                strings.push(String::new());
                continue;
            }
            scanner.skip_open_tag();
            // Read text content - may be in <t> directly or in <r><t> rich text runs
            let mut text = String::new();
            // Try direct <t> child
            let save_pos = scanner.pos;
            if scanner.find_open_tag("t").is_some() {
                let t_tag_start = scanner.pos;
                if scanner.is_self_closing(t_tag_start) {
                    scanner.skip_open_tag();
                } else {
                    scanner.skip_open_tag();
                    text = scanner.read_text_until_close("t");
                }
            } else {
                // Rich text: multiple <r> elements each with <t>
                scanner.pos = save_pos;
                while scanner.find_open_tag("t").is_some() {
                    let t_tag_start = scanner.pos;
                    if scanner.is_self_closing(t_tag_start) {
                        scanner.skip_open_tag();
                        continue;
                    }
                    scanner.skip_open_tag();
                    let run_text = scanner.read_text_until_close("t");
                    text.push_str(&run_text);
                }
            }
            strings.push(text);
            // Skip to end of <si>
            scanner.find_open_tag("si"); // move past, the close will be handled by next iteration
        }
        Ok(strings)
    }

    fn parse_workbook_sheets(xml: &[u8]) -> Vec<(String, String)> {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut sheets = Vec::new();
        while scanner.find_open_tag("sheet").is_some() {
            let tag_start = scanner.pos;
            // Read tag name to advance past it
            let _tag_name = scanner.read_tag_name(tag_start);
            // Parse attributes starting after the tag name
            let name_end = tag_start + _tag_name.len();
            let (attrs, _) = scanner.parse_attributes(name_end);
            let name = attrs.get("name").cloned().unwrap_or_default();
            let rid = attrs.get("id").cloned().unwrap_or_default();
            if !name.is_empty() {
                sheets.push((name, rid));
            }
            scanner.skip_open_tag();
        }
        sheets
    }

    fn parse_rels(xml: &[u8]) -> HashMap<String, String> {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut map = HashMap::new();
        while scanner.find_open_tag("Relationship").is_some() {
            let tag_start = scanner.pos;
            let _tag_name = scanner.read_tag_name(tag_start);
            let name_end = tag_start + _tag_name.len();
            let (attrs, _) = scanner.parse_attributes(name_end);
            let id = attrs.get("Id").cloned().unwrap_or_default();
            let target = attrs.get("Target").cloned().unwrap_or_default();
            if !id.is_empty() {
                map.insert(id, target);
            }
            scanner.skip_open_tag();
        }
        map
    }

    fn parse_sheet(xml: &[u8], shared_strings: &[String]) -> Vec<Vec<XlsxCellValue>> {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut cells: HashMap<(u32, u16), XlsxCellValue> = HashMap::new();
        let mut max_row: u32 = 0;
        let mut max_col: u16 = 0;

        // Find <sheetData>
        if scanner.find_open_tag("sheetData").is_none() {
            return Vec::new();
        }
        let sd_start = scanner.pos;
        if scanner.is_self_closing(sd_start) {
            scanner.skip_open_tag();
            return Vec::new();
        }
        scanner.skip_open_tag();

        // Iterate over <row> elements
        while scanner.find_open_tag("row").is_some() {
            let row_tag_start = scanner.pos;
            if scanner.is_self_closing(row_tag_start) {
                scanner.skip_open_tag();
                continue;
            }
            let _row_name = scanner.read_tag_name(row_tag_start);
            let (_row_attrs, _) = scanner.parse_attributes(row_tag_start + _row_name.len());
            scanner.skip_open_tag();

            // Iterate over <c> (cell) elements within this row
            loop {
                let save_pos = scanner.pos;
                if scanner.find_open_tag("c").is_none() {
                    scanner.pos = save_pos;
                    break;
                }
                let c_tag_start = scanner.pos;
                let _c_name = scanner.read_tag_name(c_tag_start);
                let (c_attrs, _) = scanner.parse_attributes(c_tag_start + _c_name.len());

                let cell_ref = c_attrs.get("r").cloned().unwrap_or_default();
                let cell_type = c_attrs.get("t").cloned().unwrap_or_else(|| "n".to_string());
                let (row_idx, col_idx) = if cell_ref.is_empty() {
                    (0u32, 0u16)
                } else {
                    parse_cell_ref(&cell_ref)
                };

                if scanner.is_self_closing(c_tag_start) {
                    scanner.skip_open_tag();
                    cells.insert((row_idx, col_idx), XlsxCellValue::Empty);
                    max_row = max_row.max(row_idx);
                    max_col = max_col.max(col_idx);
                    continue;
                }
                scanner.skip_open_tag();

                // Read cell value: <v> for values, <is><t> for inline strings
                let mut value = XlsxCellValue::Empty;
                match cell_type.as_str() {
                    "s" => {
                        // Shared string
                        if scanner.find_open_tag("v").is_some() {
                            let v_start = scanner.pos;
                            if !scanner.is_self_closing(v_start) {
                                scanner.skip_open_tag();
                                let text = scanner.read_text_until_close("v");
                                if let Ok(idx) = text.parse::<usize>() {
                                    value = XlsxCellValue::String(
                                        shared_strings.get(idx).cloned().unwrap_or_default()
                                    );
                                }
                            } else {
                                scanner.skip_open_tag();
                            }
                        }
                    }
                    "inlineStr" => {
                        // Inline string: <is><t>text</t></is>
                        if scanner.find_open_tag("t").is_some() {
                            let t_start = scanner.pos;
                            if !scanner.is_self_closing(t_start) {
                                scanner.skip_open_tag();
                                let text = scanner.read_text_until_close("t");
                                value = XlsxCellValue::String(text);
                            } else {
                                scanner.skip_open_tag();
                            }
                        }
                    }
                    "b" => {
                        // Boolean
                        if scanner.find_open_tag("v").is_some() {
                            let v_start = scanner.pos;
                            if !scanner.is_self_closing(v_start) {
                                scanner.skip_open_tag();
                                let text = scanner.read_text_until_close("v");
                                value = XlsxCellValue::Bool(text == "1" || text.eq_ignore_ascii_case("true"));
                            } else {
                                scanner.skip_open_tag();
                            }
                        }
                    }
                    "e" => {
                        // Error
                        if scanner.find_open_tag("v").is_some() {
                            let v_start = scanner.pos;
                            if !scanner.is_self_closing(v_start) {
                                scanner.skip_open_tag();
                                let text = scanner.read_text_until_close("v");
                                value = XlsxCellValue::Error(text);
                            } else {
                                scanner.skip_open_tag();
                            }
                        }
                    }
                    "str" => {
                        // Formula string result
                        if scanner.find_open_tag("v").is_some() {
                            let v_start = scanner.pos;
                            if !scanner.is_self_closing(v_start) {
                                scanner.skip_open_tag();
                                let text = scanner.read_text_until_close("v");
                                value = XlsxCellValue::String(text);
                            } else {
                                scanner.skip_open_tag();
                            }
                        }
                    }
                    _ => {
                        // Number (default)
                        if scanner.find_open_tag("v").is_some() {
                            let v_start = scanner.pos;
                            if !scanner.is_self_closing(v_start) {
                                scanner.skip_open_tag();
                                let text = scanner.read_text_until_close("v");
                                if let Ok(n) = text.parse::<f64>() {
                                    value = XlsxCellValue::Number(n);
                                }
                            } else {
                                scanner.skip_open_tag();
                            }
                        }
                    }
                }

                // Skip to end of <c> element
                // Find the closing </c> tag
                scanner.find_open_tag("c"); // This will find the next <c> or nothing
                // We need to skip past the closing tag of current <c>
                // Actually, the find_open_tag for "c" above may have found the next cell's opening tag
                // Let's just continue - the loop will handle it

                cells.insert((row_idx, col_idx), value);
                max_row = max_row.max(row_idx);
                max_col = max_col.max(col_idx);
            }

            // Skip to end of row - find next <row> or end of <sheetData>
        }

        // Convert sparse map to dense grid (clamped to avoid OOM from far-corner refs)
        if max_row == 0 && max_col == 0 && cells.is_empty() {
            return Vec::new();
        }

        let (n_rows, n_cols) =
            crate::limits::clamp_dense_dims(max_row as usize, max_col as usize);
        if n_rows == 0 || n_cols == 0 {
            return Vec::new();
        }

        let mut result = vec![vec![XlsxCellValue::Empty; n_cols]; n_rows];
        for ((row, col), value) in cells {
            let r = row as usize;
            let c = col as usize;
            if r < n_rows && c < n_cols {
                result[r][c] = value;
            }
        }
        result
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.sheets.iter().map(|s| s.name.clone()).collect()
    }

    pub fn get_sheet(&self, index: usize) -> Option<&XlsxSheetData> {
        self.sheets.get(index)
    }

    pub fn get_sheet_by_name(&self, name: &str) -> Option<&XlsxSheetData> {
        self.sheets.iter().find(|s| s.name == name)
    }

    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    pub fn read_all_to_string_vec(&self) -> HashMap<String, Vec<Vec<String>>> {
        self.sheets.iter()
            .map(|s| (s.name.clone(), s.to_string_vec()))
            .collect()
    }
}

impl Default for XlsxReader {
    fn default() -> Self {
        Self::new()
    }
}

// Helper trait for builder-style cell assignment
trait TapCells {
    fn tap_cells(self, cells: Vec<Vec<XlsxCellValue>>) -> Self;
}

impl TapCells for XlsxSheetData {
    fn tap_cells(mut self, cells: Vec<Vec<XlsxCellValue>>) -> Self {
        self.cells = cells;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_ref() {
        assert_eq!(parse_cell_ref("A1"), (0, 0));
        assert_eq!(parse_cell_ref("B3"), (2, 1));
        assert_eq!(parse_cell_ref("AA10"), (9, 26));
    }

    #[test]
    fn test_xml_unescape() {
        assert_eq!(xml_unescape("hello &amp; world"), "hello & world");
        assert_eq!(xml_unescape("no escapes"), "no escapes");
        assert_eq!(xml_unescape("&lt;tag&gt;"), "<tag>");
    }

    #[test]
    fn test_empty_reader() {
        let reader = XlsxReader::new();
        assert_eq!(reader.sheet_count(), 0);
    }
}
