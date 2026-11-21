//! Reading user-chosen files for chat: images (sent to vision models), PDFs and
//! Word documents (text extracted locally), and plain-text/code files.
//! Nothing here executes or interprets file contents.

use crate::error::{AppError, AppResult};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

/// Max characters of extracted text kept per file (~10k tokens).
pub const MAX_TEXT_CHARS: usize = 40_000;
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_DOC_BYTES: u64 = 30 * 1024 * 1024;
const MAX_TEXT_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Image,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub kind: AttachmentKind,
    pub mime: String,
    pub size_bytes: u64,
    /// Extracted text (Text kind).
    #[serde(default)]
    pub text: Option<String>,
    /// Base64 data (Image kind).
    #[serde(default)]
    pub data_base64: Option<String>,
    /// True if `text` was cut to MAX_TEXT_CHARS.
    #[serde(default)]
    pub truncated: bool,
}

const TEXT_EXTS: &[&str] = &[
    "txt", "md", "markdown", "csv", "tsv", "json", "jsonl", "xml", "html", "htm", "log", "yaml", "yml", "toml",
    "ini", "cfg", "conf", "py", "js", "mjs", "cjs", "ts", "tsx", "jsx", "rs", "go", "java", "kt", "c", "h", "cpp",
    "hpp", "cc", "cs", "rb", "php", "swift", "sh", "bash", "ps1", "bat", "sql", "css", "scss", "vue", "svelte",
    "r", "lua", "dart", "tex", "srt", "env.example",
];

fn ext_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

pub fn supported_extensions() -> Vec<&'static str> {
    let mut v = vec!["png", "jpg", "jpeg", "webp", "pdf", "docx"];
    v.extend_from_slice(TEXT_EXTS);
    v
}

/// Cut to `max` characters on a char boundary.
pub fn truncate_chars(s: &str, max: usize) -> (String, bool) {
    match s.char_indices().nth(max) {
        Some((i, _)) => (s[..i].to_string(), true),
        None => (s.to_string(), false),
    }
}

/// Collapse runs of blank lines / trailing spaces left by PDF extraction.
pub fn tidy_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank = 0;
    for line in s.lines() {
        let l = line.trim_end();
        if l.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(l);
        out.push('\n');
    }
    out.trim().to_string()
}

fn text_attachment(name: String, mime: &str, size: u64, raw: String) -> AppResult<Attachment> {
    let tidy = tidy_text(&raw);
    if tidy.is_empty() {
        return Err(AppError::Validation(format!("'{name}' contains no readable text")));
    }
    let (text, truncated) = truncate_chars(&tidy, MAX_TEXT_CHARS);
    Ok(Attachment { name, kind: AttachmentKind::Text, mime: mime.into(), size_bytes: size, text: Some(text), data_base64: None, truncated })
}

/// Extract the visible text from a .docx (word/document.xml paragraphs).
pub fn docx_text(bytes: &[u8]) -> AppResult<String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| AppError::Validation("This .docx file is damaged or not a Word document".into()))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|_| AppError::Validation("This .docx file has no document body".into()))?
        .take(50 * 1024 * 1024)
        .read_to_string(&mut xml)
        .map_err(|e| AppError::Validation(format!("Could not read .docx: {e}")))?;
    Ok(xml_paragraph_text(&xml))
}

fn xml_paragraph_text(xml: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let b = xml.as_bytes();
    while i < b.len() {
        if b[i] == b'<' {
            let end = xml[i..].find('>').map(|e| i + e).unwrap_or(b.len() - 1);
            let tag = &xml[i + 1..end];
            if tag.starts_with("/w:p") && !tag.starts_with("/w:pP") {
                out.push('\n');
            } else if tag.starts_with("w:tab") {
                out.push('\t');
            } else if tag.starts_with("w:br") {
                out.push('\n');
            }
            i = end + 1;
        } else {
            let end = xml[i..].find('<').map(|e| i + e).unwrap_or(b.len());
            out.push_str(&xml[i..end]);
            i = end;
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'")
}

fn pdf_text(bytes: Vec<u8>) -> AppResult<String> {
    // pdf-extract can panic on malformed files; contain it.
    std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem(&bytes))
        .map_err(|_| AppError::Validation("This PDF could not be read (it may be damaged)".into()))?
        .map_err(|e| AppError::Validation(format!("This PDF could not be read: {e}")))
}

pub fn read(path: &Path) -> AppResult<Attachment> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
    let meta = std::fs::metadata(path).map_err(|e| AppError::NotFound(format!("Cannot open '{name}': {e}")))?;
    if !meta.is_file() {
        return Err(AppError::Validation(format!("'{name}' is not a file")));
    }
    let size = meta.len();
    let ext = ext_of(path);
    let too_big = |max: u64| AppError::Validation(format!("'{name}' is too large (max {} MB)", max / 1024 / 1024));
    let read_all = || std::fs::read(path).map_err(|e| AppError::Internal(format!("Cannot read '{name}': {e}")));

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => {
            if size > MAX_IMAGE_BYTES {
                return Err(too_big(MAX_IMAGE_BYTES));
            }
            let mime = if ext == "png" { "image/png" } else if ext == "webp" { "image/webp" } else { "image/jpeg" };
            let data = base64::engine::general_purpose::STANDARD.encode(read_all()?);
            Ok(Attachment { name, kind: AttachmentKind::Image, mime: mime.into(), size_bytes: size, text: None, data_base64: Some(data), truncated: false })
        }
        "pdf" => {
            if size > MAX_DOC_BYTES {
                return Err(too_big(MAX_DOC_BYTES));
            }
            let raw = pdf_text(read_all()?)?;
            if raw.trim().is_empty() {
                return Err(AppError::Validation(format!(
                    "'{name}' has no selectable text (it is probably a scanned PDF). Try a screenshot of the page with a vision model instead."
                )));
            }
            text_attachment(name, "application/pdf", size, raw)
        }
        "docx" => {
            if size > MAX_DOC_BYTES {
                return Err(too_big(MAX_DOC_BYTES));
            }
            let raw = docx_text(&read_all()?)?;
            text_attachment(name, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", size, raw)
        }
        e if TEXT_EXTS.contains(&e) => {
            if size > MAX_TEXT_BYTES {
                return Err(too_big(MAX_TEXT_BYTES));
            }
            let bytes = read_all()?;
            if bytes.iter().take(8000).any(|&b| b == 0) {
                return Err(AppError::Validation(format!("'{name}' looks like a binary file, not text")));
            }
            text_attachment(name, "text/plain", size, String::from_utf8_lossy(&bytes).into_owned())
        }
        "doc" => Err(AppError::Validation("Old .doc files aren't supported. Save it as .docx or PDF first.".into())),
        "" => Err(AppError::Validation(format!("'{name}' has no file extension, so its type is unknown"))),
        other => Err(AppError::Validation(format!(".{other} files aren't supported yet"))),
    }
}

/// Format text attachments as a block prepended to the user's message.
pub fn as_prompt_context(atts: &[Attachment]) -> String {
    let mut s = String::new();
    for a in atts.iter().filter(|a| a.kind == AttachmentKind::Text) {
        if let Some(t) = &a.text {
            s.push_str(&format!("<file name=\"{}\">\n{}\n</file>\n", a.name.replace('"', "'"), t));
            if a.truncated {
                s.push_str(&format!("(Only the first part of {} was included because it is long.)\n", a.name));
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("baseio-att-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn reads_text_and_code() {
        let a = read(&tmp("notes.md", b"# Title\n\n\n\nbody  \n")).unwrap();
        assert_eq!(a.kind, AttachmentKind::Text);
        assert_eq!(a.text.as_deref(), Some("# Title\n\nbody"));
    }

    #[test]
    fn rejects_binary_and_unknown() {
        assert!(read(&tmp("x.txt", &[0, 1, 2, 0])).is_err());
        assert!(read(&tmp("x.exe", b"MZ")).is_err());
        assert!(read(&tmp("x.doc", b"..")).is_err());
    }

    #[test]
    fn image_is_base64() {
        let a = read(&tmp("p.png", &[0x89, b'P', b'N', b'G'])).unwrap();
        assert_eq!(a.kind, AttachmentKind::Image);
        assert_eq!(a.data_base64.as_deref(), Some("iVBORw=="));
    }

    #[test]
    fn truncation_respects_chars() {
        let (t, cut) = truncate_chars("héllo", 2);
        assert_eq!(t, "hé");
        assert!(cut);
    }

    #[test]
    fn docx_extraction() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            z.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(br#"<w:document><w:body><w:p><w:pPr/><w:r><w:t>Hello &amp; welcome</w:t></w:r></w:p><w:p><w:r><w:t>Line two</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
            z.finish().unwrap();
        }
        let a = read(&tmp("d.docx", buf.get_ref())).unwrap();
        assert_eq!(a.text.as_deref(), Some("Hello & welcome\nLine two"));
    }

    /// Build a minimal valid one-page PDF with correct xref offsets.
    fn tiny_pdf(text: &str) -> Vec<u8> {
        let stream = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
        let objs = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
        ];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
        for off in offsets {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
        out
    }

    #[test]
    fn pdf_text_extraction() {
        let a = read(&tmp("doc.pdf", &tiny_pdf("Hello from a PDF"))).unwrap();
        assert_eq!(a.kind, AttachmentKind::Text);
        assert!(a.text.unwrap().contains("Hello from a PDF"));
    }

    #[test]
    fn damaged_pdf_is_an_error_not_a_crash() {
        assert!(read(&tmp("bad.pdf", b"%PDF-1.4 garbage")).is_err());
    }
}
