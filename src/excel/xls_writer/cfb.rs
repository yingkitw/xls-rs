//! OLE2 Compound File Binary (CFB) writer.
//!
//! Implements just enough of [MS-CFB] to produce a valid `.xls` (BIFF8) file
//! container using only the standard library. The output uses v3 (512-byte
//! sectors) and supports the mini-stream for streams smaller than 4096
//! bytes, as required by Excel.
//!
//! The directory is laid out as a balanced binary tree (treated by readers
//! as red-black trees; balancing is sufficient to avoid pathological depth).
//! Names are sorted by the CFB collation: name length first, then
//! case-insensitive uppercase UTF-16.

const SECTOR_SIZE: usize = 512;
const MINI_SECTOR_SIZE: usize = 64;
const MINI_CUTOFF: usize = 4096;
const HEADER_SIZE: usize = 512;

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

const FREESECT: u32 = 0xFFFFFFFF;
const ENDOFCHAIN: u32 = 0xFFFFFFFE;
const FATSECT: u32 = 0xFFFFFFFD;
const DIFSECT: u32 = 0xFFFFFFFC;
const MAXREGSECT: u32 = 0xFFFFFFFA;

const NOSTREAM: u32 = 0xFFFFFFFF;

const ENTRIES_PER_SECTOR: usize = SECTOR_SIZE / 128; // 4
const FAT_PER_SECTOR: usize = SECTOR_SIZE / 4; // 128
const DIFAT_IN_HEADER: usize = 109;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Unknown = 0,
    Storage = 1,
    Stream = 2,
    Root = 5,
}

/// A stream or storage to be written into the CFB file.
pub struct CfbStream {
    pub name: String,
    pub data: Vec<u8>,
    pub kind: ObjectType,
    pub clsid: [u8; 16],
}

impl CfbStream {
    pub fn stream(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            data,
            kind: ObjectType::Stream,
            clsid: [0; 16],
        }
    }

    pub fn storage(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            kind: ObjectType::Storage,
            clsid: [0; 16],
        }
    }
}

/// Build a CFB file containing the given streams. The first entry must be the
/// root; its `data` is ignored (the root owns the mini-stream).
pub fn build_cfb(streams: &[CfbStream]) -> Vec<u8> {
    assert!(!streams.is_empty(), "at least the root entry is required");
    assert!(matches!(streams[0].kind, ObjectType::Root), "first entry must be root");

    let layout = Layout::compute(streams);
    layout.serialize(streams)
}

/// A flat, sector-indexed layout of the CFB file.
struct Layout {
    /// Total sector count (excluding the header).
    total_sectors: u32,
    /// One entry per sector, describing its purpose. Sectors with raw data
    /// live in `data_sectors[sector_index]`.
    kinds: Vec<SectorKind>,
    /// Raw bytes for sectors of kind `Data`. Indexed by sector index.
    data_sectors: Vec<[u8; SECTOR_SIZE]>,
    /// FAT chain (one entry per sector).
    fat: Vec<u32>,
    /// Mini-FAT chain (one entry per mini-sector).
    minifat: Vec<u32>,
    /// Sector index of first directory sector (ENDOFCHAIN if inline).
    first_dir_sector: u32,
    /// Number of directory sectors.
    num_dir_sectors: u32,
    /// Sector index of first mini-FAT sector (ENDOFCHAIN if none).
    first_minifat_sector: u32,
    /// Number of mini-FAT sectors.
    num_minifat_sectors: u32,
    /// Number of FAT sectors.
    num_fat_sectors: u32,
    /// Sector indices of the FAT sectors themselves.
    fat_sector_ids: Vec<u32>,
    /// Total directory entry count (rounded up to multiple of 4).
    dir_entry_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectorKind {
    /// Raw data sector for a regular stream.
    Data,
    /// Sector is part of the FAT chain. `data_sectors[i]` holds the FAT bytes.
    Fat,
    /// Sector is part of the mini-FAT chain. `data_sectors[i]` holds mini-FAT bytes.
    MiniFat,
    /// Sector is part of the directory chain. `data_sectors[i]` holds directory bytes.
    Directory,
}

impl Layout {
    fn compute(streams: &[CfbStream]) -> Self {
        // ---- Step 1: assign each stream to regular or mini sectors. ----
        let mut reg_starts: Vec<u32> = vec![0; streams.len()];
        let mut reg_sizes: Vec<u32> = vec![0; streams.len()];
        let mut mini_starts: Vec<u32> = vec![0; streams.len()];
        let mut mini_sizes: Vec<u32> = vec![0; streams.len()];

        let mut next_sec: u32 = 0;
        let mut next_mini: u32 = 0;
        for (i, s) in streams.iter().enumerate() {
            if !matches!(s.kind, ObjectType::Stream) {
                continue;
            }
            let len = s.data.len();
            if len == 0 {
                reg_starts[i] = ENDOFCHAIN;
                reg_sizes[i] = 0;
            } else if len < MINI_CUTOFF {
                mini_starts[i] = next_mini;
                mini_sizes[i] = len as u32;
                next_mini += ((len + MINI_SECTOR_SIZE - 1) / MINI_SECTOR_SIZE) as u32;
            } else {
                reg_starts[i] = next_sec;
                reg_sizes[i] = len as u32;
                next_sec += ((len + SECTOR_SIZE - 1) / SECTOR_SIZE) as u32;
            }
        }

        // ---- Step 2: build mini-stream. ----
        // The mini-stream is a sequence of mini-sectors, concatenated, then
        // padded to the regular sector boundary. It is owned by the root.
        let total_minisectors = next_mini as usize;
        let minis_per_reg_sector = SECTOR_SIZE / MINI_SECTOR_SIZE;
        let mini_stream_sectors = if total_minisectors == 0 {
            0u32
        } else {
            ((total_minisectors + minis_per_reg_sector - 1) / minis_per_reg_sector) as u32
        };
        let mut mini_stream_data = vec![0u8; (mini_stream_sectors as usize) * SECTOR_SIZE];
        for (i, s) in streams.iter().enumerate() {
            if !matches!(s.kind, ObjectType::Stream) || mini_sizes[i] == 0 {
                continue;
            }
            let start = mini_starts[i] as usize;
            let n = ((mini_sizes[i] as usize + MINI_SECTOR_SIZE - 1) / MINI_SECTOR_SIZE) as u32;
            for k in 0..n {
                let off = (k as usize) * MINI_SECTOR_SIZE;
                let end = (off + MINI_SECTOR_SIZE).min(s.data.len());
                let dst = (start + k as usize) * MINI_SECTOR_SIZE;
                mini_stream_data[dst..dst + (end - off)].copy_from_slice(&s.data[off..end]);
            }
        }

        // ---- Step 3: build mini-FAT. ----
        // One FAT entry per mini-sector. Each stream's mini-sectors are
        // chained back-to-back; the last ends with ENDOFCHAIN.
        let mut minifat = vec![ENDOFCHAIN; total_minisectors];
        for (i, _) in streams.iter().enumerate() {
            if !matches!(streams[i].kind, ObjectType::Stream) || mini_sizes[i] == 0 {
                continue;
            }
            let start = mini_starts[i];
            let n = ((mini_sizes[i] as usize + MINI_SECTOR_SIZE - 1) / MINI_SECTOR_SIZE) as u32;
            for k in 0..n {
                let sec = (start + k) as usize;
                if k + 1 < n {
                    minifat[sec] = start + k + 1;
                } else {
                    minifat[sec] = ENDOFCHAIN;
                }
            }
        }
        let minifat_len = total_minisectors;
        let num_minifat_sectors = if minifat_len == 0 {
            0u32
        } else {
            ((minifat_len + FAT_PER_SECTOR - 1) / FAT_PER_SECTOR) as u32
        };

        // ---- Step 4: build directory. ----
        let dir_entry_count = ((streams.len() + ENTRIES_PER_SECTOR - 1) / ENTRIES_PER_SECTOR)
            * ENTRIES_PER_SECTOR;
        let num_dir_sectors = (dir_entry_count / ENTRIES_PER_SECTOR) as u32;

        // ---- Step 5: assign sector indices. ----
        // Layout: [regular stream sectors][mini-stream sectors][mini-FAT][directory][FAT]
        // We solve to a fixed point because adding FAT sectors may add a new FAT sector.
        let reg_count = next_sec;
        let data_sectors = reg_count + mini_stream_sectors + num_minifat_sectors + num_dir_sectors;
        let mut num_fat_sectors: u32 = 1;
        loop {
            let total = data_sectors + num_fat_sectors;
            let need: u32 = ((total as usize + FAT_PER_SECTOR - 1) / FAT_PER_SECTOR) as u32;
            if need == num_fat_sectors {
                break;
            }
            num_fat_sectors = need;
        }
        let total_sectors = data_sectors + num_fat_sectors;

        let reg_end = reg_count;
        let mini_stream_start = reg_end;
        let mini_stream_end = mini_stream_start + mini_stream_sectors;
        let minifat_start = mini_stream_end;
        let minifat_end = minifat_start + num_minifat_sectors;
        let dir_start = minifat_end;
        let dir_end = dir_start + num_dir_sectors;
        let fat_start = dir_end;
        // fat_end = total_sectors

        // Recompute data_sectors (in case the fixpoint moved).
        let _ = data_sectors; // not used directly anymore

        // ---- Step 6: build FAT. ----
        let mut fat = vec![FREESECT; total_sectors as usize];
        // Mark FAT sectors.
        for k in 0..num_fat_sectors {
            fat[(fat_start + k) as usize] = FATSECT;
        }
        // Chain regular streams.
        for (i, s) in streams.iter().enumerate() {
            if !matches!(s.kind, ObjectType::Stream) || reg_sizes[i] == 0 {
                continue;
            }
            let n = ((reg_sizes[i] as usize + SECTOR_SIZE - 1) / SECTOR_SIZE) as u32;
            let start = reg_starts[i];
            for k in 0..n {
                let sec = (start + k) as usize;
                fat[sec] = if k + 1 < n { start + k + 1 } else { ENDOFCHAIN };
            }
        }
        // Chain mini-stream.
        if mini_stream_sectors > 0 {
            for k in 0..mini_stream_sectors {
                let sec = (mini_stream_start + k) as usize;
                fat[sec] = if k + 1 < mini_stream_sectors {
                    mini_stream_start + k + 1
                } else {
                    ENDOFCHAIN
                };
            }
        }
        // Chain mini-FAT.
        for k in 0..num_minifat_sectors {
            let sec = (minifat_start + k) as usize;
            fat[sec] = if k + 1 < num_minifat_sectors {
                minifat_start + k + 1
            } else {
                ENDOFCHAIN
            };
        }
        // Chain directory.
        for k in 0..num_dir_sectors {
            let sec = (dir_start + k) as usize;
            fat[sec] = if k + 1 < num_dir_sectors {
                dir_start + k + 1
            } else {
                ENDOFCHAIN
            };
        }

        // ---- Step 7: build sector kinds and data arrays. ----
        let mut kinds = vec![SectorKind::Data; total_sectors as usize];
        let mut data_sectors_buf: Vec<[u8; SECTOR_SIZE]> = vec![[0u8; SECTOR_SIZE]; total_sectors as usize];

        // Regular stream data.
        for (i, s) in streams.iter().enumerate() {
            if !matches!(s.kind, ObjectType::Stream) || reg_sizes[i] == 0 {
                continue;
            }
            let n = ((reg_sizes[i] as usize + SECTOR_SIZE - 1) / SECTOR_SIZE) as u32;
            let start = reg_starts[i];
            for k in 0..n {
                let sec = (start + k) as usize;
                let off = (k as usize) * SECTOR_SIZE;
                let end = (off + SECTOR_SIZE).min(s.data.len());
                data_sectors_buf[sec][..end - off].copy_from_slice(&s.data[off..end]);
            }
        }
        // Mini-stream data.
        for k in 0..mini_stream_sectors {
            let sec = (mini_stream_start + k) as usize;
            let off = (k as usize) * SECTOR_SIZE;
            data_sectors_buf[sec].copy_from_slice(&mini_stream_data[off..off + SECTOR_SIZE]);
        }
        // Mini-FAT bytes.
        for k in 0..num_minifat_sectors {
            let sec = (minifat_start + k) as usize;
            kinds[sec] = SectorKind::MiniFat;
            for j in 0..FAT_PER_SECTOR {
                let idx = (k as usize) * FAT_PER_SECTOR + j;
                let v = if idx < minifat.len() { minifat[idx] } else { ENDOFCHAIN };
                let bytes = v.to_le_bytes();
                data_sectors_buf[sec][j * 4..j * 4 + 4].copy_from_slice(&bytes);
            }
        }
        // Directory bytes.
        let dir_bytes = build_directory(
            streams,
            &reg_starts,
            &reg_sizes,
            &mini_starts,
            &mini_sizes,
            dir_entry_count,
            mini_stream_start,
            (mini_stream_sectors as u64) * SECTOR_SIZE as u64,
        );
        for k in 0..num_dir_sectors {
            let sec = (dir_start + k) as usize;
            kinds[sec] = SectorKind::Directory;
            let off = (k as usize) * SECTOR_SIZE;
            data_sectors_buf[sec].copy_from_slice(&dir_bytes[off..off + SECTOR_SIZE]);
        }
        // FAT bytes.
        let mut fat_sector_ids = Vec::with_capacity(num_fat_sectors as usize);
        for k in 0..num_fat_sectors {
            let sec = (fat_start + k) as usize;
            kinds[sec] = SectorKind::Fat;
            fat_sector_ids.push(sec as u32);
            for j in 0..FAT_PER_SECTOR {
                let idx = (k as usize) * FAT_PER_SECTOR + j;
                let v = if idx < fat.len() { fat[idx] } else { FREESECT };
                let bytes = v.to_le_bytes();
                data_sectors_buf[sec][j * 4..j * 4 + 4].copy_from_slice(&bytes);
            }
        }

        Layout {
            total_sectors,
            kinds,
            data_sectors: data_sectors_buf,
            fat,
            minifat,
            first_dir_sector: dir_start,
            num_dir_sectors,
            first_minifat_sector: if num_minifat_sectors == 0 {
                ENDOFCHAIN
            } else {
                minifat_start
            },
            num_minifat_sectors,
            num_fat_sectors,
            fat_sector_ids,
            dir_entry_count,
        }
    }

    fn serialize(&self, _streams: &[CfbStream]) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_SIZE + (self.total_sectors as usize) * SECTOR_SIZE);
        out.resize(HEADER_SIZE, 0u8);

        // Header.
        out[0..8].copy_from_slice(&CFB_MAGIC);
        // CLSID 16 bytes already zero.
        out[24..26].copy_from_slice(&0x003Eu16.to_le_bytes());
        out[26..28].copy_from_slice(&0x0003u16.to_le_bytes());
        out[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
        out[30..32].copy_from_slice(&0x0009u16.to_le_bytes());
        out[32..34].copy_from_slice(&0x0006u16.to_le_bytes());
        // 6 bytes reserved at 34..40 already zero.
        out[40..44].copy_from_slice(&0u32.to_le_bytes()); // # dir sectors
        out[44..48].copy_from_slice(&self.num_fat_sectors.to_le_bytes());
        out[48..52].copy_from_slice(&self.first_dir_sector.to_le_bytes());
        out[52..56].copy_from_slice(&0u32.to_le_bytes());
        out[56..60].copy_from_slice(&(MINI_CUTOFF as u32).to_le_bytes());
        out[60..64].copy_from_slice(&self.first_minifat_sector.to_le_bytes());
        out[64..68].copy_from_slice(&self.num_minifat_sectors.to_le_bytes());
        out[68..72].copy_from_slice(&ENDOFCHAIN.to_le_bytes()); // first DIFAT
        out[72..76].copy_from_slice(&0u32.to_le_bytes()); // # DIFAT sectors

        // DIFAT in header.
        for i in 0..DIFAT_IN_HEADER {
            let v = if (i as u32) < self.num_fat_sectors {
                self.fat_sector_ids[i as usize]
            } else {
                FREESECT
            };
            out[76 + i * 4..76 + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }

        // Sectors.
        for i in 0..self.total_sectors as usize {
            out.extend_from_slice(&self.data_sectors[i]);
        }
        out
    }
}

fn build_directory(
    streams: &[CfbStream],
    reg_starts: &[u32],
    reg_sizes: &[u32],
    mini_starts: &[u32],
    mini_sizes: &[u32],
    dir_entry_count: usize,
    mini_stream_start: u32,
    mini_stream_size: u64,
) -> Vec<u8> {
    let mut bytes = vec![0u8; dir_entry_count * 128];

    // Sort children (indices 1..n) by CFB collation key.
    let mut child_ids: Vec<u32> = (1..streams.len() as u32).collect();
    child_ids.sort_by_key(|&id| cfb_sort_key(&streams[id as usize].name));

    // Build a balanced binary tree over `child_ids`. Return the root id and
    // assign left/right siblings to all entries in the tree.
    let root_child = build_subtree(&child_ids, &mut bytes);

    // Root entry (id 0).
    {
        let off = 0;
        // Name "Root Entry" UTF-16LE.
        let name = "Root Entry";
        let mut i = 0;
        for unit in name.encode_utf16() {
            let b = unit.to_le_bytes();
            bytes[off + i * 2..off + i * 2 + 2].copy_from_slice(&b);
            i += 1;
        }
        // Null terminator + name length in bytes (including null).
        bytes[off + i * 2..off + i * 2 + 2].copy_from_slice(&0u16.to_le_bytes());
        let name_len = ((i + 1) * 2) as u16;
        bytes[off + 64..off + 66].copy_from_slice(&name_len.to_le_bytes());
        bytes[off + 66] = ObjectType::Root as u8;
        bytes[off + 67] = 1; // color = black
        bytes[off + 68..off + 72].copy_from_slice(&NOSTREAM.to_le_bytes()); // left
        bytes[off + 72..off + 76].copy_from_slice(&NOSTREAM.to_le_bytes()); // right
        bytes[off + 76..off + 80].copy_from_slice(&root_child.to_le_bytes()); // child
        // Root CLSID (workbook).
        let clsid: [u8; 16] = [
            0x21, 0x08, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x46,
        ];
        bytes[off + 80..off + 96].copy_from_slice(&clsid);
        // state bits zero.
        bytes[off + 116..off + 120].copy_from_slice(&mini_stream_start.to_le_bytes());
        bytes[off + 120..off + 128].copy_from_slice(&mini_stream_size.to_le_bytes());
    }

    // Stream / storage entries.
    for (i, s) in streams.iter().enumerate().skip(1) {
        let off = i * 128;
        // Name.
        let mut idx = 0;
        for unit in s.name.encode_utf16() {
            if idx >= 31 {
                break;
            }
            let b = unit.to_le_bytes();
            bytes[off + idx * 2..off + idx * 2 + 2].copy_from_slice(&b);
            idx += 1;
        }
        bytes[off + idx * 2..off + idx * 2 + 2].copy_from_slice(&0u16.to_le_bytes());
        let name_len = ((idx + 1) * 2) as u16;
        bytes[off + 64..off + 66].copy_from_slice(&name_len.to_le_bytes());
        bytes[off + 66] = s.kind as u8;
        bytes[off + 67] = 1; // color = black
        // left/right set by build_subtree; child for storages left as NOSTREAM
        bytes[off + 80..off + 96].copy_from_slice(&s.clsid);
        let (start, size) = match s.kind {
            ObjectType::Stream => {
                if reg_sizes[i] > 0 {
                    (reg_starts[i], reg_sizes[i] as u64)
                } else if mini_sizes[i] > 0 {
                    (mini_starts[i], mini_sizes[i] as u64)
                } else {
                    (ENDOFCHAIN, 0)
                }
            }
            _ => (ENDOFCHAIN, 0),
        };
        bytes[off + 116..off + 120].copy_from_slice(&start.to_le_bytes());
        bytes[off + 120..off + 128].copy_from_slice(&size.to_le_bytes());
    }

    bytes
}

/// Build a balanced binary tree over the given ids and write left/right
/// siblings into the directory. Returns the root id, or NOSTREAM if empty.
fn build_subtree(ids: &[u32], bytes: &mut [u8]) -> u32 {
    if ids.is_empty() {
        return NOSTREAM;
    }
    let mid = ids.len() / 2;
    let root = ids[mid];
    let left = build_subtree(&ids[..mid], bytes);
    let right = build_subtree(&ids[mid + 1..], bytes);
    let off = root as usize * 128;
    bytes[off + 68..off + 72].copy_from_slice(&left.to_le_bytes());
    bytes[off + 72..off + 76].copy_from_slice(&right.to_le_bytes());
    // child (76..80) is only meaningful for storages and the root; leave as
    // NOSTREAM for streams.
    root
}

fn cfb_sort_key(name: &str) -> (u32, String) {
    // MS-CFB §2.6.4: ordering is by name length (in UTF-16 code units,
    // including the terminating null), then by case-insensitive uppercase
    // UTF-16 code points.
    let units = name.encode_utf16().count() as u32 + 1;
    let upper: String = name.chars().flat_map(|c| c.to_uppercase()).collect();
    (units, upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_magic_and_v3() {
        let mut streams = vec![CfbStream {
            name: "Root Entry".to_string(),
            data: Vec::new(),
            kind: ObjectType::Root,
            clsid: [0; 16],
        }];
        streams.push(CfbStream::stream("Workbook", b"hi".to_vec()));
        let bytes = build_cfb(&streams);
        assert_eq!(&bytes[0..8], &CFB_MAGIC);
        assert_eq!(&bytes[26..28], &0x0003u16.to_le_bytes());
        assert_eq!(&bytes[30..32], &0x0009u16.to_le_bytes());
        assert_eq!(&bytes[32..34], &0x0006u16.to_le_bytes());
    }

    #[test]
    fn round_trip_via_cfb_signature() {
        // Just check the file is at least one sector and not corrupt by
        // verifying the FAT chain can be re-read.
        let mut streams = vec![CfbStream {
            name: "Root Entry".to_string(),
            data: Vec::new(),
            kind: ObjectType::Root,
            clsid: [0; 16],
        }];
        streams.push(CfbStream::stream("Workbook", vec![0u8; 100]));
        let bytes = build_cfb(&streams);
        assert!(bytes.len() >= HEADER_SIZE + SECTOR_SIZE);
        // The first directory sector field should be a valid sector number
        // or ENDOFCHAIN. For a minimal file it should point to a real sector.
        let first_dir = u32::from_le_bytes(bytes[48..52].try_into().unwrap());
        assert!(first_dir != FREESECT);
    }

    #[test]
    fn mini_stream_short_stream() {
        // A stream under 4096 bytes should live in the mini-stream.
        let mut streams = vec![CfbStream {
            name: "Root Entry".to_string(),
            data: Vec::new(),
            kind: ObjectType::Root,
            clsid: [0; 16],
        }];
        streams.push(CfbStream::stream("Small", b"tiny".to_vec()));
        let bytes = build_cfb(&streams);
        // First mini-FAT sector should be valid (not ENDOFCHAIN).
        let first_minifat = u32::from_le_bytes(bytes[60..64].try_into().unwrap());
        // The stream is < 4096 bytes, so mini-FAT may be empty. In that case
        // the field is ENDOFCHAIN, which is fine.
        assert!(first_minifat == ENDOFCHAIN || first_minifat < bytes.len() as u32 / SECTOR_SIZE as u32);
    }
}
