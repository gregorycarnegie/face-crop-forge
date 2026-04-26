# UI Improvements Checklist

Tracking layout and ergonomic fixes for the inner pages (Single / Batch / CSV).

## Layout bugs

- [x] **Width mismatch between header content and body grid**
      Both upload card and `.app-body` now share `max-width: 1680px`.

- [x] **Single mode: empty 1fr column when no image is loaded**
      Added `.side-by-side-container:has(> .canvas-container.hidden) { grid-template-columns: 1fr; }` so the cropped-faces panel takes full width when no image is loaded.

- [x] **Inconsistent layout language: home full-width vs. inner pages 1680px**
      Capped `.home-container` at `max-width: 1680px` to match inner pages.

- [x] **Invalid grid alignment value**
      Changed `align-items: flex-start` to `align-items: start` on `.app-body`.

## Ergonomic problems

- [x] **Collapse arrows don't actually collapse anything** *(quick win)*
      Added `Collapsible` and `CollapsibleSubsection` components in [src/panels.rs](src/panels.rs) with click + keyboard toggle, ARIA attributes, and rotating chevron. Refactored all 8 sites to use them. CSS now hides `.collapsible-content.collapsed`.

- [x] **Diagnostic info crowds out user settings**
      Split each inner page's workflow tools into two collapsibles: an expanded *Processing Status* with user-relevant runtime info, and a separate *Diagnostics* (default collapsed) holding MediaPipe Load Strategy / Offscreen Pipeline / Browser Fallback (Single), Rust Progress Status (Batch), and Rust CSV Runtime (CSV).

- [ ] **Long vertical scroll inside a 380px sidebar**
      Sidebar uses `max-height: calc(100vh - 108px); overflow-y: auto` ([css/styles.css:502-503](css/styles.css:502)) with 4–5 stacked panels. Internal scrollbar inside an already-narrow column.

- [ ] **Tabbed or accordion-only sidebar to eliminate scrolling** *(deferred)*
      Deferred. With working collapsibles + default-collapsed Diagnostics, the scroll length is already much shorter. Tabs force exclusivity (can't view Crop + Preprocessing simultaneously). Revisit if user feedback says scroll is still painful.

- [x] **Promote common actions to a sticky toolbar above the canvas**
      Single page: added `.workspace-toolbar` above the canvas with Detect Faces / Download Results / Clear Image (forward to existing buttons via `click_element_by_id`).
      Batch + CSV: made the existing top-level `.batch-controls` sticky to the viewport at `top: 60px`. Added a `:has()` rule to push the sidebar's sticky `.control-scroll` down to `top: 144px` so the two don't overlap.

- [x] **Settings panels duplicated across pages** *(re-scoped)*
      Re-evaluated: each panel is functionally needed on every page (users still want to set output format / naming template regardless of mode), and state is already shared via `AppState`, so removing any of them would degrade UX. Instead, defaulted `PreprocessingSettingsPanel` to `start_collapsed=true` everywhere — most users don't tweak preprocessing frequently, so collapsing it by default reduces scroll length. Crop + Output remain expanded as the primary inputs.
