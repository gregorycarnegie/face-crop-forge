#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadEntry {
    pub file_name: String,
    pub source_image: String,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZipPlan {
    pub zip_name: String,
    pub entry_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchProgress {
    pub total: usize,
    pub processed: usize,
    pub failed: usize,
    pub running: bool,
    pub status: String,
}

impl Default for BatchProgress {
    fn default() -> Self {
        Self {
            total: 0,
            processed: 0,
            failed: 0,
            running: false,
            status: "Ready".to_string(),
        }
    }
}

impl BatchProgress {
    pub fn start(&mut self, total: usize, status: impl Into<String>) {
        self.total = total;
        self.processed = 0;
        self.failed = 0;
        self.running = true;
        self.status = status.into();
    }

    pub fn record_result(&mut self, success: bool) {
        self.processed = self.processed.saturating_add(1);
        if !success {
            self.failed = self.failed.saturating_add(1);
        }
    }

    pub fn complete(&mut self, status: impl Into<String>) {
        self.running = false;
        self.status = status.into();
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        {
            ((self.processed as f64 / self.total as f64) * 100.0).round() as u8
        }
    }
}

#[cfg(test)]
pub fn default_zip_name(timestamp_utc: &str) -> String {
    format!("face-crops-{}.zip", timestamp_utc)
}

#[cfg(test)]
pub fn plan_zip_export(entries: &[DownloadEntry], timestamp_utc: &str) -> ZipPlan {
    ZipPlan {
        zip_name: default_zip_name(timestamp_utc),
        entry_count: entries.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tracks_counts_and_percent() {
        let mut progress = BatchProgress::default();
        progress.start(4, "Processing...");
        assert!(progress.running);
        assert_eq!(progress.percent(), 0);

        progress.record_result(true);
        progress.record_result(false);
        assert_eq!(progress.processed, 2);
        assert_eq!(progress.failed, 1);
        assert_eq!(progress.percent(), 50);

        progress.complete("Done");
        assert!(!progress.running);
        assert_eq!(progress.status, "Done");
    }

    #[test]
    fn zip_name_and_plan_are_deterministic() {
        let entries = vec![
            DownloadEntry {
                file_name: "a.png".to_string(),
                source_image: "img1.jpg".to_string(),
            },
            DownloadEntry {
                file_name: "b.png".to_string(),
                source_image: "img2.jpg".to_string(),
            },
        ];
        let plan = plan_zip_export(&entries, "20260227T190000Z");
        assert_eq!(plan.entry_count, 2);
        assert_eq!(plan.zip_name, "face-crops-20260227T190000Z.zip");
    }
}
