use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CsvMapping {
    pub file_path_column: String,
    pub file_name_column: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CsvRow(pub HashMap<String, String>);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CsvCoreState {
    pub headers: Vec<String>,
    pub rows: Vec<CsvRow>,
    pub mapping: Option<CsvMapping>,
    pub filename_to_output: HashMap<String, String>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedUpload {
    pub file_name: String,
    pub output_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CsvExportNameContext<'a> {
    pub template: &'a str,
    pub csv_output_name: Option<&'a str>,
    pub original_file_name: &'a str,
    pub face_index: usize,
    pub timestamp_ms: u64,
    pub output_width: u32,
    pub output_height: u32,
    pub output_format: &'a str,
}

impl CsvCoreState {
    #[cfg(test)]
    pub fn set_headers(&mut self, headers: Vec<String>) {
        self.headers = headers;
        self.rows.clear();
        self.mapping = None;
        self.filename_to_output.clear();
    }

    pub fn parse_csv_line(line: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in line.chars() {
            if ch == '"' {
                in_quotes = !in_quotes;
            } else if ch == ',' && !in_quotes {
                result.push(current.trim().to_string());
                current.clear();
            } else {
                current.push(ch);
            }
        }

        result.push(current.trim().to_string());
        result
    }

    pub fn parse_csv_text(&mut self, csv_text: &str) -> bool {
        let lines: Vec<&str> = csv_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if lines.len() < 2 {
            return false;
        }

        let headers = Self::parse_csv_line(lines[0]);
        if headers.is_empty() {
            return false;
        }

        let mut rows = Vec::new();
        for line in &lines[1..] {
            let values = Self::parse_csv_line(line);
            if values.len() != headers.len() {
                continue;
            }
            let row_map = headers
                .iter()
                .cloned()
                .zip(values)
                .collect::<HashMap<_, _>>();
            rows.push(CsvRow(row_map));
        }

        self.headers = headers;
        self.rows = rows;
        self.mapping = None;
        self.filename_to_output.clear();
        true
    }

    pub fn can_confirm_mapping(&self, file_path_column: &str, file_name_column: &str) -> bool {
        if file_path_column.is_empty() || file_name_column.is_empty() {
            return false;
        }
        if file_path_column == file_name_column {
            return false;
        }
        self.headers.iter().any(|h| h == file_path_column)
            && self.headers.iter().any(|h| h == file_name_column)
    }

    pub fn apply_mapping(&mut self, file_path_column: &str, file_name_column: &str) -> bool {
        if !self.can_confirm_mapping(file_path_column, file_name_column) {
            return false;
        }

        self.mapping = Some(CsvMapping {
            file_path_column: file_path_column.to_string(),
            file_name_column: file_name_column.to_string(),
        });
        self.rebuild_filename_mapping();
        true
    }

    #[cfg(test)]
    pub fn mapped_file_name<'a>(&self, row: &'a CsvRow) -> Option<&'a str> {
        let mapping = self.mapping.as_ref()?;
        row.0.get(&mapping.file_name_column).map(|s| s.as_str())
    }

    #[cfg(test)]
    pub fn mapped_file_path<'a>(&self, row: &'a CsvRow) -> Option<&'a str> {
        let mapping = self.mapping.as_ref()?;
        row.0.get(&mapping.file_path_column).map(|s| s.as_str())
    }

    #[cfg(test)]
    pub fn build_output_name(&self, row: &CsvRow, fallback_original_name: &str) -> String {
        self.mapped_file_name(row)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback_original_name)
            .to_string()
    }

    fn extract_file_name(path: &str) -> String {
        path.rsplit(['/', '\\'])
            .next()
            .unwrap_or(path)
            .to_lowercase()
    }

    fn without_extension(file_name: &str) -> String {
        match file_name.rsplit_once('.') {
            Some((base, _)) => base.to_string(),
            None => file_name.to_string(),
        }
    }

    pub fn rebuild_filename_mapping(&mut self) {
        self.filename_to_output.clear();
        let Some(mapping) = self.mapping.as_ref() else {
            return;
        };

        for row in &self.rows {
            let Some(file_path) = row.0.get(&mapping.file_path_column) else {
                continue;
            };
            let Some(output_name) = row.0.get(&mapping.file_name_column) else {
                continue;
            };
            let trimmed_output = output_name.trim();
            if trimmed_output.is_empty() {
                continue;
            }
            let file_name = Self::extract_file_name(file_path);
            if !file_name.is_empty() {
                self.filename_to_output
                    .insert(file_name, trimmed_output.to_string());
            }
        }
    }

    #[cfg(test)]
    pub fn match_uploaded_files(&self, uploaded_file_names: &[String]) -> Vec<MatchedUpload> {
        let mut matched = Vec::new();
        for file_name in uploaded_file_names {
            let output_name = self.output_name_for_file(file_name);

            if let Some(output_name) = output_name {
                matched.push(MatchedUpload {
                    file_name: file_name.clone(),
                    output_name,
                });
            }
        }
        matched
    }

    pub fn output_name_for_file(&self, file_name: &str) -> Option<String> {
        let normalized = file_name.to_lowercase();
        let normalized_no_ext = Self::without_extension(&normalized);

        self.filename_to_output
            .get(&normalized)
            .cloned()
            .or_else(|| self.filename_to_output.get(&normalized_no_ext).cloned())
            .or_else(|| {
                self.filename_to_output
                    .iter()
                    .find_map(|(csv_name, output)| {
                        let csv_no_ext = Self::without_extension(csv_name);
                        (csv_no_ext == normalized_no_ext).then(|| output.clone())
                    })
            })
    }

    pub fn generate_export_filename(ctx: CsvExportNameContext<'_>) -> String {
        let template = if ctx.template.trim().is_empty() {
            "{csv_name}"
        } else {
            ctx.template
        };
        let original = Self::without_extension(ctx.original_file_name);
        let csv_name = ctx
            .csv_output_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&original);
        let extension = if ctx.output_format.eq_ignore_ascii_case("jpeg") {
            "jpg"
        } else {
            ctx.output_format
        };

        let stem = template
            .replace("{csv_name}", csv_name)
            .replace("{original}", &original)
            .replace("{index}", &(ctx.face_index + 1).to_string())
            .replace("{timestamp}", &ctx.timestamp_ms.to_string())
            .replace("{width}", &ctx.output_width.to_string())
            .replace("{height}", &ctx.output_height.to_string());

        format!("{stem}.{extension}")
    }

    #[cfg(test)]
    pub fn preview_rows(&self, max_rows: usize) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .take(max_rows)
            .map(|row| {
                self.headers
                    .iter()
                    .map(|header| row.0.get(header).cloned().unwrap_or_default())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(path: &str, name: &str) -> CsvRow {
        let mut map = HashMap::new();
        map.insert("path".to_string(), path.to_string());
        map.insert("output_name".to_string(), name.to_string());
        CsvRow(map)
    }

    #[test]
    fn mapping_requires_known_headers() {
        let mut state = CsvCoreState::default();
        state.set_headers(vec!["path".into(), "output_name".into()]);
        assert!(state.apply_mapping("path", "output_name"));
        assert!(!state.apply_mapping("missing", "output_name"));
        assert!(!state.apply_mapping("path", "path"));
    }

    #[test]
    fn mapped_values_are_extracted() {
        let mut state = CsvCoreState::default();
        state.set_headers(vec!["path".into(), "output_name".into()]);
        state.apply_mapping("path", "output_name");

        let row = make_row("images/a.png", "person_a");
        assert_eq!(state.mapped_file_path(&row), Some("images/a.png"));
        assert_eq!(state.mapped_file_name(&row), Some("person_a"));
    }

    #[test]
    fn output_name_falls_back_when_mapped_name_empty() {
        let mut state = CsvCoreState::default();
        state.set_headers(vec!["path".into(), "output_name".into()]);
        state.apply_mapping("path", "output_name");

        let row_empty = make_row("images/a.png", "");
        assert_eq!(state.build_output_name(&row_empty, "a"), "a");

        let row_named = make_row("images/a.png", "custom_a");
        assert_eq!(state.build_output_name(&row_named, "a"), "custom_a");
    }

    #[test]
    fn csv_text_parse_builds_headers_and_rows() {
        let mut state = CsvCoreState::default();
        let ok = state.parse_csv_text(
            "file_path,output_name\nimages/A.jpg,alice\n\"images/B,alt.jpg\",bob\ninvalid_only",
        );
        assert!(ok);
        assert_eq!(state.headers, vec!["file_path", "output_name"]);
        assert_eq!(state.rows.len(), 2);
        assert_eq!(
            state.rows[1].0.get("file_path").map(String::as_str),
            Some("images/B,alt.jpg")
        );
    }

    #[test]
    fn uploaded_file_matching_uses_filename_and_no_extension() {
        let mut state = CsvCoreState::default();
        assert!(state.parse_csv_text(
            "file_path,output_name\nimages/person_a.jpg,Alice\nimages/person_b,Bruno"
        ));
        assert!(state.apply_mapping("file_path", "output_name"));

        let matched = state.match_uploaded_files(&[
            "person_a.jpg".to_string(),
            "person_b.png".to_string(),
            "no_match.jpeg".to_string(),
        ]);

        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].output_name, "Alice");
        assert_eq!(matched[1].output_name, "Bruno");
    }

    #[test]
    fn parse_csv_rejects_missing_data_rows() {
        let mut state = CsvCoreState::default();
        assert!(!state.parse_csv_text("file_path,output_name"));
    }

    #[test]
    fn duplicate_csv_filenames_last_row_wins() {
        let mut state = CsvCoreState::default();
        assert!(state.parse_csv_text(
            "file_path,output_name\nimages/person_a.jpg,Alpha\nimages/person_a.jpg,Latest"
        ));
        assert!(state.apply_mapping("file_path", "output_name"));

        let matched = state.match_uploaded_files(&["person_a.jpg".to_string()]);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].output_name, "Latest");
    }

    #[test]
    fn windows_style_paths_and_case_insensitive_match_work() {
        let mut state = CsvCoreState::default();
        assert!(state.parse_csv_text("file_path,output_name\nC:\\\\photos\\\\Person_A.PNG,Alice"));
        assert!(state.apply_mapping("file_path", "output_name"));

        let matched = state.match_uploaded_files(&["person_a.png".to_string()]);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].output_name, "Alice");
    }

    #[test]
    fn export_filename_template_tokens_expand_like_ts_logic() {
        let filename = CsvCoreState::generate_export_filename(CsvExportNameContext {
            template: "{csv_name}_{original}_{index}_{width}x{height}_{timestamp}",
            csv_output_name: Some("alice"),
            original_file_name: "person_a.png",
            face_index: 1,
            timestamp_ms: 123456,
            output_width: 512,
            output_height: 768,
            output_format: "jpeg",
        });
        assert_eq!(filename, "alice_person_a_2_512x768_123456.jpg");
    }

    #[test]
    fn export_filename_defaults_to_csv_name_template_and_fallbacks_to_original() {
        let filename = CsvCoreState::generate_export_filename(CsvExportNameContext {
            template: "",
            csv_output_name: Some(""),
            original_file_name: "model.final.png",
            face_index: 0,
            timestamp_ms: 1,
            output_width: 100,
            output_height: 100,
            output_format: "png",
        });
        assert_eq!(filename, "model.final.png");
    }

    #[test]
    fn export_filename_preserves_output_format_extensions() {
        let png = CsvCoreState::generate_export_filename(CsvExportNameContext {
            template: "{csv_name}",
            csv_output_name: Some("alice"),
            original_file_name: "input.jpeg",
            face_index: 0,
            timestamp_ms: 0,
            output_width: 512,
            output_height: 512,
            output_format: "png",
        });
        let jpg = CsvCoreState::generate_export_filename(CsvExportNameContext {
            template: "{csv_name}",
            csv_output_name: Some("alice"),
            original_file_name: "input.jpeg",
            face_index: 0,
            timestamp_ms: 0,
            output_width: 512,
            output_height: 512,
            output_format: "jpeg",
        });
        let webp = CsvCoreState::generate_export_filename(CsvExportNameContext {
            template: "{csv_name}",
            csv_output_name: Some("alice"),
            original_file_name: "input.jpeg",
            face_index: 0,
            timestamp_ms: 0,
            output_width: 512,
            output_height: 512,
            output_format: "webp",
        });

        assert_eq!(png, "alice.png");
        assert_eq!(jpg, "alice.jpg");
        assert_eq!(webp, "alice.webp");
    }

    #[test]
    fn output_name_for_file_resolves_same_as_batch_matching() {
        let mut state = CsvCoreState::default();
        assert!(state.parse_csv_text(
            "file_path,output_name\nimages/person_a.jpg,Alice\nimages/person_b,Bruno"
        ));
        assert!(state.apply_mapping("file_path", "output_name"));

        assert_eq!(
            state.output_name_for_file("person_a.jpg").as_deref(),
            Some("Alice")
        );
        assert_eq!(
            state.output_name_for_file("person_b.png").as_deref(),
            Some("Bruno")
        );
        assert_eq!(state.output_name_for_file("missing.jpeg"), None);
    }

    #[test]
    fn preview_rows_follow_header_order_and_limit() {
        let mut state = CsvCoreState::default();
        assert!(state.parse_csv_text(
            "file_path,output_name\nimages/a.jpg,Alice\nimages/b.jpg,Bruno\nimages/c.jpg,Carla"
        ));

        let preview = state.preview_rows(2);
        assert_eq!(preview.len(), 2);
        assert_eq!(
            preview[0],
            vec!["images/a.jpg".to_string(), "Alice".to_string()]
        );
        assert_eq!(
            preview[1],
            vec!["images/b.jpg".to_string(), "Bruno".to_string()]
        );
    }
}
