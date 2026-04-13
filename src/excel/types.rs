use anyhow::Result;

/// Cell style configuration
#[derive(Debug, Clone, Default)]
pub struct CellStyle {
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Background color (hex without #, e.g., "4472C4")
    pub bg_color: Option<String>,
    /// Font color (hex without #)
    pub font_color: Option<String>,
    /// Font size
    pub font_size: Option<f64>,
    /// Border style
    pub border: bool,
    /// Horizontal alignment
    pub align: Option<String>,
    /// Number format (e.g., "0.00", "#,##0", "yyyy-mm-dd")
    pub number_format: Option<String>,
}

impl CellStyle {
    pub fn header() -> Self {
        Self {
            bold: true,
            bg_color: Some("4472C4".to_string()),
            font_color: Some("FFFFFF".to_string()),
            border: true,
            align: Some("center".to_string()),
            ..Default::default()
        }
    }

    /// Parse hex color string to RGB color
    pub fn parse_hex_color(hex: &str) -> Result<u32> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            anyhow::bail!("Invalid hex color: {}", hex);
        }
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        Ok(r as u32 * 0x10000 + g as u32 * 0x100 + b as u32)
    }
}

/// Options for styled Excel writing
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Sheet name
    pub sheet_name: Option<String>,
    /// Apply header styling to first row
    pub style_header: bool,
    /// Header style
    pub header_style: CellStyle,
    /// Column-specific styles (by index)
    pub column_styles: Option<std::collections::HashMap<usize, CellStyle>>,
    /// Freeze first row
    pub freeze_header: bool,
    /// Enable auto-filter
    pub auto_filter: bool,
    /// Auto-fit column widths
    pub auto_fit: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            sheet_name: None,
            style_header: true,
            header_style: CellStyle::header(),
            column_styles: None,
            freeze_header: false,
            auto_filter: false,
            auto_fit: true,
        }
    }
}
