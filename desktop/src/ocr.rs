use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PIXELS: u64 = 40_000_000;
const MAX_OCR_BYTES: u64 = 1024 * 1024;
const OCR_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, PartialEq, Eq)]
pub enum OcrError {
    Unavailable,
    InvalidImage,
    TimedOut,
    Cancelled,
    Failed,
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "offline OCR is unavailable on this computer"),
            Self::InvalidImage => write!(f, "choose a PNG or JPEG under 8 MB and 40 megapixels"),
            Self::TimedOut => write!(f, "offline OCR timed out; the engine was stopped"),
            Self::Cancelled => write!(f, "offline OCR was cancelled"),
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

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    while at + 4 <= bytes.len() {
        if bytes[at] != 0xff {
            return None;
        }
        while at < bytes.len() && bytes[at] == 0xff {
            at += 1;
        }
        let marker = *bytes.get(at)?;
        at += 1;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]) as usize;
        if length < 2 || at.checked_add(length)? > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 7 {
                return None;
            }
            let height = u16::from_be_bytes([bytes[at + 3], bytes[at + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]) as u32;
            return Some((width, height));
        }
        at += length;
    }
    None
}

fn valid_image(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return false;
    }
    let dimensions = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24 {
        Some((
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        ))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        jpeg_dimensions(bytes)
    } else {
        None
    };
    dimensions.is_some_and(|(width, height)| {
        width > 0
            && height > 0
            && width <= 12_000
            && height <= 12_000
            && u64::from(width) * u64::from(height) <= MAX_PIXELS
    })
}

fn read_capped(mut stream: impl Read, limit: u64) -> Result<Vec<u8>, OcrError> {
    let mut bytes = Vec::new();
    stream
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| OcrError::Failed)?;
    if bytes.len() as u64 > limit {
        return Err(OcrError::Failed);
    }
    Ok(bytes)
}

fn run_child(
    mut command: Command,
    bytes: &[u8],
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<String, OcrError> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| OcrError::Unavailable)?;
    let mut stdin = child.stdin.take().ok_or(OcrError::Failed)?;
    let stdout = child.stdout.take().ok_or(OcrError::Failed)?;
    let stderr = child.stderr.take().ok_or(OcrError::Failed)?;
    let input = bytes.to_vec();
    let writer = thread::spawn(move || stdin.write_all(&input).map_err(|_| OcrError::Failed));
    let output = thread::spawn(move || read_capped(stdout, MAX_OCR_BYTES));
    let errors = thread::spawn(move || read_capped(stderr, 32 * 1024));
    let started = Instant::now();
    let result = loop {
        if cancelled.load(Ordering::Relaxed) {
            break Err(OcrError::Cancelled);
        }
        if started.elapsed() >= timeout {
            break Err(OcrError::TimedOut);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(_) => break Err(OcrError::Failed),
        }
    };
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let write_result = writer.join().map_err(|_| OcrError::Failed)?;
    let text_result = output.join().map_err(|_| OcrError::Failed)?;
    let error_result = errors.join().map_err(|_| OcrError::Failed)?;
    let status = result?;
    write_result?;
    let text_bytes = text_result?;
    let _ = error_result?;
    if !status.success() {
        return Err(OcrError::Failed);
    }
    let text = String::from_utf8(text_bytes).map_err(|_| OcrError::Failed)?;
    if text.trim().is_empty() {
        return Err(OcrError::Failed);
    }
    Ok(text)
}

pub fn extract_local_image_with_cancel(
    bytes: &[u8],
    engine: &Path,
    cancelled: &AtomicBool,
) -> Result<String, OcrError> {
    if !valid_image(bytes) {
        return Err(OcrError::InvalidImage);
    }
    if !engine.is_file() {
        return Err(OcrError::Unavailable);
    }
    let mut command = Command::new(engine);
    command.args(["stdin", "stdout", "-l", "eng", "--psm", "6"]);
    run_child(command, bytes, OCR_TIMEOUT, cancelled)
}

#[cfg(test)]
pub fn extract_local_image(bytes: &[u8], engine: &Path) -> Result<String, OcrError> {
    extract_local_image_with_cancel(bytes, engine, &AtomicBool::new(false))
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
    fn invalid_or_oversized_dimensions_never_reach_ocr() {
        assert_eq!(
            extract_local_image(b"not an image", Path::new("missing-tesseract.exe")),
            Err(OcrError::InvalidImage)
        );
        let mut image = include_bytes!("../../tests/images/synthetic-clean.png").to_vec();
        image[16..20].copy_from_slice(&12_001_u32.to_be_bytes());
        assert_eq!(
            extract_local_image(&image, Path::new("missing-tesseract.exe")),
            Err(OcrError::InvalidImage)
        );
        let jpeg = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x07, 0x08, 0x00, 0x10, 0x00, 0x20, 0xff, 0xd9,
        ];
        assert!(valid_image(&jpeg));
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

    #[cfg(windows)]
    #[test]
    fn stalled_engine_times_out_and_cancellation_stops_it() {
        let mut slow = Command::new("powershell.exe");
        slow.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 5"]);
        let start = Instant::now();
        assert_eq!(
            run_child(
                slow,
                b"small",
                Duration::from_millis(150),
                &AtomicBool::new(false)
            ),
            Err(OcrError::TimedOut)
        );
        assert!(start.elapsed() < Duration::from_secs(3));
        let mut slow = Command::new("powershell.exe");
        slow.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 5"]);
        assert_eq!(
            run_child(
                slow,
                b"small",
                Duration::from_secs(5),
                &AtomicBool::new(true)
            ),
            Err(OcrError::Cancelled)
        );
    }
}
