use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageStatus {
    Loaded,
    Processing,
    Processed,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchImage {
    pub id: String,
    pub selected: bool,
    pub processed: bool,
    pub status: ImageStatus,
}

impl BatchImage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            selected: true,
            processed: false,
            status: ImageStatus::Loaded,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BatchCoreState {
    pub images: HashMap<String, BatchImage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BatchQueueState {
    pub page_size: usize,
    pub loaded_ids: Vec<String>,
    pub queued_pages: Vec<Vec<String>>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchWorkPlan {
    pub selected_total: usize,
    pub chunk_size: usize,
    pub chunks: Vec<Vec<String>>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchLazyLoadPlan {
    pub total_files: usize,
    pub page_size: usize,
    pub initial_load_count: usize,
    pub queued_files: usize,
    pub queued_pages: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchRuntimeStats {
    pub total_faces_detected: u64,
    pub images_processed: u64,
    pub successful_processing: u64,
    pub processing_times_ms: Vec<u64>,
    pub last_export_time_ms: Option<u64>,
    pub detected_backend: String,
    pub last_image_dimensions: Option<(u32, u32)>,
    pub last_downscale_factor: f64,
    pub logs: Vec<String>,
}

impl Default for BatchRuntimeStats {
    fn default() -> Self {
        Self {
            total_faces_detected: 0,
            images_processed: 0,
            successful_processing: 0,
            processing_times_ms: Vec::new(),
            last_export_time_ms: None,
            detected_backend: String::new(),
            last_image_dimensions: None,
            last_downscale_factor: 1.0,
            logs: vec!["Ready to process images...".to_string()],
        }
    }
}

impl BatchRuntimeStats {
    pub fn record_image(&mut self, processing_time_ms: u64, faces_detected: u32, success: bool) {
        self.images_processed = self.images_processed.saturating_add(1);
        self.total_faces_detected = self
            .total_faces_detected
            .saturating_add(u64::from(faces_detected));
        if success {
            self.successful_processing = self.successful_processing.saturating_add(1);
        }
        self.processing_times_ms.push(processing_time_ms);
        if self.processing_times_ms.len() > 50 {
            let overflow = self.processing_times_ms.len() - 50;
            self.processing_times_ms.drain(0..overflow);
        }
    }

    #[cfg(test)]
    pub fn success_rate_pct(&self) -> u32 {
        if self.images_processed == 0 {
            return 0;
        }
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        {
            ((self.successful_processing as f64 / self.images_processed as f64) * 100.0).round()
                as u32
        }
    }

    pub fn avg_processing_time_ms(&self) -> u64 {
        if self.processing_times_ms.is_empty() {
            return 0;
        }
        let sum: u64 = self.processing_times_ms.iter().sum();
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        {
            (sum as f64 / self.processing_times_ms.len() as f64).round() as u64
        }
    }

    #[allow(dead_code)]
    pub fn record_export(&mut self, time_ms: u64) {
        self.last_export_time_ms = Some(time_ms);
    }

    pub fn record_backend(&mut self, backend: &str) {
        if self.detected_backend.is_empty() {
            self.detected_backend = backend.to_string();
        }
    }

    pub fn record_image_size(&mut self, width: u32, height: u32, downscale: f64) {
        self.last_image_dimensions = Some((width, height));
        self.last_downscale_factor = downscale;
    }

    pub fn push_log(&mut self, message: impl Into<String>) {
        self.logs.push(message.into());
        if self.logs.len() > 100 {
            let overflow = self.logs.len() - 100;
            self.logs.drain(0..overflow);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl BatchCoreState {
    pub fn set_images(&mut self, ids: Vec<String>) {
        self.images.clear();
        for id in ids {
            self.images.insert(id.clone(), BatchImage::new(id));
        }
    }

    pub fn add_images(&mut self, ids: Vec<String>) {
        for id in ids {
            self.images
                .entry(id.clone())
                .or_insert_with(|| BatchImage::new(id));
        }
    }

    #[cfg(test)]
    pub fn select_all(&mut self) {
        for image in self.images.values_mut() {
            image.selected = true;
        }
    }

    #[cfg(test)]
    pub fn select_none(&mut self) {
        for image in self.images.values_mut() {
            image.selected = false;
        }
    }

    pub fn toggle_selection(&mut self, id: &str) {
        if let Some(image) = self.images.get_mut(id) {
            image.selected = !image.selected;
        }
    }

    pub fn mark_processing(&mut self, id: &str) {
        if let Some(image) = self.images.get_mut(id) {
            image.status = ImageStatus::Processing;
        }
    }

    pub fn mark_processed(&mut self, id: &str) {
        if let Some(image) = self.images.get_mut(id) {
            image.status = ImageStatus::Processed;
            image.processed = true;
        }
    }

    pub fn mark_error(&mut self, id: &str) {
        if let Some(image) = self.images.get_mut(id) {
            image.status = ImageStatus::Error;
            image.processed = false;
        }
    }

    pub fn selected_count(&self) -> usize {
        self.images.values().filter(|img| img.selected).count()
    }

    pub fn total_count(&self) -> usize {
        self.images.len()
    }

    #[cfg(test)]
    pub fn has_selected_images(&self) -> bool {
        self.selected_count() > 0
    }

    pub fn selected_ids(&self) -> Vec<String> {
        let mut ids = self
            .images
            .values()
            .filter(|img| img.selected)
            .map(|img| img.id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    #[cfg(test)]
    pub fn build_work_plan(&self, preferred_chunk_size: usize) -> BatchWorkPlan {
        let selected_ids = self.selected_ids();
        let selected_total = selected_ids.len();
        let chunk_size = preferred_chunk_size.max(1);
        let chunks = selected_ids
            .chunks(chunk_size)
            .map(<[std::string::String]>::to_vec)
            .collect();

        BatchWorkPlan {
            selected_total,
            chunk_size,
            chunks,
        }
    }

    #[cfg(test)]
    pub fn estimate_preview_memory_bytes(
        preview_width: usize,
        preview_height: usize,
        buffered_images: usize,
    ) -> usize {
        let pixels_per_image = preview_width.saturating_mul(preview_height);
        let rgba_bytes_per_image = pixels_per_image.saturating_mul(4);
        rgba_bytes_per_image.saturating_mul(buffered_images)
    }

    #[cfg(test)]
    pub fn build_lazy_load_plan(
        total_files: usize,
        preferred_page_size: usize,
    ) -> BatchLazyLoadPlan {
        let page_size = preferred_page_size.max(1);
        let initial_load_count = total_files.min(page_size);
        let queued_files = total_files.saturating_sub(initial_load_count);
        let queued_pages = if queued_files == 0 {
            0
        } else {
            queued_files.div_ceil(page_size)
        };

        BatchLazyLoadPlan {
            total_files,
            page_size,
            initial_load_count,
            queued_files,
            queued_pages,
        }
    }
}

impl BatchQueueState {
    pub fn from_files(ids: &[String], preferred_page_size: usize) -> Self {
        let page_size = preferred_page_size.max(1);
        let mut chunks = ids
            .chunks(page_size)
            .map(<[std::string::String]>::to_vec)
            .collect::<Vec<_>>();
        let loaded_ids = chunks.first().cloned().unwrap_or_default();
        if !chunks.is_empty() {
            chunks.remove(0);
        }
        Self {
            page_size,
            loaded_ids,
            queued_pages: chunks,
        }
    }

    #[cfg(test)]
    pub fn queued_pages_count(&self) -> usize {
        self.queued_pages.len()
    }

    pub fn queued_files_count(&self) -> usize {
        self.queued_pages.iter().map(Vec::len).sum()
    }

    pub fn dequeue_next_page(&mut self) -> Option<Vec<String>> {
        if self.queued_pages.is_empty() {
            return None;
        }
        let page = self.queued_pages.remove(0);
        self.loaded_ids.extend(page.iter().cloned());
        Some(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_all_none_toggle_updates_counts() {
        let mut state = BatchCoreState::default();
        state.set_images(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(state.total_count(), 3);
        assert_eq!(state.selected_count(), 3);

        state.select_none();
        assert_eq!(state.selected_count(), 0);
        assert!(!state.has_selected_images());

        state.toggle_selection("b");
        assert_eq!(state.selected_count(), 1);
        assert!(state.has_selected_images());

        state.select_all();
        assert_eq!(state.selected_count(), 3);
    }

    #[test]
    fn processing_status_transitions_work() {
        let mut state = BatchCoreState::default();
        state.set_images(vec!["img_1".into()]);

        state.mark_processing("img_1");
        assert_eq!(
            state.images.get("img_1").unwrap().status,
            ImageStatus::Processing
        );

        state.mark_processed("img_1");
        let image = state.images.get("img_1").unwrap();
        assert_eq!(image.status, ImageStatus::Processed);
        assert!(image.processed);
    }

    #[test]
    fn work_plan_chunks_selected_images() {
        let mut state = BatchCoreState::default();
        state.set_images((1..=7).map(|i| format!("img_{i}")).collect());
        state.toggle_selection("img_2");
        state.toggle_selection("img_4");
        state.toggle_selection("img_6");

        let plan = state.build_work_plan(2);
        assert_eq!(plan.selected_total, 4);
        assert_eq!(plan.chunk_size, 2);
        assert_eq!(plan.chunks.len(), 2);
        assert_eq!(plan.chunks[0].len(), 2);
        assert_eq!(plan.chunks[1].len(), 2);
    }

    #[test]
    fn estimate_preview_memory_is_linear_and_bounded() {
        let bytes = BatchCoreState::estimate_preview_memory_bytes(640, 640, 200);
        assert_eq!(bytes, 640 * 640 * 4 * 200);
        assert!(bytes < 400_000_000);
    }

    #[test]
    fn large_batch_build_work_plan_remains_chunked() {
        let mut state = BatchCoreState::default();
        let ids = (0..10_000).map(|i| format!("img_{i}")).collect();
        state.set_images(ids);
        let plan = state.build_work_plan(128);

        assert_eq!(plan.selected_total, 10_000);
        assert_eq!(plan.chunk_size, 128);
        assert_eq!(plan.chunks.len(), 79);
        assert_eq!(plan.chunks.last().unwrap().len(), 16);
    }

    #[test]
    fn runtime_stats_compute_success_rate_and_average() {
        let mut stats = BatchRuntimeStats::default();
        stats.record_image(100, 2, true);
        stats.record_image(300, 1, false);

        assert_eq!(stats.images_processed, 2);
        assert_eq!(stats.total_faces_detected, 3);
        assert_eq!(stats.success_rate_pct(), 50);
        assert_eq!(stats.avg_processing_time_ms(), 200);
    }

    #[test]
    fn large_batch_progress_and_stats_stay_accurate_over_500_images() {
        use crate::batch_export::BatchProgress;

        const N: usize = 500;
        const FAILURES: usize = 47;

        let mut progress = BatchProgress::default();
        progress.start(N, "Running...");
        assert!(progress.running);
        assert!(!progress.cancelled);

        let mut stats = BatchRuntimeStats::default();
        for i in 0..N {
            let success = i >= FAILURES;
            progress.record_result(success);
            // Alternate between 80 ms and 120 ms processing time.
            let time_ms = if i % 2 == 0 { 80 } else { 120 };
            stats.record_image(time_ms, if success { 1 } else { 0 }, success);
        }

        // Progress counters must be exact.
        assert_eq!(progress.total, N);
        assert_eq!(progress.processed, N);
        assert_eq!(progress.failed, FAILURES);
        assert_eq!(progress.percent(), 100);
        assert!(
            progress.running,
            "running must still be true before complete()"
        );

        progress.complete(format!(
            "Batch complete: {} processed, {} failed.",
            N - FAILURES,
            FAILURES
        ));
        assert!(!progress.running);
        assert!(!progress.cancelled);

        // Stats counters must be exact.
        assert_eq!(stats.images_processed, N as u64);
        assert_eq!(stats.successful_processing, (N - FAILURES) as u64);
        assert_eq!(stats.total_faces_detected, (N - FAILURES) as u64);

        // avg is over the last 50 entries only (ring buffer cap).
        // Last 50 entries alternate 80/120 → avg = 100.
        assert_eq!(
            stats.avg_processing_time_ms(),
            100,
            "avg over last 50 entries must be 100 ms"
        );

        // Export time starts as None, is recorded after export.
        assert!(stats.last_export_time_ms.is_none());
        stats.record_export(312);
        assert_eq!(stats.last_export_time_ms, Some(312));
    }

    #[test]
    fn lazy_load_plan_queues_pages_after_initial_page() {
        let plan = BatchCoreState::build_lazy_load_plan(73, 20);
        assert_eq!(plan.total_files, 73);
        assert_eq!(plan.initial_load_count, 20);
        assert_eq!(plan.queued_files, 53);
        assert_eq!(plan.queued_pages, 3);
    }

    #[test]
    fn lazy_load_plan_handles_small_or_empty_sets() {
        let small = BatchCoreState::build_lazy_load_plan(8, 20);
        assert_eq!(small.initial_load_count, 8);
        assert_eq!(small.queued_files, 0);
        assert_eq!(small.queued_pages, 0);

        let empty = BatchCoreState::build_lazy_load_plan(0, 20);
        assert_eq!(empty.initial_load_count, 0);
        assert_eq!(empty.queued_files, 0);
        assert_eq!(empty.queued_pages, 0);
    }

    #[test]
    fn queue_state_loads_first_page_and_dequeues_remaining() {
        let ids = (1..=7).map(|i| format!("img_{i}")).collect::<Vec<_>>();
        let mut queue = BatchQueueState::from_files(&ids, 3);

        assert_eq!(queue.loaded_ids.len(), 3);
        assert_eq!(queue.queued_pages_count(), 2);
        assert_eq!(queue.queued_files_count(), 4);

        let first = queue.dequeue_next_page().unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(queue.queued_pages_count(), 1);

        let second = queue.dequeue_next_page().unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(queue.queued_pages_count(), 0);
        assert_eq!(queue.loaded_ids.len(), 7);
    }
}
