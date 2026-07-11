//! File text extraction.
//! Works on in-memory bytes from the multipart upload — no temp files needed.

use anyhow::{anyhow, Result};

pub const ALLOWED_EXTENSIONS: [&str; 4] = ["pdf", "md", "markdown", "txt"];

pub fn extension(filename: &str) -> Option<&str> {
    filename.rsplit_once('.').map(|(_, ext)| ext)
}

pub fn is_allowed(filename: &str) -> bool {
    extension(filename)
        .map(|ext| ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Extract plain text from an uploaded file's bytes, dispatching on extension.
pub fn extract_text(filename: &str, bytes: &[u8]) -> Result<String> {
    let ext = extension(filename)
        .map(|e| e.to_lowercase())
        .ok_or_else(|| anyhow!("file has no extension: {filename}"))?;

    match ext.as_str() {
        "pdf" => extract_pdf(bytes),
        "md" | "markdown" | "txt" => Ok(String::from_utf8_lossy(bytes).into_owned()),
        _ => Err(anyhow!("unsupported file format: .{ext}")),
    }
}

// ponytail: pdf-extract gives plain text only — no layout/tables/OCR like
// PyMuPDF. Good enough for LLM entity extraction over prose documents.
fn extract_pdf(bytes: &[u8]) -> Result<String> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| anyhow!("PDF text extraction failed: {e}"))?;
    if text.trim().is_empty() {
        return Err(anyhow!("no text extracted from PDF (scanned/image-only?)"));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_extensions() {
        assert!(is_allowed("doc.pdf"));
        assert!(is_allowed("notes.MD"));
        assert!(is_allowed("a.b.txt"));
        assert!(!is_allowed("image.png"));
        assert!(!is_allowed("noext"));
    }

    #[test]
    fn extracts_txt_and_md() {
        assert_eq!(extract_text("a.txt", b"hello").unwrap(), "hello");
        assert_eq!(extract_text("a.md", "# 标题".as_bytes()).unwrap(), "# 标题");
    }

    #[test]
    fn rejects_unsupported() {
        assert!(extract_text("a.docx", b"x").is_err());
        assert!(extract_text("noext", b"x").is_err());
    }
}
