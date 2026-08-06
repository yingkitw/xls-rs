//! BIFF8 record parser.
//!
//! Parses BIFF8 records from XLS workbook and sheet streams.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum RecordId {
    Bof = 0x0809,
    Eof = 0x000A,
    CodePage = 0x0042,
    DateMode = 0x0022,
    Dimensions = 0x0200,
    Index = 0x020B,
    Window1 = 0x003D,
    Window2 = 0x023E,
    Font = 0x0031,
    Format = 0x041E,
    Xf = 0x00E0,
    Style = 0x0293,
    BoundSheet = 0x0085,
    Sst = 0x00FC,
    ExtSst = 0x00FF,
    Number = 0x0203,
    Rk = 0x027E,
    BoolErr = 0x0205,
    LabelSst = 0x00FD,
    Formula = 0x0006,
    Row = 0x0208,
    ColInfo = 0x007D,
    ColWidth = 0x0024,
    DefColWidth = 0x0055,
    Guts = 0x0080,
    Blank = 0x0201,
    MulRk = 0x00BD,
    Label = 0x0204,
    UseSelfs = 0x0160,
    Country = 0x008C,
    Obj = 0x005D,
    TxO = 0x01B6,
    WriteProtect = 0x0086,
    Continue = 0x003C,
    // BIFF8 setup records (must be recognized so the reader can skip past them).
    InterfaceHdr = 0x00E1,
    InterfaceEnd = 0x00E2,
    Mms = 0x00C1,
    WriteAccess = 0x005C,
    Dsf = 0x0161,
    TabId = 0x013D,
    FnGroupName = 0x009C,
    WindowProtect = 0x0019,
    Protect = 0x0012,
    ObjectProtect = 0x0063,
    Password = 0x0013,
    Prot4Rev = 0x01AF,
    Prot4RevPass = 0x01BC,
    Backup = 0x0040,
    HideObj = 0x008D,
    Precision = 0x000E,
    RefreshAll = 0x01B7,
    BookBool = 0x00DA,
    Palette = 0x0092,
    // Per-sheet setup records. The writer emits these right after the
    // sheet BOF; the reader must know about them so it skips past
    // them when iterating.
    CalcCount = 0x000C,
    CalcMode = 0x000D,
    RefMode = 0x000F,
    Delta = 0x0010,
    Iteration = 0x0011,
    SaferRecalc = 0x005F,
    WsBool = 0x0081,
    GridSet = 0x0082,
    HCenter = 0x0083,
    VCenter = 0x0084,
    PrintHeaders = 0x002A,
    PrintGridlines = 0x002B,
    DefaultRowHeight = 0x0225,
}

/// Cached value type from a Formula record.
#[derive(Debug, Clone)]
pub enum FormulaCachedValue {
    Number(f64),
    String,
    Bool(bool),
    Error(String),
}

/// Map BIFF8 error code byte to Excel error name.
pub fn error_name(code: u8) -> String {
    match code {
        0x00 => "NULL!".to_string(),
        0x07 => "DIV/0!".to_string(),
        0x0F => "VALUE!".to_string(),
        0x17 => "REF!".to_string(),
        0x1D => "NAME?".to_string(),
        0x24 => "NUM!".to_string(),
        0x2A => "N/A".to_string(),
        _ => format!("ERR{}", code),
    }
}

impl RecordId {
    pub fn from_u16(id: u16) -> Option<Self> {
        match id {
            0x0809 => Some(RecordId::Bof),
            0x000A => Some(RecordId::Eof),
            0x0042 => Some(RecordId::CodePage),
            0x0022 => Some(RecordId::DateMode),
            0x0200 => Some(RecordId::Dimensions),
            0x020B => Some(RecordId::Index),
            0x003D => Some(RecordId::Window1),
            0x023E => Some(RecordId::Window2),
            0x0031 => Some(RecordId::Font),
            0x041E => Some(RecordId::Format),
            0x00E0 => Some(RecordId::Xf),
            0x0293 => Some(RecordId::Style),
            0x0085 => Some(RecordId::BoundSheet),
            0x00FC => Some(RecordId::Sst),
            0x00FF => Some(RecordId::ExtSst),
            0x0203 => Some(RecordId::Number),
            0x027E => Some(RecordId::Rk),
            0x0205 => Some(RecordId::BoolErr),
            0x00FD => Some(RecordId::LabelSst),
            0x0006 => Some(RecordId::Formula),
            0x0208 => Some(RecordId::Row),
            0x007D => Some(RecordId::ColInfo),
            0x0024 => Some(RecordId::ColWidth),
            0x0055 => Some(RecordId::DefColWidth),
            0x0080 => Some(RecordId::Guts),
            0x0201 => Some(RecordId::Blank),
            0x00BD => Some(RecordId::MulRk),
            0x0204 => Some(RecordId::Label),
            0x0160 => Some(RecordId::UseSelfs),
            0x008C => Some(RecordId::Country),
            0x005D => Some(RecordId::Obj),
            0x01B6 => Some(RecordId::TxO),
            0x0086 => Some(RecordId::WriteProtect),
            0x003C => Some(RecordId::Continue),
            0x00E1 => Some(RecordId::InterfaceHdr),
            0x00E2 => Some(RecordId::InterfaceEnd),
            0x00C1 => Some(RecordId::Mms),
            0x005C => Some(RecordId::WriteAccess),
            0x0161 => Some(RecordId::Dsf),
            0x013D => Some(RecordId::TabId),
            0x009C => Some(RecordId::FnGroupName),
            0x0019 => Some(RecordId::WindowProtect),
            0x0012 => Some(RecordId::Protect),
            0x0063 => Some(RecordId::ObjectProtect),
            0x0013 => Some(RecordId::Password),
            0x01AF => Some(RecordId::Prot4Rev),
            0x01BC => Some(RecordId::Prot4RevPass),
            0x0040 => Some(RecordId::Backup),
            0x008D => Some(RecordId::HideObj),
            0x000E => Some(RecordId::Precision),
            0x01B7 => Some(RecordId::RefreshAll),
            0x00DA => Some(RecordId::BookBool),
            0x0092 => Some(RecordId::Palette),
            0x000C => Some(RecordId::CalcCount),
            0x000D => Some(RecordId::CalcMode),
            0x000F => Some(RecordId::RefMode),
            0x0010 => Some(RecordId::Delta),
            0x0011 => Some(RecordId::Iteration),
            0x005F => Some(RecordId::SaferRecalc),
            0x0081 => Some(RecordId::WsBool),
            0x0082 => Some(RecordId::GridSet),
            0x0083 => Some(RecordId::HCenter),
            0x0084 => Some(RecordId::VCenter),
            0x002A => Some(RecordId::PrintHeaders),
            0x002B => Some(RecordId::PrintGridlines),
            0x0225 => Some(RecordId::DefaultRowHeight),
            _ => None,
        }
    }
}

/// A single BIFF8 record
#[derive(Debug, Clone)]
pub struct BiffRecord {
    pub id: RecordId,
    pub data: Vec<u8>,
}

impl BiffRecord {
    /// Parse a record from bytes at offset, returns (record, bytes_consumed)
    pub fn parse_at(data: &[u8], offset: usize) -> Option<(Self, usize)> {
        if offset + 4 > data.len() {
            return None;
        }

        let id = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;

        if offset + 4 + len > data.len() {
            return None;
        }

        let record_id = RecordId::from_u16(id)?;
        let record_data = data[offset + 4..offset + 4 + len].to_vec();

        Some((
            BiffRecord {
                id: record_id,
                data: record_data,
            },
            offset + 4 + len,
        ))
    }

    /// Parse stream into iterator of records
    pub fn parse_stream(data: &[u8]) -> BiffRecordIterator<'_> {
        BiffRecordIterator { data, offset: 0 }
    }

    /// Parse BoundSheet record: returns (stream_pos, sheet_name).
    ///
    /// BIFF8 layout (MS-XLS §2.4.86):
    ///   - Position (4)
    ///   - Visibility (2) — 0 = visible, 1 = hidden, 2 = very hidden
    ///   - Type (1)
    ///   - Reserved (1) — MUST be 0
    ///   - Name (XlsUnicodeString)
    ///
    /// We also accept the legacy BIFF5/7 layout (1-byte visibility,
    /// no reserved) for backward-compat with older writers.
    pub fn parse_bound_sheet(data: &[u8]) -> anyhow::Result<(u32, String)> {
        if data.len() < 6 {
            anyhow::bail!("BoundSheet record too short");
        }

        let stream_pos = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

        // Detect layout: BIFF8 starts the name at offset 8, BIFF5/7 at
        // offset 6. We can disambiguate by looking at the byte at
        // offset 6: if it parses as a sensible cch (0..=255) AND the
        // record is long enough, treat as BIFF5/7. Otherwise use
        // BIFF8.
        let is_legacy = data.len() >= 8
            && (data[6] as usize) <= 255
            && data[6] != 0 // cch 0 is a degenerate sheet name
            && data[6] <= (data.len() - 8) as u8;

        let (name_offset, visibility_end) = if is_legacy {
            (6usize, 6usize)
        } else {
            (8usize, 8usize)
        };
        let _visibility = &data[4..visibility_end];

        if name_offset >= data.len() {
            anyhow::bail!("BoundSheet record missing name");
        }

        // Parse XlsUnicodeString (cch as u8, flags, then chars)
        let cch = data[name_offset] as usize;
        let flags = data[name_offset + 1];
        let is_high_byte = (flags & 0x01) != 0;

        let expected_len = if is_high_byte {
            name_offset + 2 + cch * 2
        } else {
            name_offset + 2 + cch
        };

        if data.len() < expected_len {
            anyhow::bail!("BoundSheet record name too short");
        }

        let name = if is_high_byte {
            // UTF-16LE (may contain surrogate pairs for astral codepoints)
            let units: Vec<u16> = (0..cch)
                .map(|i| {
                    let byte_offset = name_offset + 2 + i * 2;
                    u16::from_le_bytes([data[byte_offset], data[byte_offset + 1]])
                })
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            // Compressed (ASCII)
            data[name_offset + 2..name_offset + 2 + cch]
                .iter()
                .map(|&b| b as char)
                .collect()
        };

        Ok((stream_pos, name))
    }

    /// Parse SST (Shared String Table) record
    pub fn parse_sst(data: &[u8]) -> anyhow::Result<Vec<String>> {
        if data.len() < 8 {
            return Ok(Vec::new());
        }

        let total = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let unique = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

        let mut strings = Vec::with_capacity(unique.min(total));
        let mut offset = 8;

        for _ in 0..unique {
            if offset >= data.len() {
                break;
            }

            let (string, new_offset) = Self::parse_sst_string(data, offset)?;
            strings.push(string);
            offset = new_offset;
        }

        Ok(strings)
    }

    /// Parse a single SST string
    fn parse_sst_string(data: &[u8], offset: usize) -> anyhow::Result<(String, usize)> {
        if offset + 2 > data.len() {
            anyhow::bail!("SST string header too short");
        }

        let cch = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        let flags = data[offset + 2];

        let is_high_byte = (flags & 0x01) != 0;
        let has_rich = (flags & 0x08) != 0;
        let has_ext = (flags & 0x04) != 0;

        let mut current_offset = offset + 3;

        // Skip rich formatting data if present
        if has_rich {
            if current_offset + 4 > data.len() {
                anyhow::bail!("SST rich formatting data too short");
            }
            let rich_count = u32::from_le_bytes([
                data[current_offset],
                data[current_offset + 1],
                data[current_offset + 2],
                data[current_offset + 3],
            ]) as usize;
            current_offset += 4 + rich_count * 4;
        }

        // Skip extended data if present
        if has_ext {
            if current_offset + 4 > data.len() {
                anyhow::bail!("SST extended data too short");
            }
            let ext_len = u32::from_le_bytes([
                data[current_offset],
                data[current_offset + 1],
                data[current_offset + 2],
                data[current_offset + 3],
            ]) as usize;
            current_offset += 4 + ext_len;
        }

        // Parse characters
        let string = if is_high_byte {
            if current_offset + cch * 2 > data.len() {
                anyhow::bail!("SST UTF-16 string data too short");
            }
            let units: Vec<u16> = (0..cch)
                .map(|i| {
                    let byte_offset = current_offset + i * 2;
                    u16::from_le_bytes([data[byte_offset], data[byte_offset + 1]])
                })
                .collect();
            current_offset += cch * 2;
            String::from_utf16_lossy(&units)
        } else {
            if current_offset + cch > data.len() {
                anyhow::bail!("SST compressed string data too short");
            }
            let string = data[current_offset..current_offset + cch]
                .iter()
                .map(|&b| b as char)
                .collect();
            current_offset += cch;
            string
        };

        Ok((string, current_offset))
    }

    /// Parse LabelSst record: returns (row, col, xf_index, sst_index)
    pub fn parse_labelsst(data: &[u8]) -> anyhow::Result<(u16, u16, u16, u32)> {
        if data.len() < 10 {
            anyhow::bail!("LabelSst record too short");
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);
        let sst_index = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);

        Ok((row, col, xf_index, sst_index))
    }

    /// Parse Number record: returns (row, col, xf_index, value)
    pub fn parse_number(data: &[u8]) -> anyhow::Result<(u16, u16, u16, f64)> {
        if data.len() < 14 {
            anyhow::bail!("Number record too short");
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);
        let value = f64::from_le_bytes([
            data[6], data[7], data[8], data[9],
            data[10], data[11], data[12], data[13],
        ]);

        Ok((row, col, xf_index, value))
    }

    /// Parse BoolErr record: returns (row, col, xf_index, value, is_error)
    pub fn parse_boolerr(data: &[u8]) -> anyhow::Result<(u16, u16, u16, u8, bool)> {
        if data.len() < 8 {
            anyhow::bail!("BoolErr record too short");
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);
        let value = data[6];
        let is_error = data[7] != 0;

        Ok((row, col, xf_index, value, is_error))
    }

    /// Parse Blank record: returns (row, col, xf_index)
    pub fn parse_blank(data: &[u8]) -> anyhow::Result<(u16, u16, u16)> {
        if data.len() < 6 {
            anyhow::bail!("Blank record too short");
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);

        Ok((row, col, xf_index))
    }

    /// Parse Formula record: returns (row, col, xf_index, cached_result)
    pub fn parse_formula(data: &[u8]) -> anyhow::Result<(u16, u16, u16, f64)> {
        if data.len() < 20 {
            anyhow::bail!("Formula record too short");
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);
        let cached_result = f64::from_le_bytes([
            data[6], data[7], data[8], data[9],
            data[10], data[11], data[12], data[13],
        ]);

        Ok((row, col, xf_index, cached_result))
    }

    /// Parse Formula record and return the cached value type.
    /// Returns (row, col, xf_index, FormulaResult) where FormulaResult
    /// indicates whether the cached value is a number, string, boolean, or error.
    pub fn parse_formula_result(data: &[u8]) -> anyhow::Result<(u16, u16, u16, FormulaCachedValue)> {
        if data.len() < 20 {
            anyhow::bail!("Formula record too short");
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);

        // Check for special value marker (0xFFFF in bytes 6-7)
        let sentinel = u16::from_le_bytes([data[6], data[7]]);
        if sentinel == 0xFFFF {
            // Bytes 8 is the type: 0 = string, 1 = boolean, 2 = error
            // Byte 9 is the value (for boolean/error)
            let value_type = data[8];
            let value_byte = data[9];
            match value_type {
                0 => Ok((row, col, xf_index, FormulaCachedValue::String)),
                1 => Ok((row, col, xf_index, FormulaCachedValue::Bool(value_byte != 0))),
                2 => Ok((row, col, xf_index, FormulaCachedValue::Error(error_name(value_byte)))),
                _ => Ok((row, col, xf_index, FormulaCachedValue::Number(0.0))),
            }
        } else {
            // It's a number (8 bytes at offset 6)
            let num = f64::from_le_bytes([
                data[6], data[7], data[8], data[9],
                data[10], data[11], data[12], data[13],
            ]);
            Ok((row, col, xf_index, FormulaCachedValue::Number(num)))
        }
    }

    /// Parse RK record (compressed number): returns (row, col, xf_index, value)
    pub fn parse_rk(data: &[u8]) -> Option<(u16, u16, u16, f64)> {
        if data.len() < 10 {
            return None;
        }

        let row = u16::from_le_bytes([data[0], data[1]]);
        let col = u16::from_le_bytes([data[2], data[3]]);
        let xf_index = u16::from_le_bytes([data[4], data[5]]);

        // Parse RK encoded value (4 bytes at offset 6)
        let rk_bytes = &data[6..10];
        let rk_value = u32::from_le_bytes([rk_bytes[0], rk_bytes[1], rk_bytes[2], rk_bytes[3]]);

        let is_int = (rk_value & 0x02) != 0;
        let is_100 = (rk_value & 0x01) != 0;
        let value_bits = rk_value & 0xFFFFFFFC;

        let value = if is_int {
            let int_value = (value_bits as i32) >> 2;
            int_value as f64
        } else {
            let double_bits: u64 = value_bits as u64;
            f64::from_bits(double_bits << 34)
        };

        let final_value = if is_100 { value / 100.0 } else { value };

        Some((row, col, xf_index, final_value))
    }
}

/// Iterator over BIFF8 records in a stream
pub struct BiffRecordIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for BiffRecordIterator<'a> {
    type Item = BiffRecord;

    fn next(&mut self) -> Option<Self::Item> {
        while self.offset < self.data.len() {
            if let Some((record, new_offset)) = BiffRecord::parse_at(self.data, self.offset) {
                self.offset = new_offset;
                return Some(record);
            } else {
                // Failed to parse record, advance and try again
                self.offset += 1;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_id_conversion() {
        assert_eq!(RecordId::from_u16(0x0809), Some(RecordId::Bof));
        assert_eq!(RecordId::from_u16(0x000A), Some(RecordId::Eof));
        assert_eq!(RecordId::from_u16(0x00FC), Some(RecordId::Sst));
        assert_eq!(RecordId::from_u16(0x0203), Some(RecordId::Number));
        assert_eq!(RecordId::from_u16(0x9999), None);
    }

    #[test]
    fn test_parse_simple_record() {
        let data = vec![
            0x03, 0x02,  // Record ID (Number = 0x0203)
            0x0E, 0x00,  // Length (14 bytes)
            0x00, 0x00,  // Row 0
            0x00, 0x00,  // Col 0
            0x00, 0x00,  // XF index 0
            // IEEE 754 double for 42.5
            0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x45, 0x40,
        ];

        let (record, consumed) = BiffRecord::parse_at(&data, 0).unwrap();
        assert_eq!(record.id, RecordId::Number);
        assert_eq!(consumed, 18);

        let (row, col, xf, value) = BiffRecord::parse_number(&record.data).unwrap();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
        assert_eq!(xf, 0);
        assert!((value - 42.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_labelsst() {
        let data = vec![
            0x00, 0x00,  // Row 0
            0x01, 0x00,  // Col 1
            0x00, 0x00,  // XF index 0
            0x05, 0x00, 0x00, 0x00,  // SST index 5
        ];

        let (row, col, xf, sst) = BiffRecord::parse_labelsst(&data).unwrap();
        assert_eq!(row, 0);
        assert_eq!(col, 1);
        assert_eq!(xf, 0);
        assert_eq!(sst, 5);
    }

    #[test]
    fn test_parse_boolerr() {
        // Boolean TRUE
        let data = vec![
            0x00, 0x00,  // Row 0
            0x02, 0x00,  // Col 2
            0x00, 0x00,  // XF index 0
            0x01,        // Value: TRUE
            0x00,        // Not error
        ];

        let (row, col, xf, value, is_error) = BiffRecord::parse_boolerr(&data).unwrap();
        assert_eq!(row, 0);
        assert_eq!(col, 2);
        assert_eq!(xf, 0);
        assert_eq!(value, 1);
        assert!(!is_error);
    }

    #[test]
    fn test_parse_rk() {
        // RK record for integer 100 (div100 = false, is_int = true)
        let data = vec![
            0x00, 0x00,  // Row 0
            0x00, 0x00,  // Col 0
            0x00, 0x00,  // XF index 0
            // RK value: 100 << 2 = 0x190 (with is_int=0x02)
            0x92, 0x01, 0x00, 0x00,
        ];

        let (row, col, xf, value) = BiffRecord::parse_rk(&data).unwrap();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
        assert_eq!(xf, 0);
        assert!((value - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_sst_parsing_ascii() {
        // SST with single ASCII string "Hello"
        let data = vec![
            0x01, 0x00, 0x00, 0x00,  // Total: 1
            0x01, 0x00, 0x00, 0x00,  // Unique: 1
            // String: cch=5, flags=0 (compressed, no rich/ext)
            0x05, 0x00, 0x00,
            b'H', b'e', b'l', b'l', b'o',
        ];

        let strings = BiffRecord::parse_sst(&data).unwrap();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0], "Hello");
    }

    #[test]
    fn test_sst_parsing_utf16() {
        // SST with single UTF-16 string "Hello"
        let mut data = vec![
            0x01, 0x00, 0x00, 0x00,  // Total: 1
            0x01, 0x00, 0x00, 0x00,  // Unique: 1
            // String: cch=5, flags=0x01 (high byte, no rich/ext)
            0x05, 0x00, 0x01,
        ];
        // UTF-16LE for "Hello"
        for c in "Hello".encode_utf16() {
            data.extend_from_slice(&c.to_le_bytes());
        }

        let strings = BiffRecord::parse_sst(&data).unwrap();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0], "Hello");
    }

    #[test]
    fn test_bound_sheet_parsing() {
        // BoundSheet record
        let mut data = vec![
            0x00, 0x10, 0x00, 0x00,  // Stream position: 0x1000
            0x00,                    // Visible
            0x00,                    // Type: worksheet
        ];
        // Sheet name "Test" (compressed)
        data.push(4);  // cch
        data.push(0);  // flags (compressed)
        data.extend_from_slice(b"Test");

        let (offset, name) = BiffRecord::parse_bound_sheet(&data).unwrap();
        assert_eq!(offset, 0x1000);
        assert_eq!(name, "Test");
    }
}