//! BIFF8 record parser.
//!
//! Parses BIFF8 records from XLS workbook and sheet streams.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Continue = 0x003C,
}

impl RecordId {
    pub fn from_u16(id: u16) -> Option<Self> {
        match id {
            0x0809 => Some(RecordId::Bof),
            0x000A => Some(RecordId::Eof),
            0x00A1 => Some(RecordId::CodePage),
            0x0022 => Some(RecordId::DateMode),
            0x0200 => Some(RecordId::Dimensions),
            0x020B => Some(RecordId::Index),
            0x002D => Some(RecordId::Window1),
            0x023E => Some(RecordId::Window2),
            0x0031 => Some(RecordId::Font),
            0x041E => Some(RecordId::Format),
            0x00E0 => Some(RecordId::Xf),
            0x0293 => Some(RecordId::Style),
            0x0085 => Some(RecordId::BoundSheet),
            0x00FC => Some(RecordId::Sst),
            0x0203 => Some(RecordId::Number),
            0x027E => Some(RecordId::Rk),
            0x0205 => Some(RecordId::BoolErr),
            0x00FD => Some(RecordId::LabelSst),
            0x0006 => Some(RecordId::Formula),
            0x0208 => Some(RecordId::Row),
            0x007D => Some(RecordId::ColInfo),
            0x0055 => Some(RecordId::DefColWidth),
            0x0080 => Some(RecordId::Guts),
            0x0201 => Some(RecordId::Blank),
            0x00BD => Some(RecordId::MulRk),
            0x0204 => Some(RecordId::Label),
            0x0179 => Some(RecordId::UseSelfs),
            0x008C => Some(RecordId::Country),
            0x005D => Some(RecordId::Obj),
            0x01B6 => Some(RecordId::TxO),
            0x0086 => Some(RecordId::WriteProtect),
            0x003C => Some(RecordId::Continue),
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

    /// Parse BoundSheet record: returns (stream_pos, sheet_name)
    pub fn parse_bound_sheet(data: &[u8]) -> anyhow::Result<(u32, String)> {
        if data.len() < 6 {
            anyhow::bail!("BoundSheet record too short");
        }

        let stream_pos = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        // Skip visibility (1 byte) and sheet type (1 byte)
        let name_offset = 6;

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
            // UTF-16LE
            let mut chars = Vec::with_capacity(cch);
            for i in 0..cch {
                let byte_offset = name_offset + 2 + i * 2;
                let unit = u16::from_le_bytes([data[byte_offset], data[byte_offset + 1]]);
                if let Some(c) = char::from_u32(unit as u32) {
                    chars.push(c);
                }
            }
            chars.into_iter().collect::<String>()
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
            let mut chars = Vec::with_capacity(cch);
            for i in 0..cch {
                let byte_offset = current_offset + i * 2;
                let unit = u16::from_le_bytes([data[byte_offset], data[byte_offset + 1]]);
                if let Some(c) = char::from_u32(unit as u32) {
                    chars.push(c);
                }
            }
            current_offset += cch * 2;
            chars.into_iter().collect::<String>()
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