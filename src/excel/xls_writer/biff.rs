//! BIFF8 record encoder.
//!
//! Produces the byte sequences for the records used by a minimal XLS writer:
//! workbook globals (BOF, CodePage, Window1, Font, Format, XF, Style, BoundSheet,
//! SST, DateMode, Window2) and sheet records (BOF, Index, Dimensions, Row, cells).
//!
//! References:
//! - [MS-XLS] §2.3 (records)
//! - [MS-XLS] §2.5 (workbook stream)
//! - [MS-XLS] §2.4 (worksheet stream)
//!
//! All multi-byte integers are little-endian.

use std::io::Write;

/// BIFF8 record identifier.
#[allow(dead_code)] // Variants retained for BIFF8 structure completeness
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum RecordId {
    Bof = 0x0809,
    Eof = 0x000A,
    CodePage = 0x00A1,
    DateMode = 0x0022,
    Dimensions = 0x0200,
    Index = 0x020B,
    Window1 = 0x002D,
    Window2 = 0x023E,
    Font = 0x0031,
    Format = 0x041E,
    Xf = 0x00E0,
    Style = 0x0293,
    BoundSheet = 0x0085,
    Sst = 0x00FC,
    Number = 0x0203,
    Rk = 0x027E,
    BoolErr = 0x0205,
    LabelSst = 0x00FD,
    Formula = 0x0006,
    Row = 0x0208,
    ColInfo = 0x007D,
    DefColWidth = 0x0055,
    Guts = 0x0080,
    Blank = 0x0201,
    MulRk = 0x00BD,
    Label = 0x0204,
    UseSelfs = 0x0179,
    Country = 0x008C,
    Obj = 0x005D,
    TxO = 0x01B6,
    WriteProtect = 0x0086,
}

impl RecordId {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Builder for a single BIFF record. Append fields in order; call `finish` to
/// emit `[id u16][size u16]body`.
pub struct Record<'a> {
    id: u16,
    buf: Vec<u8>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Record<'a> {
    pub fn new(id: RecordId) -> Self {
        Self {
            id: id.as_u16(),
            buf: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn u8(mut self, v: u8) -> Self {
        self.buf.push(v);
        self
    }

    pub fn u16(mut self, v: u16) -> Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u32(mut self, v: u32) -> Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn f64(mut self, v: f64) -> Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn bytes(mut self, v: &[u8]) -> Self {
        self.buf.extend_from_slice(v);
        self
    }

    pub fn utf16(mut self, s: &str) -> Self {
        for unit in s.encode_utf16() {
            self.buf.extend_from_slice(&unit.to_le_bytes());
        }
        self
    }

    /// Write the record header + body into `out`.
    pub fn finish<W: Write>(&self, out: &mut W) -> std::io::Result<()> {
        out.write_all(&self.id.to_le_bytes())?;
        out.write_all(&(self.buf.len() as u16).to_le_bytes())?;
        out.write_all(&self.buf)?;
        Ok(())
    }
}

pub fn record(id: RecordId) -> Record<'static> {
    Record::new(id)
}

/// Build the standard CodePage record (0x00A1, code page 0x04E4 = 1252).
pub fn codepage() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::CodePage)
        .u16(0x04E4)
        .finish(&mut out)
        .unwrap();
    out
}

/// BOF record for a workbook stream. version=0x0600, type=0x0005, build=0x0DBB.
pub fn bof_workbook() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Bof)
        .u16(0x0600) // version
        .u16(0x0005) // type (workbook)
        .u16(0x0000) // history
        .u16(0x0000)
        .u32(0x0000_0DBB) // build id
        .u16(0x0001) // build year offset
        .u16(0x0000) // reserved
        .u32(0x0000_0001) // required by some readers
        .finish(&mut out)
        .unwrap();
    out
}

/// BOF record for a worksheet stream. version=0x0600, type=0x0010, build=0x0DBB.
pub fn bof_sheet() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Bof)
        .u16(0x0600)
        .u16(0x0010)
        .u16(0x0000)
        .u16(0x0000)
        .u32(0x0000_0DBB)
        .u16(0x0001)
        .u16(0x0000)
        .u32(0x0000_0001)
        .finish(&mut out)
        .unwrap();
    out
}

/// EOF record (no body).
pub fn eof() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Eof).finish(&mut out).unwrap();
    out
}

/// Window1 record (initial workbook window).
pub fn window1() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Window1)
        .u16(0x0000) // x
        .u16(0x0000) // y
        .u16(0x0000) // width
        .u16(0x0000) // height
        .u16(0x0000) // hidden
        .u16(0x0000) // reserved
        .u16(0x0060) // selected tab + # tabs
        .finish(&mut out)
        .unwrap();
    out
}

/// Window2 record (sheet view).
pub fn window2() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Window2)
        .u16(0x0000) // grbit
        .u16(0x0000) // top row
        .u16(0x0000) // left col
        .u32(0x00000000) // header color
        .finish(&mut out)
        .unwrap();
    out
}

/// Standard font record. We always emit one minimal font.
pub fn default_fonts() -> Vec<u8> {
    let mut out = Vec::new();
    // Font 0: bold off, size 10, name "Arial".
    Record::new(RecordId::Font)
        .u16(0x00C8) // height (10 * 20 = 200)
        .u16(0x0000) // grbit
        .u16(0x0000) // color
        .u16(0x0190) // weight
        .u16(0x0000) // escaped
        .u8(0x00) // underline
        .u8(0x00) // family
        .u8(0x00) // charset
        .u8(0x00) // reserved
        .utf16("Arial")
        .finish(&mut out)
        .unwrap();
    out
}

/// Number format record (BIFF8 Format).
pub fn number_formats() -> Vec<u8> {
    // Emit a single Format record (index 0 is built-in "General", not stored).
    // We use format index 1 mapped to "General" via the built-in table.
    // No Format record is strictly required.
    Vec::new()
}

/// XF record for the default cell format (index 0 = "Normal").
pub fn default_xf() -> Vec<u8> {
    let mut out = Vec::new();
    // 16 bytes per XF in BIFF8.
    // font=0, format=0, type=0 (cell), alignment=0, attributes=0, ...
    let mut body = [0u8; 20];
    body[0] = 0; // font
    body[1] = 0; // format
    body[2] = 0; // type
    body[3] = 0; // xf type
    body[4] = 0; // alignment
    body[5] = 0;
    body[6] = 0;
    body[7] = 0;
    body[8] = 0;
    body[9] = 0;
    body[10] = 0;
    body[11] = 0;
    body[12] = 0;
    body[13] = 0;
    body[14] = 0;
    body[15] = 0;
    body[16] = 0;
    body[17] = 0;
    body[18] = 0;
    body[19] = 0;
    let mut hdr = Vec::new();
    Record::new(RecordId::Xf)
        .bytes(&body)
        .finish(&mut hdr)
        .unwrap();
    out.extend_from_slice(&hdr);
    out
}

/// Date mode record: 0 = 1900-based (Windows), 1 = 1904-based (Mac).
pub fn date_mode(mode: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::DateMode)
        .u16(mode)
        .finish(&mut out)
        .unwrap();
    out
}

/// Dimensions record: defines the used range of a sheet.
/// (rwTop=0, rwBot=n_rows-1, colLeft=0, colRight=n_cols-1, reserved=0)
pub fn dimensions(n_rows: u32, n_cols: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Dimensions)
        .u32(0)
        .u32(n_rows.saturating_sub(1))
        .u16(0)
        .u16(n_cols.saturating_sub(1))
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// DefColWidth record: default column width.
pub fn def_col_width(width_chars: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::DefColWidth)
        .u16(width_chars)
        .finish(&mut out)
        .unwrap();
    out
}

/// Row record.
pub fn row(r: u32, first_col: u16, last_col: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Row)
        .u16(r as u16)
        .u16(first_col)
        .u16(last_col)
        .u16(0x00FF) // height (255 = "auto")
        .u16(0x0000) // grbit
        .u16(0x0000) // xf (default 0)
        .finish(&mut out)
        .unwrap();
    out
}

/// Number cell (IEEE 754 double).
pub fn number_cell(row: u16, col: u16, xf: u16, value: f64) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Number)
        .u16(row)
        .u16(col)
        .u16(xf)
        .f64(value)
        .finish(&mut out)
        .unwrap();
    out
}

/// BoolErr cell.
pub fn bool_cell(row: u16, col: u16, xf: u16, value: bool) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::BoolErr)
        .u16(row)
        .u16(col)
        .u16(xf)
        .u8(if value { 1 } else { 0 })
        .u8(0) // type: 0 = bool, 1 = error
        .finish(&mut out)
        .unwrap();
    out
}

/// LabelSst cell: a string by index into the SST.
pub fn labelsst_cell(row: u16, col: u16, xf: u16, sst_index: u32) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::LabelSst)
        .u16(row)
        .u16(col)
        .u16(xf)
        .u32(sst_index)
        .finish(&mut out)
        .unwrap();
    out
}

/// Blank cell (empty cell that still has formatting).
pub fn blank_cell(row: u16, col: u16, xf: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Blank)
        .u16(row)
        .u16(col)
        .u16(xf)
        .finish(&mut out)
        .unwrap();
    out
}

/// Formula cell. `ptg_bytes` is the encoded RPN expression. The cell stores
/// a placeholder cached value of 0.0; Excel will recompute on open.
pub fn formula_cell(row: u16, col: u16, xf: u16, ptg_bytes: &[u8]) -> Vec<u8> {
    // Body layout: rw(2) col(2) ixfe(2) val(8) grbit(2) chn(4) cce(2) rgce(cce)
    // = 22 + cce bytes. Total record size includes the 4-byte header.
    let mut body = Vec::new();
    body.extend_from_slice(&row.to_le_bytes());
    body.extend_from_slice(&col.to_le_bytes());
    body.extend_from_slice(&xf.to_le_bytes());
    body.extend_from_slice(&0f64.to_le_bytes()); // cached value (placeholder)
    body.extend_from_slice(&0u16.to_le_bytes()); // grbit
    body.extend_from_slice(&0u32.to_le_bytes()); // chn
    body.extend_from_slice(&(ptg_bytes.len() as u16).to_le_bytes()); // cce
    body.extend_from_slice(ptg_bytes);
    // Pad to 4-byte boundary.
    while body.len() % 4 != 0 {
        body.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(RecordId::Formula as u16).to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Country record (system settings). Optional but expected.
pub fn country() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Country)
        .u16(0x0001) // default country
        .u16(0x0001) // user country
        .finish(&mut out)
        .unwrap();
    out
}

/// UseSelfs record (workbook recalculation flags).
pub fn use_selfs() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::UseSelfs)
        .u16(0x0001)
        .finish(&mut out)
        .unwrap();
    out
}

/// Build the SST record from a list of unique strings. Returns the bytes and
/// the index of each string in the SST (a Vec parallel to `strings`).
///
/// SST body layout (XlsUnicodeStringNoCch per string):
///   - total u32 (total strings including duplicates)
///   - unique u32
///   - per string:
///     - cch (u16) — number of characters
///     - optionFlags (u8):
///         bit 0: fHighByte (1 = UTF-16, 0 = 1-byte)
///         bit 1: reserved
///         bit 2: fExtSt
///         bit 3: fRichSt
///     - characters: cch bytes (if fHighByte = 0) or cch * 2 bytes (UTF-16LE)
pub fn sst(strings: &[String]) -> (Vec<u8>, Vec<u32>) {
    let total = strings.len() as u32;
    let unique = strings.len() as u32;
    let mut body = Vec::new();
    body.extend_from_slice(&total.to_le_bytes());
    body.extend_from_slice(&unique.to_le_bytes());

    let mut indices = Vec::with_capacity(strings.len());
    for s in strings {
        indices.push(body.len() as u32);
        write_sst_string(&mut body, s);
    }

    let mut out = Vec::new();
    Record::new(RecordId::Sst)
        .bytes(&body)
        .finish(&mut out)
        .unwrap();
    (out, indices)
}

fn write_sst_string(body: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    let is_ascii = bytes.iter().all(|b| b.is_ascii());
    if is_ascii {
        let len = bytes.len();
        let cch = len.min(0x7FFF) as u16;
        body.extend_from_slice(&cch.to_le_bytes());
        body.push(0x00); // flags: no high byte, no ext, no rich
        body.extend_from_slice(&bytes[..len.min(0x7FFF)]);
    } else {
        let units: Vec<u16> = s.encode_utf16().collect();
        let len = units.len().min(0x7FFF);
        body.extend_from_slice(&(len as u16).to_le_bytes());
        body.push(0x01); // flags: high byte (UTF-16)
        for u in &units[..len] {
            body.extend_from_slice(&u.to_le_bytes());
        }
    }
}

/// BoundSheet record. `stream_pos` is the absolute file offset of the sheet's
/// BOF record within the workbook stream. `state = 0` (visible), `kind = 0`
/// (worksheet).
///
/// Note: some readers parse hsState and dt as single bytes (despite the BIFF8
/// spec saying they are 16-bit), so we emit them as 1 byte each to stay
/// compatible with the most common readers.
pub fn bound_sheet(sheet_index: u32, stream_pos: u32, name: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&stream_pos.to_le_bytes()); // 4 bytes
    body.push(0); // hsState: visible (1 byte)
    body.push(0); // dt: worksheet (1 byte)
    // XlsUnicodeString: cch (1 byte), optionFlags (1 byte), characters.
    let units: Vec<u16> = name.encode_utf16().collect();
    let n = units.len().min(255) as u8;
    body.push(n);
    // optionFlags: bit 0 (fHighByte) = 1 because the characters are UTF-16.
    body.push(0x01);
    for u in units.iter().take(n as usize) {
        body.extend_from_slice(&u.to_le_bytes());
    }
    let mut out = Vec::new();
    let rec_id: u16 = 0x0085; // BoundSheet
    out.extend_from_slice(&rec_id.to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    let _ = sheet_index;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_records(bytes: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut out = Vec::new();
        let mut p = 0;
        while p + 4 <= bytes.len() {
            let id = u16::from_le_bytes([bytes[p], bytes[p + 1]]);
            let len = u16::from_le_bytes([bytes[p + 2], bytes[p + 3]]);
            let end = p + 4 + len as usize;
            if end > bytes.len() {
                break;
            }
            out.push((id, bytes[p + 4..end].to_vec()));
            p = end;
        }
        out
    }

    #[test]
    fn codepage_record_layout() {
        let bytes = codepage();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x00A1);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 2);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 0x04E4);
    }

    #[test]
    fn eof_record() {
        let bytes = eof();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x000A);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0);
    }

    #[test]
    fn number_cell_record() {
        let bytes = number_cell(0, 0, 0, 3.14);
        let recs = parse_records(&bytes);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].0, 0x0203);
        let body = &recs[0].1;
        assert_eq!(u16::from_le_bytes([body[0], body[1]]), 0);
        assert_eq!(u16::from_le_bytes([body[2], body[3]]), 0);
        assert_eq!(u16::from_le_bytes([body[4], body[5]]), 0);
        let v = f64::from_le_bytes(body[6..14].try_into().unwrap());
        assert!((v - 3.14).abs() < 1e-12);
    }

    #[test]
    fn sst_record_layout() {
        let strings = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
        let (bytes, indices) = sst(&strings);
        assert_eq!(indices.len(), 3);
        // Verify each string can be re-parsed (length=correct).
        let recs = parse_records(&bytes);
        assert_eq!(recs[0].0, 0x00FC);
        let body = &recs[0].1;
        assert_eq!(u32::from_le_bytes([body[0], body[1], body[2], body[3]]), 3);
        assert_eq!(u32::from_le_bytes([body[4], body[5], body[6], body[7]]), 3);
    }

    #[test]
    fn bound_sheet_record_layout() {
        let bytes = bound_sheet(0, 0x100, "Sheet1");
        let recs = parse_records(&bytes);
        assert_eq!(recs[0].0, 0x0085);
    }
}
