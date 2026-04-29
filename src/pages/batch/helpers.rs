use crate::batch_core::ImageStatus;
use crate::state::ProcessingSettings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BatchGalleryFilter {
    All,
    Done,
    Failed,
}

pub(super) fn filter_tab_class(current: BatchGalleryFilter, expected: BatchGalleryFilter) -> &'static str {
    if current == expected { "on" } else { "" }
}

pub(super) fn batch_filter_matches(filter: BatchGalleryFilter, status: &ImageStatus) -> bool {
    match filter {
        BatchGalleryFilter::All => true,
        BatchGalleryFilter::Done => matches!(status, ImageStatus::Processed),
        BatchGalleryFilter::Failed => matches!(status, ImageStatus::Error),
    }
}

pub(super) fn batch_empty_filter_label(filter: BatchGalleryFilter) -> &'static str {
    match filter {
        BatchGalleryFilter::All => "Drop images to populate the queue.",
        BatchGalleryFilter::Done => "No processed images yet.",
        BatchGalleryFilter::Failed => "No failed images.",
    }
}

pub(super) fn status_badge(status: &ImageStatus) -> (&'static str, &'static str) {
    match status {
        ImageStatus::Loaded => ("badge queued", "queued"),
        ImageStatus::Processing => ("badge run", "running"),
        ImageStatus::Processed => ("badge ok", "done"),
        ImageStatus::Error => ("badge fail", "failed"),
    }
}

pub(super) fn gallery_cell_class(selected: bool, status: &ImageStatus) -> String {
    let mut class_name = "gcell".to_string();
    if selected {
        class_name.push_str(" sel");
    }
    match status {
        ImageStatus::Processing => class_name.push_str(" processing"),
        ImageStatus::Error => class_name.push_str(" failed"),
        ImageStatus::Loaded | ImageStatus::Processed => {}
    }
    class_name
}

pub(super) fn batch_format_class(current: &str, expected: &str) -> String {
    if current.eq_ignore_ascii_case(expected)
        || (expected == "jpeg" && current.eq_ignore_ascii_case("jpg"))
    {
        "on".to_string()
    } else {
        String::new()
    }
}

pub(super) fn batch_aspect_class(settings: &ProcessingSettings, width: u32, height: u32) -> &'static str {
    if settings.output_width == width && settings.output_height == height {
        "on"
    } else {
        ""
    }
}
