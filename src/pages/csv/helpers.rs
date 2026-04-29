use crate::batch_core::ImageStatus;
use crate::state::ProcessingSettings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CsvMatchFilter {
    All,
    Done,
    Failed,
}

pub(super) fn csv_filter_tab_class(
    current: CsvMatchFilter,
    expected: CsvMatchFilter,
) -> &'static str {
    if current == expected { "on" } else { "" }
}

pub(super) fn csv_filter_matches(filter: CsvMatchFilter, status: &ImageStatus) -> bool {
    match filter {
        CsvMatchFilter::All => true,
        CsvMatchFilter::Done => matches!(status, ImageStatus::Processed),
        CsvMatchFilter::Failed => matches!(status, ImageStatus::Error),
    }
}

pub(super) fn csv_empty_filter_label(filter: CsvMatchFilter) -> &'static str {
    match filter {
        CsvMatchFilter::All => "No CSV matches yet.",
        CsvMatchFilter::Done => "No processed CSV matches yet.",
        CsvMatchFilter::Failed => "No failed CSV matches.",
    }
}

pub(super) fn guess_column(headers: &[String], candidates: &[&str]) -> String {
    candidates
        .iter()
        .find_map(|candidate| {
            headers
                .iter()
                .find(|header| header.eq_ignore_ascii_case(candidate))
                .cloned()
        })
        .or_else(|| headers.first().cloned())
        .unwrap_or_default()
}

pub(super) fn csv_status_badge(status: &ImageStatus) -> (&'static str, &'static str) {
    match status {
        ImageStatus::Loaded => ("badge queued", "queued"),
        ImageStatus::Processing => ("badge run", "running"),
        ImageStatus::Processed => ("badge ok", "done"),
        ImageStatus::Error => ("badge fail", "failed"),
    }
}

pub(super) fn csv_format_class(current: &str, expected: &str) -> String {
    if current.eq_ignore_ascii_case(expected)
        || (expected == "jpeg" && current.eq_ignore_ascii_case("jpg"))
    {
        "on lime".to_string()
    } else {
        String::new()
    }
}

pub(super) fn csv_aspect_class(
    settings: &ProcessingSettings,
    width: u32,
    height: u32,
) -> &'static str {
    if settings.output_width == width && settings.output_height == height {
        "on lime"
    } else {
        ""
    }
}

pub(super) fn csv_template_display(template: &str) -> String {
    if template.trim().is_empty() || template == "face_{original}_{index}" {
        "{csv_name}".to_string()
    } else {
        template.to_string()
    }
}
