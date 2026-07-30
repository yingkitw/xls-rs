//! Native ODS reader (OpenDocument Spreadsheet).
//!
//! Reads ODS files using only the `zip` crate for decompression and a
//! minimal hand-written XML scanner. No external spreadsheet library is used.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Read;
use zip::ZipArchive;

use super::xlsx_reader::XlsxCellValue;

/// Sheet data for ODS reading.
#[derive(Debug, Clone)]
pub struct OdsSheetData {
    pub name: String,
    pub cells: Vec<Vec<XlsxCellValue>>,
}

impl OdsSheetData {
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
}

/// Minimal XML scanner for ODS (reuses the same approach as xlsx_reader).
struct OdsXmlScanner<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> OdsXmlScanner<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn skip_declaration(&mut self) {
        self.skip_whitespace();
        if self.pos + 5 <= self.data.len() && &self.data[self.pos..self.pos + 5] == b"<?xml" {
            if let Some(end) = self.find_from(b"?>", self.pos) {
                self.pos = end + 2;
            }
        }
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

    /// Find the next opening tag with the given local name (ignoring namespace prefix).
    fn find_open_tag(&mut self, local_name: &str) -> Option<usize> {
        let name_bytes = local_name.as_bytes();
        while self.pos < self.data.len() {
            if self.data[self.pos] != b'<' {
                self.pos += 1;
                continue;
            }
            if self.pos + 1 < self.data.len() {
                let next = self.data[self.pos + 1];
                if next == b'/' || next == b'?' || next == b'!' {
                    self.pos += 1;
                    continue;
                }
            }
            let tag_start = self.pos + 1;
            if tag_start + name_bytes.len() <= self.data.len() {
                let after_tag = tag_start + name_bytes.len();
                if &self.data[tag_start..after_tag] == name_bytes {
                    if after_tag < self.data.len() {
                        let c = self.data[after_tag];
                        if c == b' ' || c == b'>' || c == b'/' || c == b':' || c == b'\t' || c == b'\n' || c == b'\r' {
                            self.pos = tag_start;
                            return Some(tag_start);
                        }
                    }
                }
                if let Some(colon_pos) = self.find_colon_in_tag(tag_start) {
                    let local_start = colon_pos + 1;
                    let local_end = local_start + name_bytes.len();
                    if local_end <= self.data.len() && &self.data[local_start..local_end] == name_bytes {
                        if local_end < self.data.len() {
                            let c = self.data[local_end];
                            if c == b' ' || c == b'>' || c == b'/' || c == b':' || c == b'\t' || c == b'\n' || c == b'\r' {
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

    fn parse_attributes(&self, attr_start: usize) -> (HashMap<String, String>, usize) {
        let mut attrs = HashMap::new();
        let mut pos = attr_start;

        loop {
            while pos < self.data.len() && matches!(self.data[pos], b' ' | b'\t' | b'\n' | b'\r') {
                pos += 1;
            }
            if pos >= self.data.len() {
                break;
            }
            if self.data[pos] == b'>' {
                pos += 1;
                break;
            }
            if self.data[pos] == b'/' {
                if pos + 1 < self.data.len() && self.data[pos + 1] == b'>' {
                    pos += 2;
                } else {
                    pos += 1;
                }
                break;
            }
            let name_start = pos;
            while pos < self.data.len() && !matches!(self.data[pos], b'=' | b' ' | b'>' | b'/' | b'\t' | b'\n' | b'\r') {
                pos += 1;
            }
            if pos >= self.data.len() || self.data[pos] != b'=' {
                pos += 1;
                continue;
            }
            let full_name = String::from_utf8_lossy(&self.data[name_start..pos]).to_string();
            let local_name = full_name.rsplit(':').next().unwrap_or(&full_name).to_string();
            pos += 1;
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
            pos += 1;
            let val_start = pos;
            while pos < self.data.len() && self.data[pos] != quote {
                pos += 1;
            }
            let value = String::from_utf8_lossy(&self.data[val_start..pos]).to_string();
            attrs.insert(local_name, xml_unescape(&value));
            if pos < self.data.len() {
                pos += 1;
            }
        }

        (attrs, pos)
    }

    fn read_text_until_close(&mut self, local_name: &str) -> String {
        let text_start = self.pos;
        let mut depth = 1;
        let name_bytes = local_name.as_bytes();

        while self.pos < self.data.len() {
            if self.data[self.pos] == b'<' {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'/' {
                    let check_start = self.pos + 2;
                    if check_start + name_bytes.len() <= self.data.len() {
                        let after = check_start + name_bytes.len();
                        if &self.data[check_start..after] == name_bytes {
                            let c = if after < self.data.len() { self.data[after] } else { b'>' };
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
                    while self.pos < self.data.len() && self.data[self.pos] != b'>' {
                        self.pos += 1;
                    }
                    if self.pos < self.data.len() {
                        self.pos += 1;
                    }
                } else if self.pos + 1 < self.data.len() && self.data[self.pos + 1] != b'?' && self.data[self.pos + 1] != b'!' {
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

    fn is_self_closing(&self, tag_name_start: usize) -> bool {
        let mut pos = tag_name_start;
        while pos < self.data.len() {
            let c = self.data[pos];
            if c == b'>' {
                return false;
            }
            if c == b'/' && pos + 1 < self.data.len() && self.data[pos + 1] == b'>' {
                return true;
            }
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

/// ODS workbook reader.
pub struct OdsReader {
    sheets: Vec<OdsSheetData>,
}

impl OdsReader {
    pub fn new() -> Self {
        Self { sheets: Vec::new() }
    }

    pub fn from_path(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open ODS file: {}", path))?;
        Self::from_reader(file)
    }

    pub fn from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)
            .context("Failed to open ODS archive")?;
        Self::from_archive(&mut archive)
    }

    fn from_archive<R: std::io::Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<Self> {
        let content_xml = Self::read_zip_entry(archive, "content.xml")?;
        let sheets = Self::parse_content_xml(&content_xml);
        Ok(Self { sheets })
    }

    fn read_zip_entry<R: std::io::Read + std::io::Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
        let mut entry = archive
            .by_name(name)
            .with_context(|| format!("Failed to find '{}' in ODS archive", name))?;
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

    fn parse_content_xml(xml: &[u8]) -> Vec<OdsSheetData> {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = OdsXmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut sheets = Vec::new();

        // Find <table:table> or just <table> elements (sheet definitions)
        while scanner.find_open_tag("table").is_some() {
            let tag_start = scanner.pos;
            let tag_name = scanner.read_tag_name(tag_start);
            let (attrs, _) = scanner.parse_attributes(tag_start + tag_name.len());

            // Only process table elements that have a name attribute (i.e., actual sheets)
            let name = attrs.get("name").cloned().unwrap_or_default();
            if name.is_empty() {
                scanner.skip_open_tag();
                continue;
            }

            if scanner.is_self_closing(tag_start) {
                scanner.skip_open_tag();
                sheets.push(OdsSheetData::new(name));
                continue;
            }
            scanner.skip_open_tag();

            // Parse rows and cells
            let mut rows: Vec<Vec<XlsxCellValue>> = Vec::new();
            let mut current_row: Vec<XlsxCellValue> = Vec::new();
            let _repeat_cols: usize = 1;

            loop {
                let save_pos = scanner.pos;
                if scanner.find_open_tag("table-row").is_some() {
                    // If we were collecting cells, finalize the current row
                    // (shouldn't happen mid-row, but handle gracefully)
                    if !current_row.is_empty() {
                        rows.push(std::mem::take(&mut current_row));
                    }

                    let row_tag_start = scanner.pos;
                    let row_name = scanner.read_tag_name(row_tag_start);
                    let (row_attrs, _) = scanner.parse_attributes(row_tag_start + row_name.len());
                    let row_repeat: usize = row_attrs
                        .get("number-rows-repeated")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1)
                        .min(crate::limits::MAX_ODS_ROW_REPEAT);

                    if scanner.is_self_closing(row_tag_start) {
                        scanner.skip_open_tag();
                        let n = crate::limits::capped_repeat(
                            row_repeat,
                            rows.len(),
                            crate::limits::MAX_SHEET_ROWS,
                        );
                        for _ in 0..n {
                            rows.push(Vec::new());
                        }
                        continue;
                    }
                    scanner.skip_open_tag();

                    // Parse cells in this row
                    let mut row_cells: Vec<XlsxCellValue> = Vec::new();
                    loop {
                        let cell_save = scanner.pos;
                        if scanner.find_open_tag("table-cell").is_none() {
                            scanner.pos = cell_save;
                            break;
                        }
                        let cell_tag_start = scanner.pos;
                        let cell_name = scanner.read_tag_name(cell_tag_start);
                        let (cell_attrs, _) = scanner.parse_attributes(cell_tag_start + cell_name.len());
                        let cell_repeat: usize = cell_attrs
                            .get("number-columns-repeated")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(1)
                            .min(crate::limits::MAX_ODS_CELL_REPEAT);

                        let value_type = cell_attrs.get("value-type").cloned().unwrap_or_default();

                        if scanner.is_self_closing(cell_tag_start) {
                            scanner.skip_open_tag();
                            let n = crate::limits::capped_repeat(
                                cell_repeat,
                                row_cells.len(),
                                crate::limits::MAX_SHEET_COLS,
                            );
                            for _ in 0..n {
                                row_cells.push(XlsxCellValue::Empty);
                            }
                            continue;
                        }
                        scanner.skip_open_tag();

                        // Read cell content from <text:p> child
                        let mut cell_value = XlsxCellValue::Empty;
                        match value_type.as_str() {
                            "float" | "currency" | "percentage" => {
                                // Value from attribute or text
                                if let Some(val) = cell_attrs.get("value") {
                                    if let Ok(n) = val.parse::<f64>() {
                                        cell_value = XlsxCellValue::Number(n);
                                    }
                                } else {
                                    // Try reading text content
                                    if scanner.find_open_tag("p").is_some() {
                                        let p_start = scanner.pos;
                                        if !scanner.is_self_closing(p_start) {
                                            scanner.skip_open_tag();
                                            let text = scanner.read_text_until_close("p");
                                            if let Ok(n) = text.parse::<f64>() {
                                                cell_value = XlsxCellValue::Number(n);
                                            } else {
                                                cell_value = XlsxCellValue::String(text);
                                            }
                                        } else {
                                            scanner.skip_open_tag();
                                        }
                                    }
                                }
                            }
                            "boolean" => {
                                if let Some(val) = cell_attrs.get("boolean-value") {
                                    cell_value = XlsxCellValue::Bool(
                                        val == "true" || val == "TRUE"
                                    );
                                }
                            }
                            "string" | "" => {
                                // Read text from <text:p>
                                let mut text = String::new();
                                while scanner.find_open_tag("p").is_some() {
                                    let p_start = scanner.pos;
                                    if scanner.is_self_closing(p_start) {
                                        scanner.skip_open_tag();
                                        continue;
                                    }
                                    scanner.skip_open_tag();
                                    let p_text = scanner.read_text_until_close("p");
                                    if !text.is_empty() {
                                        text.push('\n');
                                    }
                                    text.push_str(&p_text);
                                }
                                if !text.is_empty() {
                                    cell_value = XlsxCellValue::String(text);
                                }
                            }
                            _ => {
                                // Read text from <text:p>
                                if scanner.find_open_tag("p").is_some() {
                                    let p_start = scanner.pos;
                                    if !scanner.is_self_closing(p_start) {
                                        scanner.skip_open_tag();
                                        let text = scanner.read_text_until_close("p");
                                        cell_value = XlsxCellValue::String(text);
                                    } else {
                                        scanner.skip_open_tag();
                                    }
                                }
                            }
                        }

                        let n = crate::limits::capped_repeat(
                            cell_repeat,
                            row_cells.len(),
                            crate::limits::MAX_SHEET_COLS,
                        );
                        for _ in 0..n {
                            row_cells.push(cell_value.clone());
                        }

                        // Skip to end of cell
                        // Find closing </table-cell> or next <table-cell>
                    }

                    // Skip to end of row
                    // Find closing </table-row> or next element
                    // Move scanner past the row content
                    // We'll rely on the next find_open_tag("table-row") or find_open_tag("table")

                    let n = crate::limits::capped_repeat(
                        row_repeat,
                        rows.len(),
                        crate::limits::MAX_SHEET_ROWS,
                    );
                    for _ in 0..n {
                        rows.push(row_cells.clone());
                    }

                    // Don't restore position - continue from where we are
                    let _ = save_pos;
                } else if scanner.find_open_tag("table").is_some() {
                    // Found next table - back up so the outer loop can process it
                    scanner.pos = save_pos;
                    break;
                } else {
                    scanner.pos = save_pos;
                    break;
                }
            }

            // Trim trailing empty columns (ODS often has repeated empty cells)
            for row in &mut rows {
                while row.last().map(|c| matches!(c, XlsxCellValue::Empty)).unwrap_or(false) {
                    row.pop();
                }
            }

            sheets.push(OdsSheetData {
                name,
                cells: rows,
            });
        }

        sheets
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.sheets.iter().map(|s| s.name.clone()).collect()
    }

    pub fn get_sheet(&self, index: usize) -> Option<&OdsSheetData> {
        self.sheets.get(index)
    }

    pub fn get_sheet_by_name(&self, name: &str) -> Option<&OdsSheetData> {
        self.sheets.iter().find(|s| s.name == name)
    }

    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }
}

impl Default for OdsReader {
    fn default() -> Self {
        Self::new()
    }
}
