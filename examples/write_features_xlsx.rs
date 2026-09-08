//! Create an XLSX workbook demonstrating the v0.1.17 feature set:
//! image embedding, rich text, worksheet/workbook protection, and
//! document properties.
//!
//! Run with: `cargo run --example write_features_xlsx`

use xls_rs::excel::{
    DocumentProperties, Image, RichTextRun, RichTextRunStyle, RowData, SheetProtection,
    WorkbookProtection, XlsxWriter,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut w = XlsxWriter::new();

    // ── Document properties ──────────────────────────────────────────
    w.set_document_properties(DocumentProperties {
        title: Some("Feature Demo".into()),
        creator: Some("xls-rs".into()),
        subject: Some("v0.1.17 capabilities".into()),
        description: Some("Images, rich text, protection, and properties.".into()),
        keywords: Some("images rich-text protection properties".into()),
        created: Some("2026-01-01T00:00:00Z".into()),
        ..Default::default()
    });

    // ── Workbook protection ────────────────────────────────────────────
    w.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        ..Default::default()
    });

    // ── Sheet 1: Rich text ─────────────────────────────────────────────
    w.add_sheet("Rich Text")?;

    let mut header = RowData::new();
    header.add_rich_text(vec![
        RichTextRun {
            text: "Sales".into(),
            style: RichTextRunStyle {
                bold: true,
                color: Some("FFFFFF".into()),
                font_size: Some(14.0),
                ..Default::default()
            },
        },
        RichTextRun {
            text: " Report".into(),
            style: RichTextRunStyle {
                italic: true,
                color: Some("CCCCCC".into()),
                ..Default::default()
            },
        },
    ]);
    w.add_row(header);

    let mut row = RowData::new();
    row.add_rich_text(vec![
        RichTextRun::plain("Q1: "),
        RichTextRun::bold("$1,200"),
        RichTextRun::plain("  Q2: "),
        RichTextRun::bold("$1,500"),
    ]);
    w.add_row(row);

    let mut row2 = RowData::new();
    row2.add_rich_text(vec![
        RichTextRun {
            text: "Important: ".into(),
            style: RichTextRunStyle {
                bold: true,
                color: Some("FF0000".into()),
                ..Default::default()
            },
        },
        RichTextRun {
            text: "review before submission".into(),
            style: RichTextRunStyle {
                underline: true,
                ..Default::default()
            },
        },
    ]);
    w.add_row(row2);

    // ── Sheet 2: Images ────────────────────────────────────────────────
    w.add_sheet("Images")?;

    // Generate a minimal 1×1 PNG in memory.
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // Insert at A1 (0-based: row=0, col=0) with auto-detected dimensions.
    w.insert_image(Image::new(png_bytes.clone(), 0, 0));

    // Insert a second image at B5 with explicit EMU dimensions.
    w.insert_image(Image::new(png_bytes, 4, 1).with_emu(500000, 500000));

    // ── Sheet 3: Protection ────────────────────────────────────────────
    w.add_sheet("Protected")?;
    w.add_data(&[
        vec!["Locked".into(), "Data".into()],
        vec!["A".into(), "42".into()],
    ]);
    // Lock everything except cell selection (data-entry mode).
    w.set_sheet_protection(SheetProtection::data_entry());

    let path = "/tmp/features_demo.xlsx";
    w.save(std::fs::File::create(path)?)?;
    println!("wrote {} ({} bytes)", path, std::fs::metadata(path)?.len());
    println!();
    println!("Features demonstrated:");
    println!("  - Document properties (docProps/core.xml)");
    println!("  - Workbook protection (lock structure)");
    println!("  - Rich text (bold/italic/underline/color/size runs)");
    println!("  - Image embedding (PNG, auto-detect + explicit EMU)");
    println!("  - Sheet protection (data-entry preset)");
    Ok(())
}
