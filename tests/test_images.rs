//! Integration tests for image embedding (write + read round-trip).
//!
//! Verifies that `XlsxWriter::insert_image` produces valid XLSX files
//! and `XlsxReader::images` reads them back with correct anchor, format,
//! and byte content.

use xls_rs::{Image, ImageFormat, XlsxReader, XlsxWriter};
use zip::ZipArchive;

/// Minimal 1×1 PNG (transparent) — 67 bytes.
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
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC,
        0x33, 0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // "IEND"
        0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ]
}

/// Minimal 1×1 BMP (24-bit) — 58 bytes.
fn tiny_bmp() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"BM"); // signature
    data.extend_from_slice(&58u32.to_le_bytes()); // file size
    data.extend_from_slice(&[0u8; 4]); // reserved
    data.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset
    // DIB header (BITMAPINFOHEADER, 40 bytes)
    data.extend_from_slice(&40u32.to_le_bytes()); // header size
    data.extend_from_slice(&1u32.to_le_bytes()); // width
    data.extend_from_slice(&1u32.to_le_bytes()); // height
    data.extend_from_slice(&1u16.to_le_bytes()); // planes
    data.extend_from_slice(&24u16.to_le_bytes()); // bpp
    data.extend_from_slice(&0u32.to_le_bytes()); // compression
    data.extend_from_slice(&4u32.to_le_bytes()); // image size (1px * 3 + 1 padding)
    data.extend_from_slice(&[0u8; 16]); // x/y ppm, colors
    // Pixel data: 1 blue pixel + 1 byte padding
    data.extend_from_slice(&[0xFF, 0x00, 0x00, 0x00]);
    data
}

fn read_zip_part(zip_bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut za = ZipArchive::new(cursor).unwrap();
    let mut buf = Vec::new();
    use std::io::Read;
    za.by_name(name).ok()?.read_to_end(&mut buf).ok()?;
    Some(buf)
}

#[test]
fn test_image_write_creates_media_and_drawing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("images.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let img = Image::new(tiny_png(), 2, 1); // anchor at B3 (0-based: row=2, col=1)
    writer.insert_image(img);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    // Media file exists
    let media = read_zip_part(&bytes, "xl/media/image1.png").unwrap();
    assert_eq!(media, tiny_png());
    // Drawing XML exists and references the image
    let drawing = read_zip_part(&bytes, "xl/drawings/drawing1.xml").unwrap();
    let drawing_str = String::from_utf8_lossy(&drawing);
    assert!(drawing_str.contains("xdr:oneCellAnchor"));
    assert!(drawing_str.contains("r:embed"));
    // Drawing rels exist
    let rels = read_zip_part(&bytes, "xl/drawings/_rels/drawing1.xml.rels").unwrap();
    let rels_str = String::from_utf8_lossy(&rels);
    assert!(rels_str.contains("/image"));
    assert!(rels_str.contains("image1.png"));
    // Content types include image defaults
    let ct = read_zip_part(&bytes, "[Content_Types].xml").unwrap();
    let ct_str = String::from_utf8_lossy(&ct);
    assert!(ct_str.contains(r#"Extension="png""#));
    assert!(ct_str.contains(r#"Extension="jpeg""#));
    // Worksheet references the drawing
    let sheet = read_zip_part(&bytes, "xl/worksheets/sheet1.xml").unwrap();
    let sheet_str = String::from_utf8_lossy(&sheet);
    assert!(sheet_str.contains(r#"<drawing r:id="rId1"/>"#));
    // Worksheet rels reference the drawing
    let sheet_rels = read_zip_part(&bytes, "xl/worksheets/_rels/sheet1.xml.rels").unwrap();
    let sheet_rels_str = String::from_utf8_lossy(&sheet_rels);
    assert!(sheet_rels_str.contains("/drawing"));
}

#[test]
fn test_image_round_trip_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip.xlsx");

    let png = tiny_png();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(png.clone(), 2, 1));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].anchor_row, 2);
    assert_eq!(images[0].anchor_col, 1);
    assert_eq!(images[0].format, "png");
    assert_eq!(images[0].data, png);
}

#[test]
fn test_image_bmp_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bmp.xlsx");

    let bmp = tiny_bmp();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(bmp.clone(), 0, 0));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].anchor_row, 0);
    assert_eq!(images[0].anchor_col, 0);
    assert_eq!(images[0].format, "bmp");
    assert_eq!(images[0].data, bmp);
}

#[test]
fn test_multiple_images_same_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.xlsx");

    let png = tiny_png();
    let bmp = tiny_bmp();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(png.clone(), 0, 0));
    writer.insert_image(Image::new(bmp.clone(), 5, 3));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].anchor_row, 0);
    assert_eq!(images[0].anchor_col, 0);
    assert_eq!(images[0].format, "png");
    assert_eq!(images[0].data, png);
    assert_eq!(images[1].anchor_row, 5);
    assert_eq!(images[1].anchor_col, 3);
    assert_eq!(images[1].format, "bmp");
    assert_eq!(images[1].data, bmp);
}

#[test]
fn test_image_on_second_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("second.xlsx");

    let png = tiny_png();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("First").unwrap();
    writer.add_sheet("Second").unwrap();
    writer.insert_image(Image::new(png.clone(), 1, 2));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    // First sheet has no images
    assert!(reader.images(0).map(|i| i.is_empty()).unwrap_or(true));
    // Second sheet has one image
    let images = reader.images(1).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].anchor_row, 1);
    assert_eq!(images[0].anchor_col, 2);
    assert_eq!(images[0].data, png);
}

#[test]
fn test_image_with_explicit_emu_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("emu.xlsx");

    let png = tiny_png();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(png.clone(), 0, 0).with_emu(200000, 100000));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    let drawing = read_zip_part(&bytes, "xl/drawings/drawing1.xml").unwrap();
    let drawing_str = String::from_utf8_lossy(&drawing);
    assert!(drawing_str.contains(r#"cx="200000""#));
    assert!(drawing_str.contains(r#"cy="100000""#));
}

#[test]
fn test_image_with_data_preserves_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.xlsx");

    let png = tiny_png();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[
        vec!["Name".into(), "Value".into()],
        vec!["A".into(), "1".into()],
    ]);
    writer.insert_image(Image::new(png.clone(), 3, 0));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    // Cells are preserved
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.col_count(), 2);
    // Image is also present
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, png);
}

#[test]
fn test_image_format_detection() {
    assert_eq!(ImageFormat::from_bytes(&tiny_png()), Some(ImageFormat::Png));
    assert_eq!(ImageFormat::from_bytes(&tiny_bmp()), Some(ImageFormat::Bmp));
    assert_eq!(ImageFormat::from_bytes(b"not an image"), None);
}

#[test]
fn test_no_images_returns_empty_or_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("noimg.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[vec!["A".into(), "B".into()], vec!["1".into(), "2".into()]]);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert!(images.is_empty());
}

/// Minimal GIF89a: 1×1 pixel, transparent.
fn tiny_gif() -> Vec<u8> {
    vec![
        b'G', b'I', b'F', b'8', b'9', b'a', // signature
        0x01, 0x00, // width = 1 (LE)
        0x01, 0x00, // height = 1 (LE)
        0x80, 0x00, 0x00, // packed, bg, aspect
        // Global color table (2 entries)
        0x00, 0x00, 0x00, // black
        0xFF, 0xFF, 0xFF, // white
        // Image descriptor
        0x2C, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
        // Image data: LZW min code size = 2
        0x02, 0x02, 0x44, 0x01, 0x00, // compressed pixel
        // Trailer
        0x3B,
    ]
}

/// Minimal JPEG: SOI + APP0 (JFIF) + SOF0 + SOS + EOI.
/// This is a minimal valid JPEG structure with 1×1 dimensions.
fn tiny_jpeg() -> Vec<u8> {
    vec![
        0xFF, 0xD8, // SOI
        0xFF, 0xE0, 0x00, 0x10, // APP0 marker, length 16
        b'J', b'F', b'I', b'F', 0x00, // JFIF identifier
        0x01, 0x01, // version
        0x00, // units
        0x00, 0x01, 0x00, 0x01, // density
        0x00, 0x00, // thumbnail
        // SOF0 (baseline)
        0xFF, 0xC0, 0x00, 0x0B, // marker, length 11
        0x08, // precision
        0x00, 0x01, // height = 1
        0x00, 0x01, // width = 1
        0x01, // components
        0x01, 0x11, 0x00, // component 1
        // SOS
        0xFF, 0xDA, 0x00, 0x08, // marker, length 8
        0x01, // components
        0x01, 0x00, // component 1
        0x00, 0x3F, 0x00, // spectral, approx
        0x00, // scan data (empty)
        0xFF, 0xD9, // EOI
    ]
}

#[test]
fn test_image_jpeg_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jpeg.xlsx");

    let jpeg = tiny_jpeg();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(jpeg.clone(), 1, 1));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].anchor_row, 1);
    assert_eq!(images[0].anchor_col, 1);
    assert_eq!(images[0].format, "jpeg");
    assert_eq!(images[0].data, jpeg);
}

#[test]
fn test_image_gif_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gif.xlsx");

    let gif = tiny_gif();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(gif.clone(), 0, 0));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].anchor_row, 0);
    assert_eq!(images[0].anchor_col, 0);
    assert_eq!(images[0].format, "gif");
    assert_eq!(images[0].data, gif);
}

#[test]
fn test_image_chart_and_image_same_sheet() {
    use xls_rs::XlsxWriter;
    use xls_rs::excel::chart::{ChartConfig, DataChartType};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chart_image.xlsx");

    let png = tiny_png();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[
        vec!["X".into(), "Y".into()],
        vec!["1".into(), "10".into()],
        vec!["2".into(), "20".into()],
    ]);

    // Add a chart
    writer.set_chart(
        ChartConfig {
            chart_type: DataChartType::Column,
            title: Some("Chart+Image".into()),
            x_axis_title: Some("X".into()),
            y_axis_title: Some("Y".into()),
            category_column: 0,
            value_columns: vec![1],
            width: 600,
            height: 400,
            ..Default::default()
        },
        vec![vec!["1".into(), "10".into()], vec!["2".into(), "20".into()]],
    );

    // Also add an image on the same sheet
    writer.insert_image(Image::new(png.clone(), 10, 5));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    // Verify the XLSX is valid and readable
    let bytes = std::fs::read(&path).unwrap();
    let drawing = read_zip_part(&bytes, "xl/drawings/drawing1.xml").unwrap();
    let drawing_str = String::from_utf8_lossy(&drawing);
    // Drawing should contain both a chart graphicFrame and an image anchor
    assert!(drawing_str.contains("graphicFrame"), "missing chart anchor");
    assert!(
        drawing_str.contains("xdr:oneCellAnchor"),
        "missing image anchor"
    );
    assert!(
        drawing_str.contains(r#"r:embed="rId2""#),
        "image should be rId2"
    );

    // Drawing rels should have both chart and image relationships
    let rels = read_zip_part(&bytes, "xl/drawings/_rels/drawing1.xml.rels").unwrap();
    let rels_str = String::from_utf8_lossy(&rels);
    assert!(rels_str.contains("/chart"), "missing chart relationship");
    assert!(rels_str.contains("/image"), "missing image relationship");

    // Reader should find the image
    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].anchor_row, 10);
    assert_eq!(images[0].anchor_col, 5);
    assert_eq!(images[0].data, png);
}

#[test]
fn test_image_format_detection_all_formats() {
    assert_eq!(ImageFormat::from_bytes(&tiny_png()), Some(ImageFormat::Png));
    assert_eq!(
        ImageFormat::from_bytes(&tiny_jpeg()),
        Some(ImageFormat::Jpeg)
    );
    assert_eq!(ImageFormat::from_bytes(&tiny_gif()), Some(ImageFormat::Gif));
    assert_eq!(ImageFormat::from_bytes(&tiny_bmp()), Some(ImageFormat::Bmp));
}

#[test]
fn test_image_try_new_fallible() {
    // Valid PNG
    assert!(Image::try_new(tiny_png(), 0, 0).is_some());
    // Invalid bytes
    assert!(Image::try_new(vec![0u8; 100], 0, 0).is_none());
}

#[test]
fn test_image_mixed_formats_same_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.xlsx");

    let png = tiny_png();
    let bmp = tiny_bmp();
    let gif = tiny_gif();
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.insert_image(Image::new(png.clone(), 0, 0));
    writer.insert_image(Image::new(bmp.clone(), 1, 1));
    writer.insert_image(Image::new(gif.clone(), 2, 2));

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].format, "png");
    assert_eq!(images[0].data, png);
    assert_eq!(images[1].format, "bmp");
    assert_eq!(images[1].data, bmp);
    assert_eq!(images[2].format, "gif");
    assert_eq!(images[2].data, gif);
}
