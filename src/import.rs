//! Portable local extraction. No subprocesses, system applications, or network calls.
use image::{DynamicImage, ImageReader};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use quick_xml::{Reader, events::Event};
use std::{
    collections::HashMap,
    fs,
    io::{self, Cursor, Read},
    path::{Component, Path},
    sync::OnceLock,
};
use zip::ZipArchive;

pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 10 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 200 * 1024 * 1024;
const MAX_PAGES: usize = 500;
const FORMATS: &str =
    "PDF, DOCX, PPTX, ODT, ODP, images (PNG/JPEG/WebP/TIFF/BMP/GIF/PNM), or UTF-8 text";

fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}
fn failed(context: &str, e: impl std::fmt::Display) -> io::Error {
    error(format!("{context}: {e}"))
}

pub struct ExtractedSource {
    pub name: String,
    pub content: String,
}

pub fn extract(path: &Path) -> io::Result<ExtractedSource> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(error("Choose a file, not a directory"));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(error("Source exceeds the 100 MiB file limit"));
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(error("Source exceeds the 100 MiB file limit"));
    }
    let text = match ext.as_str() {
        "md" | "markdown" | "txt" | "rst" | "csv" | "tsv" | "json" | "yaml" | "yml" | "toml"
        | "log" | "tex" => {
            String::from_utf8(bytes).map_err(|_| error("Text files must be encoded as UTF-8"))?
        }
        "pdf" => pdf_text(&bytes)?,
        "docx" => office_text(&bytes, Office::Word)?,
        "pptx" => office_text(&bytes, Office::PowerPoint)?,
        "odt" | "odp" => office_text(&bytes, Office::OpenDocument)?,
        "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff" | "bmp" | "gif" | "pnm" | "ppm"
        | "pgm" | "pbm" => image_text(&bytes)?,
        "doc" | "ppt" => {
            return Err(error(
                "Legacy .doc/.ppt files are not supported yet. Save as .docx/.pptx or PDF, then import that file",
            ));
        }
        _ => {
            return Err(error(format!(
                "Unsupported source format. Import {FORMATS}"
            )));
        }
    };
    let content = clean_text(&text)?;
    Ok(ExtractedSource {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        content,
    })
}

fn clean_text(text: &str) -> io::Result<String> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(error("Extracted text exceeds the 10 MiB limit"));
    }
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let text: String = text
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect();
    if text.trim().is_empty() {
        return Err(error(
            "No readable text found. Image OCR reads printed text, not diagrams or handwriting",
        ));
    }
    Ok(text.trim().to_owned())
}

fn append_section(text: &mut String, title: &str, body: &str) -> io::Result<()> {
    text.push_str(&format!("## {title}\n\n{}\n\n", body.trim()));
    if text.len() > MAX_TEXT_BYTES {
        return Err(error("Extracted text exceeds the 10 MiB limit"));
    }
    Ok(())
}

static OCR: OnceLock<Result<OcrEngine, String>> = OnceLock::new();
fn ocr() -> io::Result<&'static OcrEngine> {
    OCR.get_or_init(|| {
        let detection =
            rten::Model::load_static_slice(include_bytes!("../assets/models/text-detection.onnx"))
                .map_err(|e| e.to_string())?;
        let recognition = rten::Model::load_static_slice(include_bytes!(
            "../assets/models/text-recognition.onnx"
        ))
        .map_err(|e| e.to_string())?;
        OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })
        .map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(|e| failed("Could not initialize bundled OCR", e))
}

fn recognize(image: DynamicImage) -> io::Result<String> {
    // Flatten transparency against white before OCR; avoid black backgrounds in cutouts.
    let image = if image.width() > 2600 || image.height() > 2600 {
        image.thumbnail(2600, 2600)
    } else {
        image
    };
    let mut rgba = image.into_rgba8();
    for p in rgba.pixels_mut() {
        let alpha = u16::from(p[3]);
        for channel in &mut p.0[..3] {
            *channel = ((u16::from(*channel) * alpha + 255 * (255 - alpha)) / 255) as u8;
        }
        p[3] = 255;
    }
    let rgb = DynamicImage::ImageRgba8(rgba).into_rgb8();
    let engine = ocr()?;
    let source = ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions())
        .map_err(|e| failed("Image input", e))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| failed("OCR preprocessing", e))?;
    engine
        .get_text(&input)
        .map_err(|e| failed("Text recognition", e))
}
fn image_text(bytes: &[u8]) -> io::Result<String> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(20000);
    limits.max_image_height = Some(20000);
    reader.limits(limits);
    recognize(
        reader
            .decode()
            .map_err(|e| failed("Cannot decode image", e))?,
    )
}

fn pdf_text(bytes: &[u8]) -> io::Result<String> {
    let mut doc =
        pdf_extract::Document::load_mem(bytes).map_err(|e| failed("Cannot read PDF", e))?;
    if doc.is_encrypted() {
        doc.decrypt("").map_err(|_| {
            error("Password-protected PDFs must be exported without a password before importing")
        })?;
    }
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(error("The PDF has no pages"));
    }
    if pages.len() > MAX_PAGES {
        return Err(error(
            "PDF exceeds the 500-page import limit; split it into smaller documents",
        ));
    }
    let rendered_pdf = std::cell::OnceCell::new();
    let cache = hayro::RenderCache::new();
    let mut result = String::new();
    let mut readable = false;
    for (index, page_num) in pages.keys().enumerate() {
        let mut text = String::new();
        let extracted = pdf_extract::output_doc_page(
            &doc,
            &mut pdf_extract::PlainTextOutput::new(&mut text),
            *page_num,
        );
        if extracted.is_err() || text.trim().is_empty() {
            if rendered_pdf.get().is_none() {
                let _ = rendered_pdf.set(
                    hayro::hayro_syntax::Pdf::new(bytes.to_vec())
                        .map_err(|e| failed("Cannot render scanned PDF", format!("{e:?}")))?,
                );
            }
            let pdf = rendered_pdf.get().unwrap();
            let page = pdf
                .pages()
                .get(index)
                .ok_or_else(|| error("PDF page could not be rendered"))?;
            let (width, height) = page.render_dimensions();
            if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
                return Err(error("PDF page has invalid dimensions"));
            }
            let scale = (2200.0 / width.max(height)).min(3.0);
            let settings = hayro::PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: hayro::vello_cpu::color::palette::css::WHITE,
            };
            let bitmap = hayro::render(
                page,
                &cache,
                &Default::default(),
                &Default::default(),
                &settings,
            );
            let rgba = image::RgbaImage::from_raw(
                u32::from(bitmap.width()),
                u32::from(bitmap.height()),
                bitmap.data_as_u8_slice().to_vec(),
            )
            .ok_or_else(|| error("PDF bitmap dimensions are invalid"))?;
            text = recognize(DynamicImage::ImageRgba8(rgba))?;
        }
        readable |= !text.trim().is_empty();
        append_section(&mut result, &format!("Page {page_num}"), &text)?;
    }
    if !readable {
        return Err(error(
            "No readable text found in the PDF, including after OCR",
        ));
    }
    Ok(result)
}

#[derive(Clone, Copy)]
enum Office {
    Word,
    PowerPoint,
    OpenDocument,
}
fn archive(bytes: &[u8]) -> io::Result<ZipArchive<Cursor<&[u8]>>> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| failed("Invalid Office document", e))?;
    if archive.len() > 10000 {
        return Err(error("Document archive contains too many entries"));
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| failed("Document entry", e))?;
        total = total.saturating_add(entry.size());
        if total > MAX_ARCHIVE_BYTES {
            return Err(error("Expanded document exceeds the 200 MiB limit"));
        }
    }
    Ok(archive)
}
fn entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> io::Result<Vec<u8>> {
    let file = archive
        .by_name(name)
        .map_err(|e| failed(&format!("Missing document component {name}"), e))?;
    if file.size() > MAX_TEXT_BYTES as u64 {
        return Err(error("Document XML exceeds the 10 MiB limit"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_TEXT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(error("Document XML exceeds the 10 MiB limit"));
    }
    Ok(bytes)
}
fn attribute(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> io::Result<Option<String>> {
    for a in e.attributes() {
        let a = a.map_err(|e| failed("Invalid XML attribute", e))?;
        if a.key.as_ref() == key {
            return Ok(Some(
                a.unescape_value()
                    .map_err(|e| failed("XML attribute", e))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}
fn xml_text(bytes: &[u8], odf: bool) -> io::Result<String> {
    let mut reader = Reader::from_reader(bytes);
    let mut result = String::new();
    let mut text_depth = 0usize;
    let mut paragraph_depth = 0usize;
    let mut depth = 0usize;
    loop {
        match reader
            .read_event()
            .map_err(|e| failed("Invalid document XML", e))?
        {
            Event::Start(e) => {
                depth += 1;
                if depth > 1000 {
                    return Err(error("Document XML is too deeply nested"));
                }
                let name = e.local_name();
                if name.as_ref() == b"t" {
                    text_depth += 1;
                }
                if matches!(name.as_ref(), b"p" | b"h") {
                    paragraph_depth += 1;
                }
            }
            Event::Empty(e) => match e.local_name().as_ref() {
                b"tab" => result.push('\t'),
                b"br" | b"line-break" => result.push('\n'),
                b"s" if odf => {
                    let count = attribute(&e, b"text:c")?
                        .and_then(|n| n.parse::<usize>().ok())
                        .unwrap_or(1)
                        .min(1000);
                    result.extend(std::iter::repeat_n(' ', count));
                }
                _ => {}
            },
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                match e.local_name().as_ref() {
                    b"t" => text_depth = text_depth.saturating_sub(1),
                    b"p" | b"h" => {
                        paragraph_depth = paragraph_depth.saturating_sub(1);
                        result.push_str("\n\n");
                    }
                    b"tr" | b"table-row" => result.push('\n'),
                    b"tc" | b"table-cell" => result.push('\t'),
                    _ => {}
                }
            }
            Event::Text(e) if text_depth > 0 || (odf && paragraph_depth > 0) => {
                result.push_str(
                    &quick_xml::escape::unescape(
                        &e.xml_content().map_err(|e| failed("XML text", e))?,
                    )
                    .map_err(|e| failed("XML entities", e))?,
                );
            }
            Event::GeneralRef(e) if text_depth > 0 || (odf && paragraph_depth > 0) => {
                let entity = format!("&{};", e.decode().map_err(|e| failed("XML entity", e))?);
                result.push_str(
                    &quick_xml::escape::unescape(&entity).map_err(|e| failed("XML entity", e))?,
                );
            }
            Event::CData(e) if text_depth > 0 || (odf && paragraph_depth > 0) => {
                result.push_str(&e.decode().map_err(|e| failed("XML text", e))?)
            }
            Event::DocType(_) => return Err(error("Document XML with a DTD is unsupported")),
            Event::Eof => {
                if depth != 0 {
                    return Err(error("Document XML is truncated"));
                }
                break;
            }
            _ => {}
        }
        if result.len() > MAX_TEXT_BYTES {
            return Err(error("Extracted text exceeds the 10 MiB limit"));
        }
    }
    Ok(result)
}
fn relationships(bytes: &[u8]) -> io::Result<HashMap<String, String>> {
    let mut reader = Reader::from_reader(bytes);
    let mut result = HashMap::new();
    loop {
        match reader
            .read_event()
            .map_err(|e| failed("Document relationships", e))?
        {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"Relationship" => {
                if attribute(&e, b"TargetMode")?.as_deref() == Some("External") {
                    continue;
                }
                if let (Some(id), Some(target)) = (attribute(&e, b"Id")?, attribute(&e, b"Target")?)
                {
                    result.insert(id, target);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(result)
}
fn resolve(base: &str, target: &str) -> io::Result<String> {
    let path = if target.starts_with('/') {
        Path::new(target.trim_start_matches('/')).to_owned()
    } else {
        Path::new(base).join(target)
    };
    let mut parts = Vec::new();
    for c in path.components() {
        match c {
            Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            Component::ParentDir if !parts.is_empty() => {
                parts.pop();
            }
            Component::CurDir => {}
            _ => return Err(error("Invalid path inside document archive")),
        }
    }
    Ok(parts.join("/"))
}
fn office_text(bytes: &[u8], format: Office) -> io::Result<String> {
    let mut zip = archive(bytes)?;
    let mut result = String::new();
    match format {
        Office::Word => {
            let body = xml_text(&entry(&mut zip, "word/document.xml")?, false)?;
            result.push_str(&body);
            for (name, label) in [
                ("word/footnotes.xml", "Footnotes"),
                ("word/endnotes.xml", "Endnotes"),
            ] {
                if zip.index_for_name(name).is_some() {
                    append_section(
                        &mut result,
                        label,
                        &xml_text(&entry(&mut zip, name)?, false)?,
                    )?;
                }
            }
        }
        Office::OpenDocument => result = xml_text(&entry(&mut zip, "content.xml")?, true)?,
        Office::PowerPoint => {
            let relations = relationships(&entry(&mut zip, "ppt/_rels/presentation.xml.rels")?)?;
            let presentation = entry(&mut zip, "ppt/presentation.xml")?;
            let mut reader = Reader::from_reader(presentation.as_slice());
            let mut paths = Vec::new();
            loop {
                match reader
                    .read_event()
                    .map_err(|e| failed("Presentation XML", e))?
                {
                    Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"sldId" => {
                        let id = attribute(&e, b"r:id")?
                            .ok_or_else(|| error("Slide has no relationship ID"))?;
                        let target = relations
                            .get(&id)
                            .ok_or_else(|| error("Missing slide relationship"))?;
                        paths.push(resolve("ppt", target)?);
                    }
                    Event::Eof => break,
                    _ => {}
                }
            }
            if paths.len() > MAX_PAGES {
                return Err(error("Presentation exceeds the 500-slide limit"));
            }
            let mut readable = false;
            for (i, path) in paths.iter().enumerate() {
                let text = xml_text(&entry(&mut zip, path)?, false)?;
                readable |= !text.trim().is_empty();
                append_section(&mut result, &format!("Slide {}", i + 1), &text)?;
                let rel_path = format!(
                    "ppt/slides/_rels/{}.rels",
                    path.rsplit('/').next().unwrap_or_default()
                );
                if zip.index_for_name(&rel_path).is_some() {
                    let rel_xml = entry(&mut zip, &rel_path)?;
                    for target in relationships(&rel_xml)?
                        .values()
                        .filter(|v| v.contains("notesSlides/"))
                    {
                        let note_path = resolve("ppt/slides", target)?;
                        let notes = xml_text(&entry(&mut zip, &note_path)?, false)?;
                        readable |= !notes.trim().is_empty();
                        append_section(&mut result, &format!("Slide {} notes", i + 1), &notes)?;
                    }
                }
            }
            if !readable {
                return Err(error(
                    "Presentation contains no extractable text. Import image-only slides as images or PDF for OCR",
                ));
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn zip_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, text) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    pub fn word_fixture() -> Vec<u8> {
        zip_bytes(&[(
            "word/document.xml",
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Replication copies </w:t></w:r><w:r><w:t>data &amp; prevents loss.</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>λ 中文</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
        )])
    }
    #[test]
    fn docx_preserves_runs_paragraphs_tables_entities_and_unicode() {
        let text = office_text(&word_fixture(), Office::Word).unwrap();
        assert!(text.contains("Replication copies data & prevents loss."));
        assert!(text.contains("\n\n"));
        assert!(text.contains("λ 中文"));
    }
    #[test]
    fn powerpoint_preserves_presentation_order_and_notes() {
        let bytes = zip_bytes(&[
            (
                "ppt/presentation.xml",
                r#"<p:presentation xmlns:p="p" xmlns:r="r"><p:sldIdLst><p:sldId id="1" r:id="rSecond"/><p:sldId id="2" r:id="rFirst"/></p:sldIdLst></p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                r#"<Relationships><Relationship Id="rFirst" Target="slides/slide1.xml"/><Relationship Id="rSecond" Target="slides/slide2.xml"/></Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                "<sld><p><t>Actually second</t></p></sld>",
            ),
            (
                "ppt/slides/slide2.xml",
                "<sld><p><t>Actually first</t></p></sld>",
            ),
            (
                "ppt/slides/_rels/slide2.xml.rels",
                r#"<Relationships><Relationship Id="notes" Target="../notesSlides/notesSlide2.xml"/></Relationships>"#,
            ),
            (
                "ppt/notesSlides/notesSlide2.xml",
                "<notes><p><t>Speaker notes about replication</t></p></notes>",
            ),
        ]);
        let text = office_text(&bytes, Office::PowerPoint).unwrap();
        assert!(text.find("Actually first").unwrap() < text.find("Actually second").unwrap());
        assert!(text.contains("## Slide 1 notes\n\nSpeaker notes about replication"));
        assert!(text.contains("## Slide 2\n\nActually second"));
    }
    #[test]
    fn open_document_preserves_text_paragraphs_and_explicit_spaces() {
        let bytes = zip_bytes(&[(
            "content.xml",
            r#"<office:document xmlns:office="o" xmlns:text="t"><text:h>Biology</text:h><text:p>Cells<text:s text:c="3"/>have <text:span>membranes &amp; nuclei</text:span>.</text:p></office:document>"#,
        )]);
        let text = office_text(&bytes, Office::OpenDocument).unwrap();
        assert!(text.contains("Biology\n\nCells   have membranes & nuclei."));
    }
    fn make_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
        let mut data = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0];
        for (i, object) in objects.iter().enumerate() {
            offsets.push(data.len());
            writeln!(data, "{} 0 obj", i + 1).unwrap();
            data.extend_from_slice(object);
            data.extend_from_slice(b"\nendobj\n");
        }
        let xref = data.len();
        write!(data, "xref\n0 {}\n0000000000 65535 f \n", offsets.len()).unwrap();
        for offset in offsets.iter().skip(1) {
            writeln!(data, "{offset:010} 00000 n ").unwrap();
        }
        write!(
            data,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .unwrap();
        data
    }
    fn stream(dictionary: &str, content: &[u8]) -> Vec<u8> {
        let mut data =
            format!("<< {dictionary} /Length {} >>\nstream\n", content.len()).into_bytes();
        data.extend_from_slice(content);
        data.extend_from_slice(b"\nendstream");
        data
    }
    pub fn text_pdf() -> Vec<u8> {
        make_pdf(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 700 180] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),
            stream("", b"BT /F1 26 Tf 30 100 Td (Replication copies data across nodes.) Tj ET"),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        ])
    }
    #[test]
    fn pdf_text_keeps_page_labels() {
        let text = pdf_text(&text_pdf()).unwrap();
        assert!(text.contains("## Page 1"));
        assert!(text.contains("Replication copies data across nodes."));
    }
    #[test]
    fn bundled_ocr_reads_images_and_scanned_pdfs_without_external_programs() {
        let pdf = hayro::hayro_syntax::Pdf::new(text_pdf()).unwrap();
        let bitmap = hayro::render(
            &pdf.pages()[0],
            &hayro::RenderCache::new(),
            &Default::default(),
            &Default::default(),
            &hayro::PixmapSettings {
                x_scale: 2.0,
                y_scale: 2.0,
                bg_color: hayro::vello_cpu::color::palette::css::WHITE,
            },
        );
        let rgba = image::RgbaImage::from_raw(
            u32::from(bitmap.width()),
            u32::from(bitmap.height()),
            bitmap.data_as_u8_slice().to_vec(),
        )
        .unwrap();
        let image = DynamicImage::ImageRgba8(rgba);
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let text = image_text(png.get_ref()).unwrap();
        assert!(
            text.to_lowercase().contains("replication copies data"),
            "{text}"
        );
        let mut jpeg = Cursor::new(Vec::new());
        image
            .into_rgb8()
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        let scanned = make_pdf(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 700 180] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>".to_vec(),
            stream("", b"q 700 0 0 180 0 0 cm /Im1 Do Q"),
            stream("/Type /XObject /Subtype /Image /Width 1400 /Height 360 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode", jpeg.get_ref()),
        ]);
        let text = pdf_text(&scanned).unwrap();
        assert!(text.contains("## Page 1"));
        assert!(
            text.to_lowercase().contains("replication copies data"),
            "{text}"
        );
    }
    #[test]
    fn malformed_empty_unsupported_and_oversized_content_is_rejected() {
        assert!(office_text(b"broken zip", Office::Word).is_err());
        assert!(office_text(&zip_bytes(&[]), Office::Word).is_err());
        assert!(pdf_text(b"not a PDF").is_err());
        assert!(image_text(b"not an image").is_err());
        assert!(clean_text("\u{feff} \n\t").is_err());
        assert!(clean_text(&"x".repeat(MAX_TEXT_BYTES + 1)).is_err());
        assert!(xml_text(b"<w:t>not closed", false).is_err());
        assert!(resolve("ppt", "../../escape").is_err());
    }
}
