use super::components::*;
use super::helpers::*;
use super::*;

macro_rules! record_failed_image {
    (
        $progress:expr,
        $stats:expr,
        $state:expr,
        $id:expr,
        $elapsed_ms:expr,
        $status:expr,
        $log:expr
    ) => {{
        $progress.update(|p| {
            p.record_result(false);
            p.status = $status;
        });
        $stats.update(|stats| {
            stats.record_image($elapsed_ms, 0, false);
            stats.push_log($log);
        });
        $state.update(|s| s.mark_error($id));
    }};
}

mod batch;
mod csv;
mod misc;
mod single;

pub(super) use batch::BatchPage;
pub(super) use csv::CsvPage;
pub(super) use misc::{LandingPage, PanelsGalleryPage};
pub(super) use single::SinglePage;
