use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug, PartialEq, Eq)]
pub enum OcrError {
    Unavailable,
    InvalidImage,
    Failed,
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "offline OCR is unavailable on this computer"),
            Self::InvalidImage => write!(f, "choose a PNG or JPEG image under 8 MB"),
            Self::Failed => write!(f, "offline OCR could not read this image"),
        }
    }
}

pub fn installed_engine() -> std::path::PathBuf {
    std::env::var_os("ProgramFiles")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Program Files"))
        .join("Tesseract-OCR")
        .join("tesseract.exe")
}

pub fn extract_local_image(bytes: &[u8], engine: &Path) -> Result<String, OcrError> {
    const LIMIT: usize = 8 * 1024 * 1024;
    let is_png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    let is_jpeg = bytes.starts_with(b"\xff\xd8\xff");
    if bytes.is_empty() || bytes.len() > LIMIT || !(is_png || is_jpeg) {
        return Err(OcrError::InvalidImage);
    }
    if !engine.is_file() {
        return Err(OcrError::Unavailable);
    }
    let mut child = Command::new(engine)
        .args(["stdin", "stdout", "-l", "eng", "--psm", "6"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| OcrError::Unavailable)?;
    child
        .stdin
        .take()
        .ok_or(OcrError::Failed)?
        .write_all(bytes)
        .map_err(|_| OcrError::Failed)?;
    let output = child.wait_with_output().map_err(|_| OcrError::Failed)?;
    if !output.status.success() {
        return Err(OcrError::Failed);
    }
    let text = String::from_utf8(output.stdout).map_err(|_| OcrError::Failed)?;
    if text.trim().is_empty() {
        return Err(OcrError::Failed);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_engine_returns_manual_fallback_signal() {
        let image = include_bytes!("../../tests/images/synthetic-clean.png");
        assert_eq!(
            extract_local_image(image, Path::new("missing-tesseract.exe")),
            Err(OcrError::Unavailable)
        );
    }

    #[test]
    fn invalid_input_never_reaches_ocr() {
        assert_eq!(
            extract_local_image(b"not an image", Path::new("missing-tesseract.exe")),
            Err(OcrError::InvalidImage)
        );
    }

    #[test]
    fn installed_english_engine_reads_synthetic_clean_image_when_present() {
        let engine = installed_engine();
        if !engine.is_file() {
            return;
        }
        let image = include_bytes!("../../tests/images/synthetic-clean.png");
        let text = extract_local_image(image, &engine).unwrap();
        assert!(text.contains("SUBTOTAL"));
        assert!(text.contains("TOTAL"));
    }
}
