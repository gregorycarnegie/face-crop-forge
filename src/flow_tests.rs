use crate::batch_core::{BatchCoreState, BatchQueueState, BatchRuntimeStats};
use crate::batch_export::BatchProgress;
use crate::crop_math::{CropSettings, FaceBox, PositioningMode, compute_crop_region};
use crate::csv_core::{CsvCoreState, CsvExportNameContext};
use crate::export_runtime::{
    build_zip_bytes, normalize_export_filename_for_mime, validate_export_filename_for_mime,
};
use crate::preprocessing::{Rgba, apply_contrast, apply_exposure, apply_sharpness_scalar};
use crate::quality_filters::{
    QualityFilterSettings, QualityLevel, classify_blur_quality, is_face_accepted,
};
use crate::single_core::build_export_plan;
use crate::worker_bridge::DetectedFace;
use std::collections::HashSet;
use std::io::Cursor;
use zip::ZipArchive;

#[test]
fn batch_drag_drop_queue_contract_preserves_all_files_across_pages() {
    let ids = (1..=45).map(|i| format!("img_{i}.jpg")).collect::<Vec<_>>();
    let mut queue = BatchQueueState::from_files(ids, 20);
    let mut state = BatchCoreState::default();
    state.set_images(queue.loaded_ids.clone());

    assert_eq!(state.total_count(), 20);
    assert_eq!(queue.queued_pages_count(), 2);
    assert_eq!(queue.queued_files_count(), 25);

    while let Some(page) = queue.dequeue_next_page() {
        state.add_images(page);
    }
    assert_eq!(state.total_count(), 45);
    assert_eq!(queue.queued_pages_count(), 0);

    let plan = state.build_work_plan(16);
    assert_eq!(plan.selected_total, 45);
    assert_eq!(plan.chunks.len(), 3);

    for id in state.selected_ids() {
        state.mark_processing(&id);
    }
    let mut progress = BatchProgress::default();
    progress.start(plan.selected_total, "Processing batch flow");
    let mut stats = BatchRuntimeStats::default();
    for _ in 0..plan.selected_total {
        progress.record_result(true);
        stats.record_image(20, 2, true);
    }
    for id in state.selected_ids() {
        state.mark_processed(&id);
    }
    progress.complete("Done");

    assert_eq!(progress.percent(), 100);
    assert_eq!(stats.images_processed, 45);
    assert_eq!(stats.success_rate_pct(), 100);
    assert_eq!(stats.avg_processing_time_ms(), 20);

    let unique_ids = state.selected_ids().into_iter().collect::<HashSet<_>>();
    assert_eq!(unique_ids.len(), 45);
}

#[test]
fn csv_mapping_and_export_name_contract_uses_real_output_names() {
    let mut csv = CsvCoreState::default();
    assert!(csv.parse_csv_text(
        "file_path,output_name\nimages/a.jpg,Alice\nimages/b.jpg,Bruno\nimages/c.jpg,Carla"
    ));
    assert!(csv.apply_mapping("file_path", "output_name"));

    let preview = csv.preview_rows(2);
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0][1], "Alice");

    let matched = csv.match_uploaded_files(&[
        "a.jpg".to_string(),
        "b.png".to_string(),
        "missing.jpg".to_string(),
    ]);
    assert_eq!(matched.len(), 2);

    let mut queue = BatchQueueState::from_files(
        matched
            .iter()
            .map(|m| m.file_name.clone())
            .collect::<Vec<_>>(),
        1,
    );
    let mut state = BatchCoreState::default();
    state.set_images(queue.loaded_ids.clone());
    while let Some(page) = queue.dequeue_next_page() {
        state.add_images(page);
    }
    assert_eq!(state.total_count(), 2);

    let plan = state.build_work_plan(8);
    let mut progress = BatchProgress::default();
    progress.start(plan.selected_total, "Processing CSV flow");
    let mut stats = BatchRuntimeStats::default();
    for _ in 0..plan.selected_total {
        progress.record_result(true);
        stats.record_image(18, 2, true);
    }
    progress.complete("Done");

    assert_eq!(progress.percent(), 100);
    assert_eq!(stats.images_processed, 2);

    let file_names = state
        .selected_ids()
        .iter()
        .enumerate()
        .map(|(idx, file_name)| {
            let output_name = csv.output_name_for_file(file_name);
            CsvCoreState::generate_export_filename(CsvExportNameContext {
                template: "{csv_name}_{index}",
                csv_output_name: output_name.as_deref(),
                original_file_name: file_name,
                face_index: idx,
                timestamp_ms: 0,
                output_width: 512,
                output_height: 512,
                output_format: "png",
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(file_names.len(), 2);
    assert!(file_names[0].contains("Alice_1"));
    assert!(file_names[1].contains("Bruno_2"));
}

#[test]
fn detection_to_crop_to_export_contract_produces_valid_zip_artifacts() {
    let faces = vec![
        DetectedFace {
            id: "face_1".to_string(),
            x: 120.0,
            y: 90.0,
            width: 80.0,
            height: 100.0,
            confidence: 0.95,
        },
        DetectedFace {
            id: "face_2".to_string(),
            x: 20.0,
            y: 30.0,
            width: 60.0,
            height: 70.0,
            confidence: 0.45,
        },
    ];

    let filter = QualityFilterSettings {
        min_confidence: 0.8,
        min_quality_score: 300.0,
        min_quality_level: QualityLevel::Medium,
    };

    let accepted = faces
        .into_iter()
        .filter(|face| {
            let quality = classify_blur_quality(900.0);
            is_face_accepted(face.confidence as f32, Some(quality), filter)
        })
        .collect::<Vec<_>>();
    assert_eq!(accepted.len(), 1);

    let crop = compute_crop_region(
        1200.0,
        800.0,
        FaceBox {
            x_min: accepted[0].x as f32,
            y_min: accepted[0].y as f32,
            width: accepted[0].width as f32,
            height: accepted[0].height as f32,
        },
        CropSettings {
            output_width: 512.0,
            output_height: 512.0,
            face_height_pct: 70.0,
            positioning_mode: PositioningMode::Center,
            vertical_offset_pct: 0.0,
            horizontal_offset_pct: 0.0,
        },
    );
    assert!(crop.width > 0.0 && crop.height > 0.0);
    assert!(crop.x >= 0.0 && crop.y >= 0.0);

    let face_ids = accepted
        .iter()
        .map(|face| face.id.clone())
        .collect::<HashSet<_>>();
    let export_plan = build_export_plan(
        &face_ids,
        "face_{original}_{index}",
        "portrait.jpg",
        512,
        512,
        "png",
        1_735_689_600_000,
    );
    assert_eq!(export_plan.filenames.len(), 1);

    let px = Rgba {
        r: 120,
        g: 130,
        b: 140,
        a: 255,
    };
    let enhanced = apply_sharpness_scalar(apply_contrast(apply_exposure(px, 0.5), 1.2), 1.0);
    let fake_png_bytes = vec![enhanced.r, enhanced.g, enhanced.b, enhanced.a];
    let final_name = normalize_export_filename_for_mime(&export_plan.filenames[0], "image/png");
    assert!(validate_export_filename_for_mime(&final_name, "image/png"));

    let zip_bytes = build_zip_bytes(&[(final_name.clone(), fake_png_bytes.clone())])
        .expect("zip bytes should be produced");
    let mut archive =
        ZipArchive::new(Cursor::new(zip_bytes)).expect("zip archive should be readable");
    assert_eq!(archive.len(), 1);
    let mut first = archive.by_index(0).expect("first entry should exist");
    assert_eq!(first.name(), final_name);
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut first, &mut out).expect("entry bytes should be readable");
    assert_eq!(out, fake_png_bytes);
}

#[test]
fn webcam_capture_flow_contract_closes_modal_and_keeps_detection_selection() {
    use crate::single_core::SingleCoreState;

    let mut state = SingleCoreState::default();
    state.open_webcam_modal();
    assert!(state.webcam_modal_open);

    state.set_faces(vec!["face_a".to_string(), "face_b".to_string()]);
    assert_eq!(state.faces_count(), 2);
    assert_eq!(state.selected_count(), 2);

    state.close_webcam_modal();
    assert!(!state.webcam_modal_open);
    assert_eq!(state.faces_count(), 2);
    assert_eq!(state.selected_count(), 2);
}
