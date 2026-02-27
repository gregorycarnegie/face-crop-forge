use crate::batch_core::BatchCoreState;
use crate::csv_core::{CsvCoreState, CsvExportNameContext};
use crate::single_core::generate_face_filename;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
struct PerfMeasurement {
    name: &'static str,
    rust_ms: u128,
    ts_baseline_ms: u128,
}

impl PerfMeasurement {
    fn faster_pct(&self) -> i128 {
        if self.ts_baseline_ms == 0 {
            return 0;
        }
        let delta = self.ts_baseline_ms as i128 - self.rust_ms as i128;
        (delta * 100) / self.ts_baseline_ms as i128
    }
}

fn benchmark_batch_work_plan() -> u128 {
    let mut state = BatchCoreState::default();
    let ids = (0..10_000).map(|idx| format!("img_{idx}.jpg")).collect();
    state.set_images(ids);

    let start = Instant::now();
    for _ in 0..50 {
        let plan = state.build_work_plan(128);
        assert_eq!(plan.selected_total, 10_000);
    }
    start.elapsed().as_millis()
}

fn benchmark_csv_parse_and_match() -> u128 {
    let mut lines = vec!["file_path,output_name".to_string()];
    for idx in 0..5000 {
        lines.push(format!("images/p_{idx}.jpg,person_{idx}"));
    }
    let csv_text = lines.join("\n");
    let uploads = (0..5000)
        .map(|idx| format!("p_{idx}.jpg"))
        .collect::<Vec<_>>();

    let start = Instant::now();
    for _ in 0..10 {
        let mut state = CsvCoreState::default();
        assert!(state.parse_csv_text(&csv_text));
        assert!(state.apply_mapping("file_path", "output_name"));
        let matched = state.match_uploaded_files(&uploads);
        assert_eq!(matched.len(), 5000);
    }
    start.elapsed().as_millis()
}

fn benchmark_export_filename_generation() -> u128 {
    let start = Instant::now();
    for idx in 0..100_000 {
        let _ = generate_face_filename(
            "face_{original}_{index}_{width}x{height}_{timestamp}",
            "portrait.png",
            idx,
            512,
            512,
            if idx % 3 == 0 {
                "png"
            } else if idx % 3 == 1 {
                "jpeg"
            } else {
                "webp"
            },
            1_706_000_000 + idx as u64,
        );
    }
    start.elapsed().as_millis()
}

fn collect_perf_snapshot() -> Vec<PerfMeasurement> {
    vec![
        // Baselines below are conservative budgets captured from pre-migration TS behavior
        // for equivalent-scale workflows and used as regression guardrails.
        PerfMeasurement {
            name: "Batch work-plan generation (10k images x50)",
            rust_ms: benchmark_batch_work_plan(),
            ts_baseline_ms: 900,
        },
        PerfMeasurement {
            name: "CSV parse+map+match (5k rows x10)",
            rust_ms: benchmark_csv_parse_and_match(),
            ts_baseline_ms: 1400,
        },
        PerfMeasurement {
            name: "Export filename generation (100k)",
            rust_ms: benchmark_export_filename_generation(),
            ts_baseline_ms: 700,
        },
    ]
}

#[test]
fn performance_snapshot_vs_typescript_baseline_budget() {
    let snapshot = collect_perf_snapshot();
    for measurement in &snapshot {
        println!(
            "[perf] {} => rust={}ms baseline={}ms delta={}%",
            measurement.name,
            measurement.rust_ms,
            measurement.ts_baseline_ms,
            measurement.faster_pct()
        );
        assert!(
            measurement.rust_ms <= measurement.ts_baseline_ms,
            "performance regression for {}: rust {}ms > baseline {}ms",
            measurement.name,
            measurement.rust_ms,
            measurement.ts_baseline_ms
        );
    }
}

#[test]
fn download_format_quality_regression_guardrails() {
    let mut csv = CsvCoreState::default();
    assert!(csv.parse_csv_text("file_path,output_name\nimages/a.jpg,Alice"));
    assert!(csv.apply_mapping("file_path", "output_name"));
    let name = csv
        .output_name_for_file("a.jpg")
        .unwrap_or_else(|| "fallback".to_string());

    let png = CsvCoreState::generate_export_filename(CsvExportNameContext {
        template: "{csv_name}_{index}",
        csv_output_name: Some(&name),
        original_file_name: "a.jpg",
        face_index: 0,
        timestamp_ms: 1,
        output_width: 512,
        output_height: 512,
        output_format: "png",
    });
    let jpg = CsvCoreState::generate_export_filename(CsvExportNameContext {
        template: "{csv_name}_{index}",
        csv_output_name: Some(&name),
        original_file_name: "a.jpg",
        face_index: 0,
        timestamp_ms: 1,
        output_width: 512,
        output_height: 512,
        output_format: "jpeg",
    });
    let webp = CsvCoreState::generate_export_filename(CsvExportNameContext {
        template: "{csv_name}_{index}",
        csv_output_name: Some(&name),
        original_file_name: "a.jpg",
        face_index: 0,
        timestamp_ms: 1,
        output_width: 512,
        output_height: 512,
        output_format: "webp",
    });

    assert_eq!(png, "Alice_1.png");
    assert_eq!(jpg, "Alice_1.jpg");
    assert_eq!(webp, "Alice_1.webp");
}
