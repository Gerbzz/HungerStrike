//! Bundled, offline receipt OCR.
//!
//! Text detection and recognition run in-process with the pure-Rust
//! [`ocrs`](https://github.com/robertknight/ocrs) engine (MIT OR Apache-2.0)
//! on the [`rten`](https://github.com/robertknight/rten) runtime. The two model
//! files in `assets/ocr/` are the published ocrs models (CC-BY-SA-4.0, see
//! `assets/ocr/NOTICE.md`). They are compiled into the executable, so OCR needs
//! no installer, network call, account or paid API.
//!
//! OCR output is only ever a draft: the receipt review screen requires a person
//! to check every line before anything is written to local storage.

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use rten::Model;
use rten_imageproc::RotatedRect;

pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PIXELS: u64 = 40_000_000;
/// Long edge the image is scaled down to before detection. Receipt photos
/// from phones are often 4000 px tall; very large inputs only cost time.
const MAX_LONG_EDGE: u32 = 2400;
pub const OCR_TIMEOUT: Duration = Duration::from_secs(60);
/// Number of text lines recognised between cancellation checks.
const RECOGNITION_CHUNK: usize = 6;

static DETECTION_MODEL: &[u8] = include_bytes!("../../assets/ocr/text-detection.rten");
static RECOGNITION_MODEL: &[u8] = include_bytes!("../../assets/ocr/text-recognition.rten");

/// Short description of the engine for the About dialog and receipt screen.
pub const ENGINE_LABEL: &str = "ocrs (bundled, offline)";

#[derive(Debug, PartialEq, Eq)]
pub enum OcrError {
    Unavailable,
    InvalidImage,
    TimedOut,
    Cancelled,
    NoText,
    Failed,
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "the bundled OCR models could not be loaded"),
            Self::InvalidImage => write!(
                f,
                "this file is not a readable PNG or JPEG under 8 MB and 40 megapixels"
            ),
            Self::TimedOut => write!(f, "offline OCR took too long and was stopped"),
            Self::Cancelled => write!(f, "offline OCR was cancelled"),
            Self::NoText => write!(f, "no receipt text was found in this image"),
            Self::Failed => write!(f, "offline OCR could not read this image"),
        }
    }
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

/// Cheap header check that runs before any decoding, so oversized or
/// non-image files never reach the decoder.
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

fn decode(bytes: &[u8]) -> Result<image::RgbImage, OcrError> {
    if !valid_image(bytes) {
        return Err(OcrError::InvalidImage);
    }
    // A truncated or corrupt body passes the header check but fails here.
    let decoded = image::load_from_memory(bytes).map_err(|_| OcrError::InvalidImage)?;
    let long_edge = decoded.width().max(decoded.height());
    let decoded = if long_edge > MAX_LONG_EDGE {
        decoded.resize(
            MAX_LONG_EDGE,
            MAX_LONG_EDGE,
            image::imageops::FilterType::Triangle,
        )
    } else {
        decoded
    };
    Ok(decoded.into_rgb8())
}

fn load_engine() -> Result<OcrEngine, OcrError> {
    let detection_model =
        Model::load_static_slice(DETECTION_MODEL).map_err(|_| OcrError::Unavailable)?;
    let recognition_model =
        Model::load_static_slice(RECOGNITION_MODEL).map_err(|_| OcrError::Unavailable)?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .map_err(|_| OcrError::Unavailable)
}

/// A recognised line with its axis-aligned position in the image.
#[derive(Clone, Debug)]
struct PlacedLine {
    text: String,
    left: f32,
    center_y: f32,
    height: f32,
}

/// Median text baseline slope across multi-word lines; used to undo slight
/// photo rotation before rows are rebuilt.
fn estimate_skew(lines: &[Vec<RotatedRect>]) -> f32 {
    let mut slopes: Vec<f32> = lines
        .iter()
        .filter(|words| words.len() >= 2)
        .filter_map(|words| {
            let first = words.first()?.center();
            let last = words.last()?.center();
            let dx = last.x - first.x;
            (dx > 1.0).then(|| (last.y - first.y) / dx)
        })
        .filter(|slope| slope.abs() < 0.2)
        .collect();
    if slopes.is_empty() {
        return 0.0;
    }
    slopes.sort_by(|a, b| a.total_cmp(b));
    slopes[slopes.len() / 2]
}

/// Receipt layouts put the item name on the left and the price far to the
/// right. The detector can return those as separate text lines; this joins
/// lines whose (skew-corrected) vertical centres overlap into one row, ordered
/// left to right, so the receipt parser sees "MILK 2% 1 GAL   4.29".
fn rebuild_rows(mut placed: Vec<PlacedLine>, skew: f32) -> Vec<String> {
    for line in &mut placed {
        line.center_y -= skew * line.left;
    }
    placed.sort_by(|a, b| a.center_y.total_cmp(&b.center_y));
    let mut rows: Vec<Vec<PlacedLine>> = Vec::new();
    for line in placed {
        let joins = rows.last().is_some_and(|row| {
            let anchor = &row[0];
            let tolerance = 0.55 * anchor.height.min(line.height).max(1.0);
            (line.center_y - anchor.center_y).abs() <= tolerance
        });
        if joins {
            rows.last_mut().expect("row exists").push(line);
        } else {
            rows.push(vec![line]);
        }
    }
    rows.into_iter()
        .map(|mut row| {
            row.sort_by(|a, b| a.left.total_cmp(&b.left));
            row.into_iter()
                .map(|line| line.text)
                .collect::<Vec<_>>()
                .join("   ")
        })
        // Drop speckle: rows with fewer than three letters or digits.
        .filter(|row| row.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 3)
        .collect()
}

fn check(cancelled: &AtomicBool, started: Instant, timeout: Duration) -> Result<(), OcrError> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(OcrError::Cancelled);
    }
    if started.elapsed() >= timeout {
        return Err(OcrError::TimedOut);
    }
    Ok(())
}

fn extract_with_timeout(
    bytes: &[u8],
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<String, OcrError> {
    let started = Instant::now();
    let image = decode(bytes)?;
    check(cancelled, started, timeout)?;
    let engine = load_engine()?;
    check(cancelled, started, timeout)?;
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|_| OcrError::Failed)?;
    let input = engine.prepare_input(source).map_err(|_| OcrError::Failed)?;
    check(cancelled, started, timeout)?;
    let words = engine.detect_words(&input).map_err(|_| OcrError::Failed)?;
    check(cancelled, started, timeout)?;
    let lines = engine.find_text_lines(&input, &words);
    if lines.is_empty() {
        return Err(OcrError::NoText);
    }
    let skew = estimate_skew(&lines);
    let mut placed = Vec::new();
    for chunk in lines.chunks(RECOGNITION_CHUNK) {
        check(cancelled, started, timeout)?;
        let recognised = engine
            .recognize_text(&input, chunk)
            .map_err(|_| OcrError::Failed)?;
        for line in recognised.into_iter().flatten() {
            let text = line.to_string();
            if text.trim().is_empty() {
                continue;
            }
            let rect = line.bounding_rect();
            placed.push(PlacedLine {
                text: text.trim().to_string(),
                left: rect.left() as f32,
                center_y: (rect.top() + rect.bottom()) as f32 / 2.0,
                height: (rect.bottom() - rect.top()).max(1) as f32,
            });
        }
    }
    check(cancelled, started, timeout)?;
    let rows = rebuild_rows(placed, skew);
    if rows.is_empty() {
        return Err(OcrError::NoText);
    }
    Ok(rows.join("\n"))
}

/// Read receipt text from a local PNG/JPEG. Cancellation and the timeout are
/// checked between decoding, detection and each small batch of recognised
/// lines, so a cancelled job stops promptly and never produces a draft.
pub fn extract_local_image_with_cancel(
    bytes: &[u8],
    cancelled: &AtomicBool,
) -> Result<String, OcrError> {
    extract_with_timeout(bytes, cancelled, OCR_TIMEOUT)
}

#[cfg(test)]
pub fn extract_local_image(bytes: &[u8]) -> Result<String, OcrError> {
    extract_local_image_with_cancel(bytes, &AtomicBool::new(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hungerstrike_core::{parse_text, review};

    const CLEAN: &[u8] = include_bytes!("../../tests/images/synthetic-receipt-clean.png");
    const PHOTO: &[u8] = include_bytes!("../../tests/images/synthetic-receipt-photo.jpg");
    const TRUNCATED: &[u8] = include_bytes!("../../tests/images/synthetic-receipt-truncated.png");

    #[test]
    fn invalid_or_oversized_dimensions_never_reach_ocr() {
        assert_eq!(
            extract_local_image(b"not an image"),
            Err(OcrError::InvalidImage)
        );
        assert_eq!(extract_local_image(b""), Err(OcrError::InvalidImage));
        let mut image = CLEAN.to_vec();
        image[16..20].copy_from_slice(&12_001_u32.to_be_bytes());
        assert_eq!(extract_local_image(&image), Err(OcrError::InvalidImage));
        let jpeg = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x07, 0x08, 0x00, 0x10, 0x00, 0x20, 0xff, 0xd9,
        ];
        assert!(valid_image(&jpeg));
    }

    #[test]
    fn corrupt_synthetic_images_are_rejected_without_a_draft() {
        // Valid PNG header, truncated body.
        assert!(valid_image(TRUNCATED));
        assert_eq!(extract_local_image(TRUNCATED), Err(OcrError::InvalidImage));
        // Valid JPEG header, garbage body.
        let mut garbage = PHOTO[..600].to_vec();
        garbage.extend(std::iter::repeat_n(0x5a, 4000));
        assert_eq!(extract_local_image(&garbage), Err(OcrError::InvalidImage));
    }

    #[test]
    fn cancellation_before_start_produces_no_text() {
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            extract_local_image_with_cancel(CLEAN, &cancelled),
            Err(OcrError::Cancelled)
        );
    }

    #[test]
    fn zero_timeout_stops_the_job() {
        assert_eq!(
            extract_with_timeout(CLEAN, &AtomicBool::new(false), Duration::ZERO),
            Err(OcrError::TimedOut)
        );
    }

    #[test]
    fn bundled_models_load() {
        assert!(load_engine().is_ok());
    }

    #[test]
    fn rows_are_rebuilt_left_to_right_with_skew() {
        let line = |text: &str, left: f32, center_y: f32| PlacedLine {
            text: text.into(),
            left,
            center_y,
            height: 30.0,
        };
        // Price column detected separately and ~20 px lower because of tilt.
        let rows = rebuild_rows(
            vec![
                line("4.29", 480.0, 121.0),
                line("MILK 2% 1 GAL", 30.0, 100.0),
                line("3.49", 480.0, 165.0),
                line("EGGS LG 12 CT", 30.0, 144.0),
            ],
            20.0 / 450.0,
        );
        assert_eq!(rows, vec!["MILK 2% 1 GAL   4.29", "EGGS LG 12 CT   3.49"]);
    }

    /// End-to-end: the bundled engine reads the clearly labelled synthetic
    /// receipt, the parser builds a draft, and the totals reconcile. The
    /// result is still only a draft; the app requires per-line review.
    #[test]
    fn bundled_engine_reads_labelled_synthetic_receipt() {
        let text = extract_local_image(CLEAN).expect("OCR text");
        eprintln!("OCR clean:\n{text}");
        let upper = text.to_ascii_uppercase();
        assert!(upper.contains("SYNTHETIC TEST RECEIPT"), "{text}");
        assert!(upper.contains("TOTAL"), "{text}");
        let draft = parse_text(&text);
        eprintln!(
            "PARSE clean: date={:?} items={:?} subtotal={:?} total={:?} flags={:?}",
            draft.purchase_date,
            draft
                .items
                .iter()
                .map(|item| (&item.name, &item.quantity, &item.net_cents))
                .collect::<Vec<_>>(),
            draft.printed_subtotal_cents,
            draft.printed_total_cents,
            draft.uncertainty
        );
        assert_eq!(draft.printed_total_cents, Some(1542), "{text}");
        assert!(draft.items.len() >= 4, "{:?}", draft.items);
        assert!(draft.items.iter().all(|item| item.expiry_date.is_none()));
    }

    /// The photo-style fixture is tilted, noisy and JPEG-compressed. The test
    /// records what was read; it asserts only that OCR produced text and that
    /// nothing in the draft claims an expiry date the receipt never printed.
    #[test]
    fn bundled_engine_handles_tilted_noisy_synthetic_photo() {
        let text = extract_local_image(PHOTO).expect("OCR text");
        eprintln!("OCR photo:\n{text}");
        let draft = parse_text(&text);
        eprintln!(
            "PARSE photo: items={:?} total={:?} reconciles={}",
            draft
                .items
                .iter()
                .map(|item| (&item.name, &item.net_cents))
                .collect::<Vec<_>>(),
            draft.printed_total_cents,
            review(draft.clone()).is_ok()
        );
        assert!(text.to_ascii_uppercase().contains("SYNTHETIC"), "{text}");
        assert!(draft.items.iter().all(|item| item.expiry_date.is_none()));
    }

    #[test]
    fn older_synthetic_matrix_is_recorded() {
        for (name, bytes) in [
            (
                "clean",
                include_bytes!("../../tests/images/synthetic-clean.png").as_slice(),
            ),
            (
                "tilted",
                include_bytes!("../../tests/images/synthetic-tilted.png").as_slice(),
            ),
            (
                "cropped",
                include_bytes!("../../tests/images/synthetic-cropped.png").as_slice(),
            ),
            (
                "blurred",
                include_bytes!("../../tests/images/synthetic-blurred.png").as_slice(),
            ),
        ] {
            match extract_local_image(bytes) {
                Ok(text) => {
                    let draft = parse_text(&text);
                    eprintln!(
                        "MATRIX {name}: items={} total={:?} reconciles={}",
                        draft.items.len(),
                        draft.printed_total_cents,
                        review(draft.clone()).is_ok()
                    );
                }
                Err(error) => eprintln!("MATRIX {name}: {error}"),
            }
        }
    }
}
