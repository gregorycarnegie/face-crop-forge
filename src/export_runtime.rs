use std::io::Write;

pub fn extension_for_mime(mime_type: &str) -> Option<&'static str> {
    let lower = mime_type.to_ascii_lowercase();
    if lower.contains("image/png") {
        Some("png")
    } else if lower.contains("image/jpeg") || lower.contains("image/jpg") {
        Some("jpg")
    } else if lower.contains("image/webp") {
        Some("webp")
    } else {
        None
    }
}

pub fn normalize_export_filename_for_mime(file_name: &str, mime_type: &str) -> String {
    let Some(ext) = extension_for_mime(mime_type) else {
        return file_name.to_string();
    };
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(base, _)| base);
    format!("{stem}.{ext}")
}

pub fn validate_export_filename_for_mime(file_name: &str, mime_type: &str) -> bool {
    let Some(ext) = extension_for_mime(mime_type) else {
        return true;
    };
    file_name.to_ascii_lowercase().ends_with(&format!(".{ext}"))
}

fn iso_timestamp_to_token(iso: &str) -> String {
    iso.chars()
        .filter(|ch| ch.is_ascii_digit() || *ch == 'T' || *ch == 'Z')
        .collect()
}

pub fn current_utc_timestamp_token() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let iso = js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string());
        iso_timestamp_to_token(&iso)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "19700101T000000000Z".to_string()
    }
}

pub fn current_timestamp_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

pub fn build_zip_bytes<S, B>(entries: &[(S, B)]) -> Result<Vec<u8>, String>
where
    S: AsRef<str>,
    B: AsRef<[u8]>,
{
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (file_name, bytes) in entries {
            writer
                .start_file(file_name.as_ref(), options)
                .map_err(|err| format!("ZIP start_file failed: {err}"))?;
            writer
                .write_all(bytes.as_ref())
                .map_err(|err| format!("ZIP write failed: {err}"))?;
        }
        writer
            .finish()
            .map_err(|err| format!("ZIP finish failed: {err}"))?;
    }
    Ok(cursor.into_inner())
}

#[cfg(target_arch = "wasm32")]
pub fn download_bytes(file_name: &str, mime_type: &str, bytes: &[u8]) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let blob_options = web_sys::BlobPropertyBag::new();
    blob_options.set_type(mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &blob_options)
        .map_err(|err| format!("Failed to build blob: {err:?}"))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|err| format!("Failed to create object URL: {err:?}"))?;
    let document = leptos::prelude::window()
        .document()
        .ok_or_else(|| "Document unavailable".to_string())?;
    let anchor = document
        .create_element("a")
        .map_err(|err| format!("Failed to create anchor: {err:?}"))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "Failed to cast anchor".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(file_name);
    let () = anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn download_bytes(_file_name: &str, _mime_type: &str, _bytes: &[u8]) -> Result<(), String> {
    Err("download_bytes is only available on wasm32".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_timestamp_filter_keeps_only_digits_t_and_z() {
        assert_eq!(
            iso_timestamp_to_token("2024-01-15T12:30:45.000Z"),
            "20240115T123045000Z"
        );
        assert_eq!(
            iso_timestamp_to_token("1970-01-01T00:00:00.000Z"),
            "19700101T000000000Z"
        );
        assert_eq!(iso_timestamp_to_token(""), "");
        assert_eq!(iso_timestamp_to_token("no-digits-here"), "");
    }

    #[test]
    fn mime_extension_mapping_covers_png_jpeg_webp() {
        assert_eq!(extension_for_mime("image/png"), Some("png"));
        assert_eq!(extension_for_mime("image/jpeg"), Some("jpg"));
        assert_eq!(extension_for_mime("image/webp"), Some("webp"));
    }

    #[test]
    fn normalize_and_validate_match_blob_format() {
        let jpg_name = normalize_export_filename_for_mime("face_1.png", "image/jpeg");
        assert_eq!(jpg_name, "face_1.jpg");
        assert!(validate_export_filename_for_mime(&jpg_name, "image/jpeg"));
        assert!(!validate_export_filename_for_mime(
            "face_1.webp",
            "image/jpeg"
        ));

        let png_name = normalize_export_filename_for_mime("face_2", "image/png");
        assert_eq!(png_name, "face_2.png");
        assert!(validate_export_filename_for_mime(&png_name, "image/png"));
    }

    // ── ZIP regression guards ─────────────────────────────────────────────────

    #[test]
    fn zip_output_has_valid_header_and_all_entries_round_trip() {
        let entries = vec![
            ("alpha.png".to_string(), vec![1u8, 2, 3, 4, 5]),
            ("beta.jpg".to_string(), vec![10u8, 20, 30]),
            ("gamma.webp".to_string(), vec![255u8; 64]),
        ];

        let zip = build_zip_bytes(&entries).expect("build_zip_bytes failed");
        assert_eq!(&zip[..4], b"PK\x03\x04", "ZIP local file header magic missing");

        let cursor = std::io::Cursor::new(&zip);
        let mut archive = zip::ZipArchive::new(cursor).expect("ZipArchive::new failed");
        assert_eq!(archive.len(), 3, "wrong entry count after round-trip");

        let mut names = std::collections::HashSet::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            names.insert(file.name().to_string());
        }
        assert!(names.contains("alpha.png"));
        assert!(names.contains("beta.jpg"));
        assert!(names.contains("gamma.webp"));
    }

    #[test]
    fn zip_stored_compression_does_not_shrink_compressible_data() {
        // 200 entries of 1 KiB each of all-zeros — maximally compressible.
        // With Deflated this would shrink dramatically; Stored must not.
        let entry_bytes = vec![0u8; 1024];
        let entries: Vec<(String, &[u8])> = (0..200)
            .map(|i| (format!("file_{i:03}.bin"), entry_bytes.as_slice()))
            .collect();

        let zip = build_zip_bytes(&entries).expect("build_zip_bytes failed");
        assert_eq!(&zip[..4], b"PK\x03\x04");

        // Stored ZIP output = Σ(entry sizes) + per-entry header overhead.
        // Local header: 30 bytes + filename len. Central dir: 46 bytes + filename.
        // End of central dir: 22 bytes. Filename ~12 chars each.
        // Min bound: raw data alone (regression: compression accidentally re-enabled shrinks this).
        let raw_total = 200 * 1024;
        assert!(
            zip.len() >= raw_total,
            "ZIP ({} bytes) smaller than raw input ({raw_total} bytes) — compression re-enabled?",
            zip.len()
        );
        // Max bound: raw + generous header overhead (~100 bytes per entry + end record).
        let max_expected = raw_total + 200 * 100 + 100;
        assert!(
            zip.len() <= max_expected,
            "ZIP ({} bytes) unexpectedly large (max {max_expected})",
            zip.len()
        );
    }

    #[test]
    fn zip_entry_content_survives_round_trip_intact() {
        let payload: Vec<u8> = (0u8..=255).cycle().take(512).collect();
        let entries = [("data.bin".to_string(), payload.clone())];

        let zip = build_zip_bytes(&entries).expect("build_zip_bytes failed");
        let cursor = std::io::Cursor::new(&zip);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut file = archive.by_name("data.bin").expect("entry not found");

        use std::io::Read;
        let mut recovered = Vec::new();
        file.read_to_end(&mut recovered).unwrap();
        assert_eq!(recovered, payload, "entry content corrupted in round-trip");
    }
}
