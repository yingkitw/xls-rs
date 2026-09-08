//! Image embedding and unified drawing XML generation for XLSX files.
//!
//! Generates the `<xdr:wsDr>` drawing markup that anchors images (and,
//! when combined with `chart_xml`, charts) to worksheet cells. Also
//! writes image media bytes into `xl/media/` and the drawing
//! relationship file that links anchors to media parts.
//!
//! # EMU
//! Excel uses English Metric Units (EMU) for drawing sizes:
//! 1 inch = 914400 EMU, 1 pixel = 9525 EMU (at 96 DPI).

use anyhow::Result;
use std::io::{Seek, Write};
use zip::ZipWriter;
use zip::write::FileOptions;

use super::types::{Image, ImageFormat};

/// Pixels → EMU conversion factor (96 DPI).
const EMU_PER_PIXEL: u64 = 9525;

/// Parse pixel dimensions (width, height) from image header bytes.
///
/// Supports PNG, JPEG, GIF, and BMP — the four formats Excel accepts
/// for embedded images. Returns `None` if the header is malformed or
/// the format is unrecognized.
pub fn image_dimensions(bytes: &[u8], format: ImageFormat) -> Option<(u32, u32)> {
    match format {
        ImageFormat::Png => png_dimensions(bytes),
        ImageFormat::Jpeg => jpeg_dimensions(bytes),
        ImageFormat::Gif => gif_dimensions(bytes),
        ImageFormat::Bmp => bmp_dimensions(bytes),
    }
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // PNG: 8-byte signature, then IHDR chunk: 4-byte length, "IHDR",
    // 4-byte width, 4-byte height (big-endian).
    if bytes.len() < 24 {
        return None;
    }
    if &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((width, height))
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // JPEG: scan markers for SOFx (0xFFC0–0xFFCF, excluding 0xFFC4/C8/CC
    // which are DHT/JPG/DAC). SOFx carries 2-byte precision, 2-byte
    // height, 2-byte width (big-endian).
    let mut i = 2; // skip SOI marker (0xFFD8)
    while i + 8 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        // Standalone markers (no payload).
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if i + 3 >= bytes.len() {
            return None;
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        // SOFx markers: 0xFFC0–0xFFCF, excluding 0xC4 (DHT), 0xC8 (JPG), 0xCC (DAC).
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            if i + 8 >= bytes.len() {
                return None;
            }
            let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((width, height));
        }
        i += 2 + len;
    }
    None
}

fn gif_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // GIF: 6-byte signature, then 2-byte logical width, 2-byte logical
    // height (little-endian).
    if bytes.len() < 10 {
        return None;
    }
    let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
    let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
    Some((width, height))
}

fn bmp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // BMP: 2-byte "BM", then DIB header. The header starts at offset 14.
    // BITMAPINFOHEADER (40 bytes): 4-byte width, 4-byte height (LE, signed).
    if bytes.len() < 26 {
        return None;
    }
    let width = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]).unsigned_abs();
    let height = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]).unsigned_abs();
    Some((width, height))
}

/// Resolve the EMU dimensions for an image, using explicit overrides
/// when present or auto-detecting from the header otherwise.
fn resolve_emu(image: &Image) -> Option<(u64, u64)> {
    if let (Some(w), Some(h)) = (image.width_emu, image.height_emu) {
        return Some((w, h));
    }
    let (w_px, h_px) = image_dimensions(&image.data, image.format)?;
    let w = image.width_emu.unwrap_or(w_px as u64 * EMU_PER_PIXEL);
    let h = image.height_emu.unwrap_or(h_px as u64 * EMU_PER_PIXEL);
    Some((w, h))
}

/// Generate a single `<xdr:oneCellAnchor>` fragment for an image.
fn image_anchor_xml(anchor_id: usize, rid: &str, image: &Image) -> String {
    let (width, height) = resolve_emu(image).unwrap_or_default();
    let row = image.anchor_row;
    let col = image.anchor_col;
    format!(
        concat!(
            r#"<xdr:oneCellAnchor>"#,
            r#"<xdr:from><xdr:col>{col}</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>{row}</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"#,
            r#"<xdr:ext cx="{width}" cy="{height}"/>"#,
            r#"<xdr:pic>"#,
            r#"<xdr:nvPicPr><xdr:cNvPr id="{id}" name="Image {id}"/><xdr:cNvPicPr/></xdr:nvPicPr>"#,
            r#"<xdr:blipFill><a:blip xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>"#,
            r#"<xdr:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{width}" cy="{height}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr>"#,
            r#"</xdr:pic>"#,
            r#"<xdr:clientData/>"#,
            r#"</xdr:oneCellAnchor>"#,
        ),
        col = col,
        row = row,
        width = width,
        height = height,
        id = anchor_id,
        rid = rid,
    )
}

/// Generate the chart `<xdr:twoCellAnchor>` graphicFrame fragment.
/// Mirrors the legacy chart drawing so charts and images share one
/// `<xdr:wsDr>` document.
fn chart_anchor_xml(chart_rid: &str, width_emu: u64, height_emu: u64) -> String {
    format!(
        concat!(
            r#"<xdr:twoCellAnchor>"#,
            r#"<xdr:from><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"#,
            r#"<xdr:to><xdr:col>14</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>20</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>"#,
            r#"<xdr:graphicFrame macro="">"#,
            r#"<xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>"#,
            r#"<xdr:xfrm><a:off x="0" y="0"/><a:ext cx="{w}" cy="{h}"/></xdr:xfrm>"#,
            r#"<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">"#,
            r#"<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="{rid}"/>"#,
            r#"</a:graphicData></a:graphic>"#,
            r#"</xdr:graphicFrame>"#,
            r#"<xdr:clientData/>"#,
            r#"</xdr:twoCellAnchor>"#,
        ),
        w = width_emu,
        h = height_emu,
        rid = chart_rid,
    )
}

/// Generate the complete `<xdr:wsDr>` drawing XML for a sheet, combining
/// an optional chart anchor with any image anchors.
///
/// - `chart`: `Some((rid, width_emu, height_emu))` when the sheet has a
///   chart; the rid references the chart part in the drawing rels.
/// - `images`: images anchored on this sheet; each gets a sequential
///   `rIdN` matching the order in the drawing rels.
pub fn generate_drawing_xml(chart: Option<(&str, u64, u64)>, images: &[Image]) -> String {
    let mut xml = String::with_capacity(1024 + images.len() * 512);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);

    // Chart anchor first (rId1), then image anchors (rId2, rId3, ...).
    if let Some((rid, w, h)) = chart {
        xml.push_str(&chart_anchor_xml(rid, w, h));
    }
    let image_rid_start: usize = if chart.is_some() { 2 } else { 1 };
    for (i, image) in images.iter().enumerate() {
        let rid = format!("rId{}", image_rid_start + i);
        let anchor_id = i + 2; // cNvPr id — start at 2 (chart uses id=2)
        xml.push_str(&image_anchor_xml(anchor_id, &rid, image));
    }

    xml.push_str(r#"</xdr:wsDr>"#);
    xml
}

/// Write all drawing-related parts for a sheet into the ZIP archive:
///
/// 1. Image media files (`xl/media/imageN.ext`) — one per image.
/// 2. `xl/drawings/drawingN.xml` — the unified drawing (chart + images).
/// 3. `xl/drawings/_rels/drawingN.xml.rels` — relationships to chart and
///    image parts.
///
/// `chart_xml` is the pre-generated chart XML string (written
/// separately by the caller to `xl/charts/chartN.xml`); here we only
/// need to know whether a chart exists so we emit the chart
/// relationship.
pub fn add_drawing_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_idx: usize,
    chart: Option<(&str, u64, u64)>,
    images: &[Image],
) -> Result<()> {
    let n = sheet_idx + 1;
    let opts = || FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. Write image media files and collect their target paths.
    let mut media_paths: Vec<String> = Vec::with_capacity(images.len());
    for (i, image) in images.iter().enumerate() {
        let media_path = format!("xl/media/image{}.{}", i + 1, image.format.extension());
        zip.start_file(&media_path, opts())?;
        zip.write_all(&image.data)?;
        media_paths.push(media_path);
    }

    // 2. Unified drawing XML.
    let drawing_xml = generate_drawing_xml(chart, images);
    zip.start_file(format!("xl/drawings/drawing{}.xml", n), opts())?;
    zip.write_all(drawing_xml.as_bytes())?;

    // 3. Drawing rels — chart (rId1) then images (rId2, rId3, ...).
    let mut rels = String::with_capacity(256 + images.len() * 128);
    rels.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    rels.push_str(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    let mut rid = 1;
    if chart.is_some() {
        rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart{}.xml"/>"#,
            rid, n
        ));
        rid += 1;
    }
    for (i, _image) in images.iter().enumerate() {
        let media_path = &media_paths[i];
        // Target is relative to xl/drawings/, so strip the "xl/" prefix.
        let target = media_path.strip_prefix("xl/").unwrap_or(media_path);
        rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../{}"/>"#,
            rid, target
        ));
        rid += 1;
    }
    rels.push_str(r#"</Relationships>"#);
    zip.start_file(format!("xl/drawings/_rels/drawing{}.xml.rels", n), opts())?;
    zip.write_all(rels.as_bytes())?;

    Ok(())
}

/// Collect the set of image format extensions used across `images`,
/// for `[Content_Types].xml` `<Default>` entries.
pub fn image_extensions_used(images_by_sheet: &[&[Image]]) -> Vec<ImageFormat> {
    let mut seen = [false; 4];
    let mut out = Vec::new();
    for images in images_by_sheet {
        for img in images.iter() {
            let idx = match img.format {
                ImageFormat::Png => 0,
                ImageFormat::Jpeg => 1,
                ImageFormat::Gif => 2,
                ImageFormat::Bmp => 3,
            };
            if !seen[idx] {
                seen[idx] = true;
                out.push(img.format);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal 1×1 PNG (67 bytes).
    fn tiny_png() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            0x49, 0x48, 0x44, 0x52, // "IHDR"
            0x00, 0x00, 0x00, 0x01, // width = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, 0x02, 0x00, 0x00, 0x00, // bit depth, color type, etc.
            0x90, 0x77, 0x53, 0xDE, // CRC
            0x00, 0x00, 0x00, 0x0C, // IDAT length
            0x49, 0x44, 0x41, 0x54, // "IDAT"
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21,
            0xBC, 0x33, 0x00, 0x00, 0x00, 0x00, // IEND length
            0x49, 0x45, 0x4E, 0x44, // "IEND"
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ]
    }

    #[test]
    fn test_detect_png() {
        let data = tiny_png();
        assert_eq!(ImageFormat::from_bytes(&data), Some(ImageFormat::Png));
    }

    #[test]
    fn test_detect_jpeg() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(ImageFormat::from_bytes(&data), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_detect_gif() {
        assert_eq!(ImageFormat::from_bytes(b"GIF89a"), Some(ImageFormat::Gif));
        assert_eq!(ImageFormat::from_bytes(b"GIF87a"), Some(ImageFormat::Gif));
    }

    #[test]
    fn test_detect_bmp() {
        assert_eq!(
            ImageFormat::from_bytes(b"BM\x00\x00"),
            Some(ImageFormat::Bmp)
        );
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(ImageFormat::from_bytes(b"hello world"), None);
    }

    #[test]
    fn test_png_dimensions() {
        let data = tiny_png();
        let dims = image_dimensions(&data, ImageFormat::Png).unwrap();
        assert_eq!(dims, (1, 1));
    }

    #[test]
    fn test_gif_dimensions() {
        // GIF89a + logical screen descriptor: width=10, height=20 (LE)
        let mut data = b"GIF89a".to_vec();
        data.extend_from_slice(&10u16.to_le_bytes());
        data.extend_from_slice(&20u16.to_le_bytes());
        data.extend_from_slice(&[0x80, 0x00, 0x00]); // packed, bg, aspect
        let dims = image_dimensions(&data, ImageFormat::Gif).unwrap();
        assert_eq!(dims, (10, 20));
    }

    #[test]
    fn test_bmp_dimensions() {
        // Minimal BMP header with 40-byte BITMAPINFOHEADER, 3×2 image.
        let mut data = b"BM".to_vec();
        data.extend_from_slice(&[0u8; 12]); // file size + reserved + offset
        data.extend_from_slice(&40u32.to_le_bytes()); // header size
        data.extend_from_slice(&3u32.to_le_bytes()); // width
        data.extend_from_slice(&2u32.to_le_bytes()); // height
        let dims = image_dimensions(&data, ImageFormat::Bmp).unwrap();
        assert_eq!(dims, (3, 2));
    }

    #[test]
    fn test_image_new_auto_detects_png() {
        let data = tiny_png();
        let img = Image::new(data, 0, 0);
        assert_eq!(img.format, ImageFormat::Png);
    }

    #[test]
    fn test_image_try_new_rejects_unknown() {
        assert!(Image::try_new(b"not an image".to_vec(), 0, 0).is_none());
    }

    #[test]
    fn test_generate_drawing_xml_images_only() {
        let img = Image::new(tiny_png(), 2, 1);
        let xml = generate_drawing_xml(None, &[img]);
        assert!(xml.contains("xdr:wsDr"));
        assert!(xml.contains("xdr:oneCellAnchor"));
        assert!(xml.contains(r#"r:embed="rId1""#));
        assert!(xml.contains("<xdr:col>1</xdr:col>"));
        assert!(xml.contains("<xdr:row>2</xdr:row>"));
        assert!(!xml.contains("graphicFrame"));
    }

    #[test]
    fn test_generate_drawing_xml_chart_and_images() {
        let img = Image::new(tiny_png(), 0, 0);
        let xml = generate_drawing_xml(Some(("rId1", 5715000, 3810000)), &[img]);
        // Chart anchor first
        assert!(xml.contains("graphicFrame"));
        assert!(xml.contains(r#"r:id="rId1""#));
        // Image anchor second with rId2
        assert!(xml.contains(r#"r:embed="rId2""#));
        assert!(xml.contains("xdr:oneCellAnchor"));
    }

    #[test]
    fn test_image_extensions_used_dedup() {
        let png = Image::new(tiny_png(), 0, 0);
        let png2 = Image::new(tiny_png(), 1, 1);
        let sheet1 = [png];
        let sheet2 = [png2];
        let sheets: Vec<&[Image]> = vec![&sheet1, &sheet2];
        let exts = image_extensions_used(&sheets);
        assert_eq!(exts, vec![ImageFormat::Png]);
    }

    #[test]
    fn test_resolve_emu_auto_detect() {
        let img = Image::new(tiny_png(), 0, 0);
        let (w, h) = resolve_emu(&img).unwrap();
        assert_eq!(w, 9525); // 1 px
        assert_eq!(h, 9525);
    }

    #[test]
    fn test_resolve_emu_override() {
        let img = Image::new(tiny_png(), 0, 0).with_emu(100000, 200000);
        let (w, h) = resolve_emu(&img).unwrap();
        assert_eq!(w, 100000);
        assert_eq!(h, 200000);
    }
}
