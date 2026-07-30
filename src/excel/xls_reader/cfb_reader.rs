//! CFB (Compound File Binary) reader.
//!
//! Parses OLE2 Compound File Binary format containers used by XLS files.
//! Keeps a single owned byte buffer (no per-sector copies) and bounds FAT
//! chain walks so cyclic sector graphs cannot hang or grow forever.

use std::collections::HashSet;

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const SECTOR_SIZE: usize = 512;
const MINI_SECTOR_SIZE: usize = 64;
const MINI_CUTOFF: usize = 4096;
const DIFAT_IN_HEADER: usize = 109;

const FREESECT: u32 = 0xFFFFFFFF;
const ENDOFCHAIN: u32 = 0xFFFFFFFE;
const NOSTREAM: u32 = 0xFFFFFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Unknown = 0,
    Storage = 1,
    Stream = 2,
    Root = 5,
}

impl ObjectType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ObjectType::Storage,
            2 => ObjectType::Stream,
            5 => ObjectType::Root,
            _ => ObjectType::Unknown,
        }
    }
}

/// Directory entry
#[derive(Debug, Clone)]
struct DirectoryEntry {
    name: String,
    object_type: ObjectType,
    start_sector: u32,
    size: u64,
    left: u32,
    right: u32,
    child: u32,
}

impl DirectoryEntry {
    fn new() -> Self {
        Self {
            name: String::new(),
            object_type: ObjectType::Unknown,
            start_sector: NOSTREAM,
            size: 0,
            left: NOSTREAM,
            right: NOSTREAM,
            child: NOSTREAM,
        }
    }
}

/// CFB container reader
#[allow(dead_code)] // difat / first_* retained for CFB structure completeness
pub struct CfbReader {
    /// Full file bytes (header + sectors). Sector data is sliced on demand.
    data: Vec<u8>,
    fat: Vec<u32>,
    difat: Vec<u32>,
    directory: Vec<DirectoryEntry>,
    first_dir_sector: u32,
    first_minifat_sector: u32,
    mini_fat: Vec<u32>,
}

impl CfbReader {
    /// Parse CFB container from owned bytes (single buffer; no sector duplication).
    pub fn parse(data: Vec<u8>) -> anyhow::Result<Self> {
        Self::parse_bytes(data)
    }

    /// Parse CFB container from a borrowed slice (copies once into an owned buffer).
    pub fn parse_slice(data: &[u8]) -> anyhow::Result<Self> {
        Self::parse_bytes(data.to_vec())
    }

    fn parse_bytes(data: Vec<u8>) -> anyhow::Result<Self> {
        // Check magic
        if data.len() < 8 || data[0..8] != CFB_MAGIC {
            anyhow::bail!("Invalid CFB magic bytes");
        }

        // Parse header
        if data.len() < HEADER_SIZE {
            anyhow::bail!("CFB data too short for header");
        }

        let sector_shift = u16::from_le_bytes([data[30], data[31]]);
        if sector_shift != 9 {
            anyhow::bail!("Only 512-byte sectors are supported (got {})", 1 << sector_shift);
        }

        let mini_sector_shift = u16::from_le_bytes([data[32], data[33]]);
        let _mini_sector_size = 1 << mini_sector_shift;

        let _num_dir_sectors = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
        let _num_fat_sectors = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
        let first_dir_sector = u32::from_le_bytes([data[48], data[49], data[50], data[51]]);
        let _mini_stream_cutoff = u32::from_le_bytes([data[56], data[57], data[58], data[59]]);
        let first_minifat_sector = u32::from_le_bytes([data[60], data[61], data[62], data[63]]);
        let num_minifat_sectors = u32::from_le_bytes([data[64], data[65], data[66], data[67]]);

        // Parse DIFAT
        let mut difat = Vec::new();
        for i in 0..DIFAT_IN_HEADER {
            let offset = 76 + i * 4;
            if offset + 4 > data.len() {
                break;
            }
            let entry = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
            if entry != FREESECT && entry != ENDOFCHAIN {
                difat.push(entry);
            }
        }

        // Parse DIFAT sectors if present
        let num_difat_sectors = u32::from_le_bytes([data[68], data[69], data[70], data[71]]);
        let mut difat_sector = u32::from_le_bytes([data[72], data[73], data[74], data[75]]);
        let mut visited_difat = HashSet::new();

        for _ in 0..num_difat_sectors {
            if difat_sector == ENDOFCHAIN || !visited_difat.insert(difat_sector) {
                break;
            }
            if difat_sector >= (data.len() / SECTOR_SIZE) as u32 {
                break;
            }

            let difat_offset = HEADER_SIZE + difat_sector as usize * SECTOR_SIZE;
            if difat_offset + SECTOR_SIZE > data.len() {
                break;
            }

            for i in 0..(SECTOR_SIZE / 4 - 1) {
                let offset = difat_offset + i * 4;
                let entry = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
                if entry != FREESECT && entry != ENDOFCHAIN {
                    difat.push(entry);
                }
            }

            difat_sector = u32::from_le_bytes([
                data[difat_offset + SECTOR_SIZE - 4],
                data[difat_offset + SECTOR_SIZE - 3],
                data[difat_offset + SECTOR_SIZE - 2],
                data[difat_offset + SECTOR_SIZE - 1],
            ]);
        }

        // Read FAT sectors using DIFAT
        let mut fat = Vec::new();
        for &fat_sector in &difat {
            if fat_sector == FREESECT || fat_sector == ENDOFCHAIN {
                continue;
            }

            let fat_offset = HEADER_SIZE + fat_sector as usize * SECTOR_SIZE;
            if fat_offset + SECTOR_SIZE > data.len() {
                continue;
            }

            for i in 0..(SECTOR_SIZE / 4) {
                let offset = fat_offset + i * 4;
                if offset + 4 > data.len() {
                    break;
                }
                let entry = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
                fat.push(entry);
            }
        }

        // Read directory sectors (cycle-safe)
        let mut directory_data = Vec::new();
        let mut dir_sector = first_dir_sector;
        let mut visited_dir = HashSet::new();
        let mut dir_hops = 0usize;

        loop {
            if dir_sector == ENDOFCHAIN || dir_sector >= fat.len() as u32 {
                break;
            }
            if !visited_dir.insert(dir_sector) {
                anyhow::bail!("Cyclic FAT chain detected while reading CFB directory");
            }
            dir_hops += 1;
            if dir_hops > crate::limits::MAX_CFB_SECTOR_HOPS {
                anyhow::bail!("CFB directory chain exceeded sector hop limit");
            }

            let dir_offset = HEADER_SIZE + dir_sector as usize * SECTOR_SIZE;
            if dir_offset + SECTOR_SIZE > data.len() {
                break;
            }

            directory_data.extend_from_slice(&data[dir_offset..dir_offset + SECTOR_SIZE]);
            dir_sector = fat[dir_sector as usize];
        }

        // Parse directory entries
        let mut directory = Vec::new();
        let num_entries = directory_data.len() / 128;

        for i in 0..num_entries {
            let entry_offset = i * 128;
            if entry_offset + 128 > directory_data.len() {
                break;
            }

            let entry = Self::parse_directory_entry(&directory_data[entry_offset..entry_offset + 128])?;
            directory.push(entry);
        }

        // Read mini-FAT (cycle-safe)
        let mut mini_fat = Vec::new();
        let mut minifat_sector = first_minifat_sector;
        let mut visited_minifat = HashSet::new();

        for _ in 0..num_minifat_sectors {
            if minifat_sector == ENDOFCHAIN || minifat_sector >= fat.len() as u32 {
                break;
            }
            if !visited_minifat.insert(minifat_sector) {
                anyhow::bail!("Cyclic FAT chain detected while reading mini-FAT");
            }

            let minifat_offset = HEADER_SIZE + minifat_sector as usize * SECTOR_SIZE;
            if minifat_offset + SECTOR_SIZE > data.len() {
                break;
            }

            for i in 0..(SECTOR_SIZE / 4) {
                let offset = minifat_offset + i * 4;
                if offset + 4 > data.len() {
                    break;
                }
                let entry = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
                mini_fat.push(entry);
            }

            minifat_sector = fat[minifat_sector as usize];
        }

        Ok(CfbReader {
            data,
            fat,
            difat,
            directory,
            first_dir_sector,
            first_minifat_sector,
            mini_fat,
        })
    }

    fn sector_bytes(&self, sector: u32) -> Option<&[u8]> {
        let offset = HEADER_SIZE + sector as usize * SECTOR_SIZE;
        if offset >= self.data.len() {
            return None;
        }
        let end = (offset + SECTOR_SIZE).min(self.data.len());
        Some(&self.data[offset..end])
    }

    /// Parse a single directory entry
    fn parse_directory_entry(data: &[u8]) -> anyhow::Result<DirectoryEntry> {
        let mut entry = DirectoryEntry::new();

        // Parse name (UTF-16LE, max 31 chars + null)
        let mut name_len = u16::from_le_bytes([data[64], data[65]]) as usize;
        name_len = name_len.min(64); // Max 32 UTF-16 chars * 2 bytes

        let mut name_chars = Vec::new();
        for i in (0..name_len).step_by(2) {
            if i + 1 < data.len() {
                let unit = u16::from_le_bytes([data[i], data[i + 1]]);
                if unit == 0 {
                    break; // Null terminator
                }
                if let Some(c) = char::from_u32(unit as u32) {
                    name_chars.push(c);
                }
            }
        }
        entry.name = name_chars.into_iter().collect();

        // Object type
        entry.object_type = ObjectType::from_u8(data[66]);

        // Tree structure
        entry.left = u32::from_le_bytes([data[68], data[69], data[70], data[71]]);
        entry.right = u32::from_le_bytes([data[72], data[73], data[74], data[75]]);
        entry.child = u32::from_le_bytes([data[76], data[77], data[78], data[79]]);

        // Start sector and size
        entry.start_sector = u32::from_le_bytes([data[116], data[117], data[118], data[119]]);
        entry.size = u64::from_le_bytes([
            data[120], data[121], data[122], data[123],
            data[124], data[125], data[126], data[127],
        ]);

        Ok(entry)
    }

    /// Find directory entry by name (case-insensitive)
    fn find_entry(&self, name: &str) -> Option<&DirectoryEntry> {
        self.directory.iter().find(|e| {
            e.name.eq_ignore_ascii_case(name)
        })
    }

    /// Read a stream chain using FAT
    fn read_stream_chain(&self, start_sector: u32, size: u64) -> anyhow::Result<Vec<u8>> {
        let mut result = Vec::with_capacity((size as usize).min(crate::limits::MAX_ZIP_ENTRY_BYTES as usize));
        let mut sector = start_sector;
        let mut remaining = size as usize;
        let mut visited = HashSet::new();
        let mut hops = 0usize;

        loop {
            if sector == ENDOFCHAIN || sector >= self.fat.len() as u32 {
                break;
            }
            if !visited.insert(sector) {
                anyhow::bail!("Cyclic FAT chain detected while reading CFB stream");
            }
            hops += 1;
            if hops > crate::limits::MAX_CFB_SECTOR_HOPS {
                anyhow::bail!("CFB stream chain exceeded sector hop limit");
            }

            let Some(sector_data) = self.sector_bytes(sector) else {
                break;
            };
            let bytes_to_copy = sector_data.len().min(remaining);
            result.extend_from_slice(&sector_data[..bytes_to_copy]);

            remaining = remaining.saturating_sub(bytes_to_copy);
            if remaining == 0 {
                break;
            }

            sector = self.fat[sector as usize];
        }

        Ok(result)
    }

    /// Read mini-stream chain using mini-FAT
    fn read_mini_stream_chain(&self, start_sector: u32, size: u64) -> anyhow::Result<Option<Vec<u8>>> {
        let Some(root_entry) = self.directory.get(0).filter(|e| e.object_type == ObjectType::Root) else {
            return Ok(None);
        };
        let mini_stream_data = self.read_stream_chain(root_entry.start_sector, root_entry.size)?;

        let mut result = Vec::with_capacity(size as usize);
        let mut sector = start_sector;
        let mut remaining = size as usize;
        let mut visited = HashSet::new();
        let mut hops = 0usize;

        loop {
            if sector == ENDOFCHAIN || sector >= self.mini_fat.len() as u32 {
                break;
            }
            if !visited.insert(sector) {
                anyhow::bail!("Cyclic mini-FAT chain detected while reading CFB stream");
            }
            hops += 1;
            if hops > crate::limits::MAX_CFB_SECTOR_HOPS {
                anyhow::bail!("CFB mini-stream chain exceeded sector hop limit");
            }

            let offset = sector as usize * MINI_SECTOR_SIZE;
            if offset + MINI_SECTOR_SIZE > mini_stream_data.len() {
                break;
            }

            let bytes_to_copy = MINI_SECTOR_SIZE.min(remaining);
            result.extend_from_slice(&mini_stream_data[offset..offset + bytes_to_copy]);

            remaining = remaining.saturating_sub(bytes_to_copy);
            if remaining == 0 {
                break;
            }

            sector = self.mini_fat[sector as usize];
        }

        Ok(Some(result))
    }

    /// Get stream data by name
    pub fn get_stream(&self, name: &str) -> Option<Vec<u8>> {
        let entry = self.find_entry(name)?;

        if entry.object_type != ObjectType::Stream {
            return None;
        }

        if entry.size == 0 {
            return Some(Vec::new());
        }

        if entry.size < MINI_CUTOFF as u64 {
            self.read_mini_stream_chain(entry.start_sector, entry.size).ok()?
        } else {
            self.read_stream_chain(entry.start_sector, entry.size).ok()
        }
    }

    /// Get list of stream names
    pub fn list_streams(&self) -> Vec<String> {
        self.directory.iter()
            .filter(|e| e.object_type == ObjectType::Stream)
            .map(|e| e.name.clone())
            .collect()
    }

    /// Get list of all entry names
    pub fn list_entries(&self) -> Vec<String> {
        self.directory.iter()
            .map(|e| e.name.clone())
            .collect()
    }
}

const HEADER_SIZE: usize = 512;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfb_magic_detection() {
        let mut data = vec![0u8; 512];
        data[0..8].copy_from_slice(&CFB_MAGIC);
        // Set sector shift to 9 for 512-byte sectors (2^9 = 512)
        data[30..32].copy_from_slice(&9u16.to_le_bytes());

        let reader = CfbReader::parse(data);
        if let Err(e) = &reader {
            println!("CFB parse error: {:?}", e);
        }
        assert!(reader.is_ok());
    }

    #[test]
    fn test_invalid_cfb_magic() {
        let data = vec![0u8; 512];

        let reader = CfbReader::parse(data);
        assert!(reader.is_err());
    }

    #[test]
    fn test_object_type_conversion() {
        assert_eq!(ObjectType::from_u8(1), ObjectType::Storage);
        assert_eq!(ObjectType::from_u8(2), ObjectType::Stream);
        assert_eq!(ObjectType::from_u8(5), ObjectType::Root);
        assert_eq!(ObjectType::from_u8(0), ObjectType::Unknown);
        assert_eq!(ObjectType::from_u8(99), ObjectType::Unknown);
    }

    #[test]
    fn test_round_trip_with_writer() {
        use crate::excel::xls_writer::{XlsWriter, RowData};

        // Create a simple workbook
        let mut writer = XlsWriter::new();
        writer.add_sheet("Test").unwrap();
        let mut row = RowData::new();
        row.add_string("Hello");
        row.add_number(42.0);
        writer.add_row(row);

        let bytes = writer.to_bytes().unwrap();

        // Read it back with CFB parser
        let cfb = CfbReader::parse(bytes).unwrap();

        // Check streams exist
        let streams = cfb.list_streams();
        assert!(streams.iter().any(|s| s.eq_ignore_ascii_case("Workbook")));

        // Get workbook data
        let workbook_data = cfb.get_stream("Workbook");
        assert!(workbook_data.is_some());
        assert!(!workbook_data.unwrap().is_empty());
    }

    #[test]
    fn test_cfb_stream_list() {
        use crate::excel::xls_writer::{XlsWriter, RowData};

        let mut writer = XlsWriter::new();
        writer.add_sheet("Sheet1").unwrap();
        let mut row = RowData::new();
        row.add_string("Data");
        writer.add_row(row);

        let bytes = writer.to_bytes().unwrap();
        let cfb = CfbReader::parse(bytes).unwrap();

        let entries = cfb.list_entries();
        assert!(entries.iter().any(|e| e.contains("Root")));

        let streams = cfb.list_streams();
        assert!(streams.iter().any(|s| s.eq_ignore_ascii_case("Workbook")));
    }
}
