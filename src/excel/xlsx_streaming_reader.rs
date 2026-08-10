//! Streaming XLSX reader — row-by-row parsing without full materialization.
//!
//! Reads shared strings and styles upfront (small), then streams sheet XML
//! one `<row>` element at a time via a buffered reader on the ZIP entry.
//! Each row is yielded as `Vec<XlsxCellValue>` without building the full
//! dense grid in memory.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{BufReader, Read};
use zip::ZipArchive;

use super::xlsx_reader::{parse_cell_ref, XmlScanner, XlsxCellValue};
use super::xlsx_style_reader::XlsxStyleTable;

/// Streaming XLSX reader that yields rows one at a time.
///
/// Unlike `XlsxReader`, which materializes all sheets into memory, this
/// reader opens the archive, reads shared strings and styles upfront, then
/// streams the requested sheet's XML row-by-row.
pub struct XlsxStreamingReader {
    archive: ZipArchive<std::fs::File>,
    shared_strings: Vec<String>,
    styles: XlsxStyleTable,
    sheet_names: Vec<String>,
    sheet_paths: HashMap<String, String>,
}

impl XlsxStreamingReader {
    /// Open an XLSX file for streaming reads.
    pub fn from_path(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open XLSX file: {path}"))?;
        let archive = ZipArchive::new(file).context("Failed to open XLSX archive")?;

        let mut reader = Self {
            archive,
            shared_strings: Vec::new(),
            styles: XlsxStyleTable::default(),
            sheet_names: Vec::new(),
            sheet_paths: HashMap::new(),
        };

        reader.read_metadata()?;
        Ok(reader)
    }

    fn read_metadata(&mut self) -> Result<()> {
        self.shared_strings = self.read_shared_strings()?;
        self.styles = self.read_styles();

        let workbook_xml = self.read_zip_entry("xl/workbook.xml")?;
        let sheets_info = Self::parse_workbook_sheets(&workbook_xml);

        let rels_xml = self.read_zip_entry("xl/_rels/workbook.xml.rels")?;
        let rels_map = Self::parse_rels(&rels_xml);

        for (name, rid) in &sheets_info {
            let target = rels_map.get(rid).cloned().unwrap_or_else(|| {
                format!(
                    "worksheets/sheet{}.xml",
                    sheets_info.iter().position(|(n, _)| n == name).map(|i| i + 1).unwrap_or(1)
                )
            });
            let sheet_path = if let Some(stripped) = target.strip_prefix('/') {
                stripped.to_string()
            } else {
                format!("xl/{}", target)
            };
            self.sheet_names.push(name.clone());
            self.sheet_paths.insert(name.clone(), sheet_path);
        }

        Ok(())
    }

    /// Get the list of sheet names in the workbook.
    pub fn sheet_names(&self) -> &[String] {
        &self.sheet_names
    }

    /// Create a row iterator for the given sheet.
    ///
    /// The sheet XML is streamed from the ZIP entry via a buffered reader.
    /// Each call to `next()` on the iterator reads and parses one `<row>`
    /// element, returning `Vec<XlsxCellValue>`.
    pub fn row_iter(&mut self, sheet_name: &str) -> Result<RowIterator<'_>> {
        let sheet_path = self
            .sheet_paths
            .get(sheet_name)
            .cloned()
            .with_context(|| format!("Sheet not found: {sheet_name}"))?;

        let zip_file = self
            .archive
            .by_name(&sheet_path)
            .with_context(|| format!("Failed to read sheet XML: {sheet_path}"))?;

        let shared_strings = &self.shared_strings;

        Ok(RowIterator {
            reader: BufReader::with_capacity(64 * 1024, zip_file),
            shared_strings,
            buffer: Vec::with_capacity(8192),
            done: false,
        })
    }

    /// Get a reference to the parsed style table.
    pub fn styles(&self) -> &XlsxStyleTable {
        &self.styles
    }

    fn read_shared_strings(&mut self) -> Result<Vec<String>> {
        let xml = match self.read_zip_entry_opt("xl/sharedStrings.xml") {
            Ok(data) => data,
            Err(_) => return Ok(Vec::new()),
        };

        let xml_str = String::from_utf8_lossy(&xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut strings = Vec::new();
        if scanner.find_open_tag("sst").is_none() {
            return Ok(strings);
        }
        scanner.skip_open_tag();

        while scanner.find_open_tag("si").is_some() {
            let si_start = scanner.pos;
            if scanner.is_self_closing(si_start) {
                scanner.skip_open_tag();
                strings.push(String::new());
                continue;
            }
            scanner.skip_open_tag();

            let mut text = String::new();
            let save = scanner.pos;
            if scanner.find_open_tag("t").is_some() {
                let t_start = scanner.pos;
                if !scanner.is_self_closing(t_start) {
                    scanner.skip_open_tag();
                    text.push_str(&scanner.read_text_until_close("t"));
                } else {
                    scanner.skip_open_tag();
                }
            } else {
                scanner.pos = save;
                while scanner.find_open_tag("r").is_some() {
                    let r_start = scanner.pos;
                    if scanner.is_self_closing(r_start) {
                        scanner.skip_open_tag();
                        continue;
                    }
                    scanner.skip_open_tag();
                    let save_r = scanner.pos;
                    if scanner.find_open_tag("t").is_some() {
                        let t_start = scanner.pos;
                        if !scanner.is_self_closing(t_start) {
                            scanner.skip_open_tag();
                            text.push_str(&scanner.read_text_until_close("t"));
                        } else {
                            scanner.skip_open_tag();
                        }
                    } else {
                        scanner.pos = save_r;
                    }
                }
            }

            strings.push(text);
        }

        Ok(strings)
    }

    fn read_styles(&mut self) -> XlsxStyleTable {
        match self.read_zip_entry_opt("xl/styles.xml") {
            Ok(data) => XlsxStyleTable::parse(&data),
            Err(_) => XlsxStyleTable::default(),
        }
    }

    fn read_zip_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        self.read_zip_entry_opt(name)
    }

    fn read_zip_entry_opt(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut entry = self
            .archive
            .by_name(name)
            .with_context(|| format!("Failed to find '{name}' in XLSX archive"))?;
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
        Ok(buf)
    }

    fn parse_workbook_sheets(xml: &[u8]) -> Vec<(String, String)> {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut sheets = Vec::new();
        if scanner.find_open_tag("sheets").is_none() {
            return sheets;
        }
        scanner.skip_open_tag();

        while scanner.find_open_tag("sheet").is_some() {
            let tag_start = scanner.pos;
            let _name = scanner.read_tag_name(tag_start);
            let (attrs, _) = scanner.parse_attributes(tag_start + _name.len());

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

        let mut rels = HashMap::new();
        while scanner.find_open_tag("Relationship").is_some() {
            let tag_start = scanner.pos;
            let _name = scanner.read_tag_name(tag_start);
            let (attrs, _) = scanner.parse_attributes(tag_start + _name.len());

            let id = attrs.get("Id").cloned().unwrap_or_default();
            let target = attrs.get("Target").cloned().unwrap_or_default();
            if !id.is_empty() {
                rels.insert(id, target);
            }
            scanner.skip_open_tag();
        }

        rels
    }
}

/// Iterator that yields rows from an XLSX sheet as `Vec<XlsxCellValue>`.
///
/// Reads the sheet XML via a buffered reader, extracting one `<row>`
/// element at a time. Each row is parsed into cell values with proper
/// column positioning (gaps filled with `XlsxCellValue::Empty`).
pub struct RowIterator<'a> {
    reader: BufReader<zip::read::ZipFile<'a>>,
    shared_strings: &'a [String],
    buffer: Vec<u8>,
    done: bool,
}

impl<'a> RowIterator<'a> {
    /// Read from the buffered reader and extract the next complete `<row>` element.
    /// Returns the row XML bytes, or None at end of sheet data.
    fn extract_next_row(&mut self) -> Option<Vec<u8>> {
        if self.done {
            return None;
        }

        loop {
            // Look for `<row` in the buffer
            if let Some(row_start) = find_subslice(&self.buffer, b"<row") {
                // Verify it's actually a <row tag (followed by space, >, /, etc.)
                let after = row_start + 4;
                if after < self.buffer.len() {
                    let c = self.buffer[after];
                    if c != b' ' && c != b'>' && c != b'/' && c != b'\t' && c != b'\n' && c != b'\r' {
                        // Not a <row tag — discard this prefix and continue
                        self.buffer.drain(..after);
                        continue;
                    }
                }

                // Try to find the end of the row element
                // Check for self-closing <row ... />
                let after_tag = row_start + 4;
                if let Some(close_pos) = find_subslice_from(&self.buffer, b"/>", after_tag) {
                    if let Some(gt_pos) = find_subslice_from(&self.buffer, b">", after_tag) {
                        if close_pos < gt_pos {
                            // Self-closing — empty row
                            let row_end = close_pos + 2;
                            let row_xml = self.buffer[row_start..row_end].to_vec();
                            self.buffer.drain(..row_end);
                            return Some(row_xml);
                        }
                    }
                }

                // Look for </row>
                if let Some(close_pos) = find_subslice_from(&self.buffer, b"</row>", after_tag) {
                    let row_end = close_pos + 6;
                    let row_xml = self.buffer[row_start..row_end].to_vec();
                    self.buffer.drain(..row_end);
                    return Some(row_xml);
                }

                // Not enough data yet — read more
                if !self.fill_buffer() {
                    // EOF — no more complete rows
                    self.done = true;
                    return None;
                }
            } else {
                // No `<row` in buffer — keep last 4 bytes (partial match) and read more
                if self.buffer.len() > 4 {
                    self.buffer.drain(..self.buffer.len() - 4);
                }
                if !self.fill_buffer() {
                    self.done = true;
                    return None;
                }
            }
        }
    }

    fn fill_buffer(&mut self) -> bool {
        let mut buf = [0u8; 32 * 1024];
        let n = self.reader.read(&mut buf).unwrap_or(0);
        if n > 0 {
            self.buffer.extend_from_slice(&buf[..n]);
            true
        } else {
            false
        }
    }

    /// Parse a single `<row>` XML fragment into cell values.
    fn parse_row(xml: &[u8], shared_strings: &[String]) -> Vec<XlsxCellValue> {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());

        if scanner.find_open_tag("row").is_none() {
            return Vec::new();
        }

        let row_tag_start = scanner.pos;
        if scanner.is_self_closing(row_tag_start) {
            scanner.skip_open_tag();
            return Vec::new();
        }

        let _row_name = scanner.read_tag_name(row_tag_start);
        let (_row_attrs, _) = scanner.parse_attributes(row_tag_start + _row_name.len());
        scanner.skip_open_tag();

        let mut cells: Vec<(u16, XlsxCellValue)> = Vec::new();
        let mut max_col: u16 = 0;

        loop {
            if scanner.find_open_tag("c").is_none() {
                break;
            }

            let c_tag_start = scanner.pos;
            let _c_name = scanner.read_tag_name(c_tag_start);
            let (c_attrs, _) = scanner.parse_attributes(c_tag_start + _c_name.len());

            let cell_ref = c_attrs.get("r").cloned().unwrap_or_default();
            let cell_type = c_attrs.get("t").cloned().unwrap_or_else(|| "n".to_string());
            let (_, col_idx) = if cell_ref.is_empty() {
                (0u32, cells.len() as u16)
            } else {
                parse_cell_ref(&cell_ref)
            };

            if scanner.is_self_closing(c_tag_start) {
                scanner.skip_open_tag();
                cells.push((col_idx, XlsxCellValue::Empty));
                max_col = max_col.max(col_idx);
                continue;
            }
            scanner.skip_open_tag();

            let mut value = XlsxCellValue::Empty;
            match cell_type.as_str() {
                "s" => {
                    let save = scanner.pos;
                    if scanner.find_open_tag("v").is_some() {
                        let v_start = scanner.pos;
                        if !scanner.is_self_closing(v_start) {
                            scanner.skip_open_tag();
                            let text = scanner.read_text_until_close("v");
                            if let Ok(idx) = text.parse::<usize>() {
                                value = XlsxCellValue::String(
                                    shared_strings.get(idx).cloned().unwrap_or_default(),
                                );
                            }
                        } else {
                            scanner.skip_open_tag();
                        }
                    } else {
                        scanner.pos = save;
                    }
                }
                "inlineStr" => {
                    let save = scanner.pos;
                    if scanner.find_open_tag("t").is_some() {
                        let t_start = scanner.pos;
                        if !scanner.is_self_closing(t_start) {
                            scanner.skip_open_tag();
                            let text = scanner.read_text_until_close("t");
                            value = XlsxCellValue::String(text);
                        } else {
                            scanner.skip_open_tag();
                        }
                    } else {
                        scanner.pos = save;
                    }
                }
                "b" => {
                    let save = scanner.pos;
                    if scanner.find_open_tag("v").is_some() {
                        let v_start = scanner.pos;
                        if !scanner.is_self_closing(v_start) {
                            scanner.skip_open_tag();
                            let text = scanner.read_text_until_close("v");
                            value = XlsxCellValue::Bool(text == "1" || text.eq_ignore_ascii_case("true"));
                        } else {
                            scanner.skip_open_tag();
                        }
                    } else {
                        scanner.pos = save;
                    }
                }
                "e" => {
                    let save = scanner.pos;
                    if scanner.find_open_tag("v").is_some() {
                        let v_start = scanner.pos;
                        if !scanner.is_self_closing(v_start) {
                            scanner.skip_open_tag();
                            let text = scanner.read_text_until_close("v");
                            value = XlsxCellValue::Error(text);
                        } else {
                            scanner.skip_open_tag();
                        }
                    } else {
                        scanner.pos = save;
                    }
                }
                "str" => {
                    let save = scanner.pos;
                    if scanner.find_open_tag("v").is_some() {
                        let v_start = scanner.pos;
                        if !scanner.is_self_closing(v_start) {
                            scanner.skip_open_tag();
                            let text = scanner.read_text_until_close("v");
                            value = XlsxCellValue::String(text);
                        } else {
                            scanner.skip_open_tag();
                        }
                    } else {
                        scanner.pos = save;
                    }
                }
                _ => {
                    let save = scanner.pos;
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
                    } else {
                        scanner.pos = save;
                    }
                }
            }

            cells.push((col_idx, value));
            max_col = max_col.max(col_idx);
        }

        // Build dense row vector
        let n_cols = (max_col as usize) + 1;
        let mut row = vec![XlsxCellValue::Empty; n_cols];
        for (col, val) in cells {
            if (col as usize) < n_cols {
                row[col as usize] = val;
            }
        }

        row
    }
}

impl<'a> Iterator for RowIterator<'a> {
    type Item = Vec<XlsxCellValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let row_xml = self.extract_next_row()?;
        Some(Self::parse_row(&row_xml, self.shared_strings))
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let end = haystack.len() - needle.len() + 1;
    for i in 0..end {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}

fn find_subslice_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() || haystack.len() - from < needle.len() {
        return None;
    }
    let end = haystack.len() - needle.len() + 1;
    for i in from..end {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_subslice() {
        assert_eq!(find_subslice(b"hello <row world", b"<row"), Some(6));
        assert_eq!(find_subslice(b"no match", b"<row"), None);
        assert_eq!(find_subslice(b"", b"<row"), None);
    }

    #[test]
    fn test_find_subslice_from() {
        assert_eq!(find_subslice_from(b"<row></row>", b"</row>", 0), Some(5));
        assert_eq!(find_subslice_from(b"<row></row>", b"</row>", 6), None);
    }
}
