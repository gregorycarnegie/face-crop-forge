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

pub fn build_zip_bytes(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (file_name, bytes) in entries {
            writer
                .start_file(file_name, options)
                .map_err(|err| format!("ZIP start_file failed: {err}"))?;
            writer
                .write_all(bytes)
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
}
