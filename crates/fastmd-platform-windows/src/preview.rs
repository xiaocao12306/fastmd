use std::fmt;

use fastmd_contracts::{
    AppCommand, AppEvent, DocumentKind, DocumentOrigin, HoverResolutionScope, HoveredItem,
    PreviewState, ResolvedDocument, ScreenPoint,
};
use fastmd_core::CoreEngine;
use fastmd_render::{
    apply_inline_edit_to_markdown, build_inline_editor_model_for_editing_state, BlockMapping,
    InlineEditorModel,
};

use crate::{
    AcceptedMarkdownPath, AdapterError, CoordinateProbeError, ExplorerAdapter, FrontmostProbeError,
    FrontmostSurfaceProbe, HoverProbeError, HoveredItemProbeOutcome, WindowsCoordinateTranslation,
};

/// Windows-specific preview loop wiring that feeds Explorer host signals into
/// the shared FastMD core without changing product semantics.
#[derive(Debug, Default)]
pub struct WindowsPreviewLoop {
    adapter: ExplorerAdapter,
    engine: CoreEngine,
}

/// Errors surfaced while translating Windows host probes into shared-core
/// preview events.
#[derive(Debug)]
pub enum PreviewLoopError {
    Adapter(AdapterError),
    FrontmostProbe(FrontmostProbeError),
    HoverProbe(HoverProbeError),
    CoordinateProbe(CoordinateProbeError),
    MissingRequiredProbeOutput { probe: &'static str },
}

impl fmt::Display for PreviewLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adapter(error) => write!(f, "{error}"),
            Self::FrontmostProbe(error) => write!(f, "{error}"),
            Self::HoverProbe(error) => write!(f, "{error}"),
            Self::CoordinateProbe(error) => write!(f, "{error}"),
            Self::MissingRequiredProbeOutput { probe } => write!(
                f,
                "missing required Windows {probe} probe output while Explorer is frontmost"
            ),
        }
    }
}

impl std::error::Error for PreviewLoopError {}

impl WindowsPreviewLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &PreviewState {
        self.engine.state()
    }

    pub fn dispatch_command(
        &mut self,
        command: AppCommand,
        blocks: &[BlockMapping],
    ) -> Vec<AppEvent> {
        self.engine.dispatch_command(command, blocks)
    }

    pub fn request_edit_at_line(
        &mut self,
        target_line: u32,
        markdown: &str,
        blocks: &[BlockMapping],
    ) -> Option<InlineEditorModel> {
        let events = self.dispatch_command(AppCommand::RequestEdit { target_line }, blocks);
        if !matches!(events.as_slice(), [AppEvent::EditSessionChanged { .. }]) {
            return None;
        }

        self.inline_editor(markdown, blocks)
    }

    pub fn inline_editor(
        &self,
        markdown: &str,
        blocks: &[BlockMapping],
    ) -> Option<InlineEditorModel> {
        build_inline_editor_model_for_editing_state(markdown, blocks, &self.engine.state().editing)
    }

    pub fn save_current_edit(
        &mut self,
        markdown: &str,
        replacement_source: &str,
        blocks: &[BlockMapping],
    ) -> Option<(String, Vec<AppEvent>)> {
        let block = self.engine.editing_block(blocks)?;
        let replacement_markdown =
            apply_inline_edit_to_markdown(markdown, &block, replacement_source)?;
        let events = self.dispatch_command(
            AppCommand::SaveEdit {
                replacement_markdown: replacement_markdown.clone(),
                replacement_source: replacement_source.replace("\r\n", "\n"),
            },
            blocks,
        );
        if events.is_empty() {
            return None;
        }

        Some((replacement_markdown, events))
    }

    pub fn cancel_edit_session(&mut self) -> Vec<AppEvent> {
        self.dispatch_command(AppCommand::CancelEdit, &[])
    }

    pub fn complete_save(
        &mut self,
        success: bool,
        persisted_markdown: Option<String>,
        message: Option<String>,
    ) -> Vec<AppEvent> {
        self.dispatch_command(
            AppCommand::CompleteSave {
                success,
                persisted_markdown,
                message,
            },
            &[],
        )
    }

    /// Polls the live Windows host and forwards the resulting Explorer facts
    /// into the shared core.
    pub fn poll_host_state(&mut self, at_ms: u64) -> Result<Vec<AppEvent>, PreviewLoopError> {
        let frontmost = self
            .adapter
            .probe_frontmost_surface()
            .map_err(PreviewLoopError::Adapter)?;

        if !frontmost.allowed {
            return Ok(self
                .engine
                .observe_hover(at_ms, frontmost.observed_surface, None, None));
        }

        let translation = self
            .adapter
            .translate_coordinates(ScreenPoint::new(0.0, 0.0))
            .map_err(PreviewLoopError::Adapter)?;
        let front_surface = frontmost
            .detected_surface
            .as_ref()
            .expect("allowed Explorer frontmost probe should carry the accepted surface")
            .clone();
        let hover = self
            .adapter
            .resolve_hovered_item(&front_surface, translation.cursor.clone())
            .map_err(PreviewLoopError::Adapter)?;

        Ok(self.observe_classified_state(at_ms, frontmost, hover, translation))
    }

    /// Test-friendly entrypoint that accepts already-captured probe outputs so
    /// this lane can validate the Windows host-to-core wiring off Windows.
    pub fn observe_probe_outputs(
        &mut self,
        at_ms: u64,
        frontmost_raw: &str,
        hover_raw: Option<&str>,
        coordinate_raw: Option<&str>,
    ) -> Result<Vec<AppEvent>, PreviewLoopError> {
        let frontmost = self
            .adapter
            .classify_frontmost_surface_from_probe_output(frontmost_raw)
            .map_err(PreviewLoopError::FrontmostProbe)?;

        if !frontmost.allowed {
            return Ok(self
                .engine
                .observe_hover(at_ms, frontmost.observed_surface, None, None));
        }

        let hover_raw =
            hover_raw.ok_or(PreviewLoopError::MissingRequiredProbeOutput { probe: "hover" })?;
        let coordinate_raw =
            coordinate_raw.ok_or(PreviewLoopError::MissingRequiredProbeOutput {
                probe: "coordinate",
            })?;

        let hover = self
            .adapter
            .classify_hovered_item_from_probe_output(hover_raw)
            .map_err(PreviewLoopError::HoverProbe)?;
        let translation = self
            .adapter
            .classify_coordinate_translation_from_probe_output(coordinate_raw)
            .map_err(PreviewLoopError::CoordinateProbe)?;

        Ok(self.observe_classified_state(at_ms, frontmost, hover, translation))
    }

    fn observe_classified_state(
        &mut self,
        at_ms: u64,
        frontmost: FrontmostSurfaceProbe,
        hover: HoveredItemProbeOutcome,
        translation: WindowsCoordinateTranslation,
    ) -> Vec<AppEvent> {
        let hovered_item = hover
            .accepted
            .as_ref()
            .map(|accepted| hovered_item_from_probe(accepted, &hover, &translation));

        self.dispatch_command(
            AppCommand::ObserveHover {
                at_ms,
                front_surface: frontmost.observed_surface,
                hovered_item,
                monitor: Some(translation.selected_monitor),
            },
            &[],
        )
    }
}

fn hovered_item_from_probe(
    accepted: &AcceptedMarkdownPath,
    hover: &HoveredItemProbeOutcome,
    translation: &WindowsCoordinateTranslation,
) -> HoveredItem {
    HoveredItem {
        document: resolved_document_from_accepted(accepted),
        screen_point: translation.cursor.clone(),
        element_description: hovered_item_description(hover),
    }
}

fn resolved_document_from_accepted(accepted: &AcceptedMarkdownPath) -> ResolvedDocument {
    let path = accepted.path();
    let display_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string());

    ResolvedDocument::new(
        path.display().to_string(),
        display_name,
        DocumentOrigin::LocalFileSystem,
        DocumentKind::File,
    )
}

fn hovered_item_description(hover: &HoveredItemProbeOutcome) -> String {
    let snapshot = &hover.snapshot;
    let element_name = snapshot
        .element_name
        .as_deref()
        .map(|name| format!(" ({name})"))
        .unwrap_or_default();

    format!(
        "Windows Explorer {} via {}{}",
        hover_scope_label(snapshot.resolution_scope),
        snapshot.backend,
        element_name
    )
}

fn hover_scope_label(scope: HoverResolutionScope) -> &'static str {
    match scope {
        HoverResolutionScope::ExactItemUnderPointer => "exact-item-under-pointer",
        HoverResolutionScope::HoveredRowDescendant => "hovered-row-descendant",
        HoverResolutionScope::NearbyCandidate => "nearby-candidate",
        HoverResolutionScope::FirstVisibleItem => "first-visible-item",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use fastmd_contracts::{
        AppCommand, AppEvent, BackgroundMode, CloseReason, EditingPhase,
        MACOS_REFERENCE_BEHAVIOR, PageDirection, PageInput,
    };
    use fastmd_render::{BlockKind, BlockMapping};
    use serde_json::json;

    use super::WindowsPreviewLoop;

    #[derive(Debug)]
    struct TempFixture {
        root: PathBuf,
    }

    impl TempFixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "fastmd-platform-windows-preview-{nonce}-{}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("temp directory should be created");
            Self { root }
        }

        fn write_file(&self, relative_path: impl AsRef<Path>, contents: &str) -> PathBuf {
            let path = self.root.join(relative_path);
            fs::write(&path, contents).expect("temp file should be written");
            path
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn explorer_frontmost_json() -> String {
        json!({
            "foreground_window_id": "hwnd:0x10001",
            "process_id": 4012,
            "process_image_name": r"C:\Windows\explorer.exe",
            "window_class": "CabinetWClass",
            "window_title": "Docs",
            "directory": r"C:\Users\example\Docs",
            "shell_window_id": "hwnd:0x10001"
        })
        .to_string()
    }

    fn non_explorer_frontmost_json() -> String {
        json!({
            "foreground_window_id": "hwnd:0x10002",
            "process_id": 777,
            "process_image_name": r"C:\Windows\System32\notepad.exe",
            "window_class": "Notepad",
            "window_title": "notes.txt",
            "shell_window_id": null
        })
        .to_string()
    }

    fn hovered_item_json(path: &Path, scope: &str) -> String {
        json!({
            "resolution_scope": scope,
            "backend": "uiautomation-element-from-point+shell-parse-name",
            "path": path.display().to_string(),
            "element_name": path.file_name().and_then(|name| name.to_str()).unwrap_or("notes.md"),
            "shell_window_id": "hwnd:0x10001"
        })
        .to_string()
    }

    fn coordinate_json(cursor_x: f64, cursor_y: f64) -> String {
        json!({
            "cursor": {
                "x": cursor_x,
                "y": cursor_y
            },
            "virtual_desktop": {
                "x": 0.0,
                "y": 0.0,
                "width": 1920.0,
                "height": 1080.0
            },
            "monitors": [
                {
                    "id": "primary",
                    "name": "Primary",
                    "is_primary": true,
                    "scale_factor": 1.0,
                    "frame": {
                        "x": 0.0,
                        "y": 0.0,
                        "width": 1920.0,
                        "height": 1080.0
                    },
                    "working_area": {
                        "x": 0.0,
                        "y": 0.0,
                        "width": 1920.0,
                        "height": 1040.0
                    }
                }
            ]
        })
        .to_string()
    }

    fn coordinate_json_for_visible_frame(
        cursor_x: f64,
        shared_cursor_y: f64,
        width: f64,
        height: f64,
    ) -> String {
        json!({
            "cursor": {
                "x": cursor_x,
                "y": height - shared_cursor_y
            },
            "virtual_desktop": {
                "x": 0.0,
                "y": 0.0,
                "width": width,
                "height": height
            },
            "monitors": [
                {
                    "id": "primary",
                    "name": "Primary",
                    "is_primary": true,
                    "scale_factor": 1.0,
                    "frame": {
                        "x": 0.0,
                        "y": 0.0,
                        "width": width,
                        "height": height
                    },
                    "working_area": {
                        "x": 0.0,
                        "y": 0.0,
                        "width": width,
                        "height": height
                    }
                }
            ]
        })
        .to_string()
    }

    fn open_visible_preview(preview: &mut WindowsPreviewLoop, path: &Path) {
        assert!(
            preview
                .observe_probe_outputs(
                    0,
                    &explorer_frontmost_json(),
                    Some(&hovered_item_json(path, "exact-item-under-pointer")),
                    Some(&coordinate_json(320.0, 180.0)),
                )
                .expect("probe outputs should classify")
                .is_empty()
        );

        let opened = preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        assert!(matches!(
            opened.as_slice(),
            [AppEvent::PreviewWindowRequested { .. }]
        ));
        assert!(preview.state().visibility.visible);
    }

    fn edit_markdown() -> &'static str {
        "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10"
    }

    fn edit_block_mappings() -> Vec<BlockMapping> {
        vec![
            BlockMapping {
                block_id: 0,
                kind: BlockKind::Paragraph,
                start_line: 0,
                end_line: 10,
            },
            BlockMapping {
                block_id: 1,
                kind: BlockKind::Blockquote,
                start_line: 2,
                end_line: 8,
            },
            BlockMapping {
                block_id: 2,
                kind: BlockKind::Paragraph,
                start_line: 3,
                end_line: 5,
            },
        ]
    }

    #[test]
    fn opens_preview_after_one_second_hover_with_windows_probe_inputs() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        assert!(
            preview
                .observe_probe_outputs(
                    0,
                    &explorer_frontmost_json(),
                    Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                    Some(&coordinate_json(320.0, 180.0)),
                )
                .expect("probe outputs should classify")
                .is_empty()
        );

        let events = preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::PreviewWindowRequested { request } => {
                assert_eq!(request.document.display_name, "notes.md");
                assert_eq!(request.requested_width_px, 560);
                assert_eq!(request.monitor_id.as_deref(), Some("primary"));
                assert!(request.interaction_hot);
            }
            other => panic!("unexpected event: {other:?}"),
        }
        assert!(preview.state().visibility.visible);
    }

    #[test]
    fn blocks_preview_opening_while_frontmost_surface_is_not_explorer() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        let events = preview
            .observe_probe_outputs(
                1_000,
                &non_explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        assert!(events.is_empty());
        assert!(!preview.state().visibility.visible);
    }

    #[test]
    fn keeps_same_hovered_markdown_from_reopening_while_stationary() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        preview
            .observe_probe_outputs(
                0,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");
        preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        let repeated = preview
            .observe_probe_outputs(
                4_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        assert!(repeated.is_empty());
        assert_eq!(
            preview
                .state()
                .current_document
                .as_ref()
                .map(|document| document.display_name.as_str()),
            Some("notes.md")
        );
    }

    #[test]
    fn replaces_preview_only_after_the_resolved_markdown_document_changes() {
        let fixture = TempFixture::new();
        let first = fixture.write_file("a.md", "# first");
        let second = fixture.write_file("b.md", "# second");
        let mut preview = WindowsPreviewLoop::new();

        preview
            .observe_probe_outputs(
                0,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&first, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");
        preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&first, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        assert!(
            preview
                .observe_probe_outputs(
                    1_500,
                    &explorer_frontmost_json(),
                    Some(&hovered_item_json(&second, "exact-item-under-pointer")),
                    Some(&coordinate_json(420.0, 220.0)),
                )
                .expect("probe outputs should classify")
                .is_empty()
        );

        let replacement = preview
            .observe_probe_outputs(
                2_500,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&second, "exact-item-under-pointer")),
                Some(&coordinate_json(420.0, 220.0)),
            )
            .expect("probe outputs should classify");

        assert_eq!(replacement.len(), 1);
        match &replacement[0] {
            AppEvent::PreviewWindowRequested { request } => {
                assert_eq!(request.document.display_name, "b.md");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn ordinary_pointer_motion_on_the_same_document_does_not_dismiss_preview() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        preview
            .observe_probe_outputs(
                0,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");
        preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        let unchanged = preview
            .observe_probe_outputs(
                4_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(500.0, 260.0)),
            )
            .expect("probe outputs should classify");

        assert!(unchanged.is_empty());
        assert!(preview.state().visibility.visible);
        assert_eq!(preview.state().last_close_reason, None);
    }

    #[test]
    fn explorer_loss_hides_an_open_preview_via_shared_core_app_switch_semantics() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        preview
            .observe_probe_outputs(
                0,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");
        preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json(320.0, 180.0)),
            )
            .expect("probe outputs should classify");

        let hidden = preview
            .observe_probe_outputs(1_500, &non_explorer_frontmost_json(), None, None)
            .expect("frontmost probe should classify");

        assert_eq!(
            hidden,
            vec![AppEvent::PreviewWindowHidden {
                reason: CloseReason::AppSwitch,
            }]
        );
        assert!(!preview.state().visibility.visible);
    }

    #[test]
    fn preview_opens_hot_so_windows_tab_toggle_needs_no_rehover() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        open_visible_preview(&mut preview, &path);

        assert!(preview.state().interaction_hot);
        let events = preview.dispatch_command(AppCommand::ToggleBackgroundMode, &[]);
        assert_eq!(
            events,
            vec![AppEvent::BackgroundModeChanged {
                background_mode: BackgroundMode::Black,
            }]
        );
        assert_eq!(preview.state().background_mode, BackgroundMode::Black);
        assert_eq!(
            preview
                .state()
                .visibility
                .last_request
                .as_ref()
                .map(|request| request.background_mode),
            Some(BackgroundMode::Black)
        );
    }

    #[test]
    fn scroll_and_paging_commands_match_macos_without_rehover_inside_preview() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        open_visible_preview(&mut preview, &path);

        assert_eq!(
            preview.dispatch_command(
                AppCommand::ScrollPreview {
                    raw_delta_y: -84.0,
                    precise: true,
                },
                &[],
            ),
            vec![AppEvent::ScrollApplied { delta_y: 84.0 }]
        );
        assert_eq!(
            preview.dispatch_command(
                AppCommand::ScrollPreview {
                    raw_delta_y: -8.4,
                    precise: false,
                },
                &[],
            ),
            vec![AppEvent::ScrollApplied { delta_y: 84.0 }]
        );

        for (input, direction) in [
            (PageInput::Space, PageDirection::Forward),
            (PageInput::PageDown, PageDirection::Forward),
            (PageInput::ShiftSpace, PageDirection::Backward),
            (PageInput::PageUp, PageDirection::Backward),
        ] {
            let events = preview.dispatch_command(AppCommand::PagePreview { input }, &[]);
            match events.as_slice() {
                [AppEvent::PageMotionRequested { motion }] => {
                    assert_eq!(motion.direction, direction);
                    assert_eq!(
                        motion.page_fraction,
                        MACOS_REFERENCE_BEHAVIOR.paging.page_fraction
                    );
                    assert_eq!(
                        motion.overshoot_factor,
                        MACOS_REFERENCE_BEHAVIOR.paging.overshoot_factor
                    );
                    assert_eq!(
                        motion.max_overshoot_px,
                        MACOS_REFERENCE_BEHAVIOR.paging.max_overshoot_px
                    );
                    assert_eq!(
                        motion.first_segment_ms,
                        MACOS_REFERENCE_BEHAVIOR.paging.first_segment_ms
                    );
                    assert_eq!(
                        motion.settle_segment_ms,
                        MACOS_REFERENCE_BEHAVIOR.paging.settle_segment_ms
                    );
                }
                other => panic!("unexpected paging events: {other:?}"),
            }
        }
    }

    #[test]
    fn outside_click_and_escape_close_match_macos_on_windows() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");

        let mut outside_click_preview = WindowsPreviewLoop::new();
        open_visible_preview(&mut outside_click_preview, &path);
        assert_eq!(
            outside_click_preview.dispatch_command(AppCommand::OutsideClick, &[]),
            vec![AppEvent::PreviewWindowHidden {
                reason: CloseReason::OutsideClick,
            }]
        );
        assert!(!outside_click_preview.state().visibility.visible);

        let mut escape_preview = WindowsPreviewLoop::new();
        open_visible_preview(&mut escape_preview, &path);
        assert_eq!(
            escape_preview.dispatch_command(AppCommand::Escape, &[]),
            vec![AppEvent::PreviewWindowHidden {
                reason: CloseReason::Escape,
            }]
        );
        assert!(!escape_preview.state().visibility.visible);
    }

    #[test]
    fn inline_edit_entry_uses_smallest_block_and_maps_original_source_on_windows() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", edit_markdown());
        let mut preview = WindowsPreviewLoop::new();
        let blocks = edit_block_mappings();

        open_visible_preview(&mut preview, &path);

        let editor = preview
            .request_edit_at_line(4, edit_markdown(), &blocks)
            .expect("inline editor should open");

        assert_eq!(editor.block.block_id, 2);
        assert_eq!(editor.block.start_line, 3);
        assert_eq!(editor.block.end_line, 5);
        assert_eq!(editor.original_source, "line 4\nline 5");
        assert_eq!(editor.editable_source, "line 4\nline 5");
        assert_eq!(editor.source_line_label, "Editing source lines 4-5");
        assert_eq!(
            preview.state().editing.phase,
            EditingPhase::Active
        );
    }

    #[test]
    fn save_and_failed_save_preserve_the_current_editor_source_on_windows() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", edit_markdown());
        let mut preview = WindowsPreviewLoop::new();
        let blocks = edit_block_mappings();

        open_visible_preview(&mut preview, &path);
        preview
            .request_edit_at_line(4, edit_markdown(), &blocks)
            .expect("inline editor should open");

        let (replacement_markdown, save_events) = preview
            .save_current_edit(edit_markdown(), "updated\r\nblock", &blocks)
            .expect("save request should be emitted");

        assert_eq!(
            replacement_markdown,
            "line 1\nline 2\nline 3\nupdated\nblock\nline 6\nline 7\nline 8\nline 9\nline 10"
        );
        match save_events.as_slice() {
            [AppEvent::MarkdownSaveRequested {
                document,
                replacement_markdown: emitted_markdown,
            }] => {
                assert_eq!(document.display_name, "notes.md");
                assert_eq!(emitted_markdown, &replacement_markdown);
            }
            other => panic!("unexpected save events: {other:?}"),
        }
        assert_eq!(preview.state().editing.phase, EditingPhase::Saving);
        assert_eq!(
            preview.state().editing.draft_source.as_deref(),
            Some("updated\nblock")
        );

        let failed = preview.complete_save(false, None, Some("disk full".to_string()));
        match failed.as_slice() {
            [AppEvent::EditSessionChanged { editing }] => {
                assert_eq!(editing.phase, EditingPhase::Active);
                assert_eq!(
                    editing.draft_markdown.as_deref(),
                    Some(replacement_markdown.as_str())
                );
                assert_eq!(editing.draft_source.as_deref(), Some("updated\nblock"));
            }
            other => panic!("unexpected failed-save events: {other:?}"),
        }

        let editor = preview
            .inline_editor(edit_markdown(), &blocks)
            .expect("inline editor should stay open after a failed save");
        assert_eq!(editor.original_source, "line 4\nline 5");
        assert_eq!(editor.editable_source, "updated\nblock");

        let canceled = preview.cancel_edit_session();
        match canceled.as_slice() {
            [AppEvent::EditSessionChanged { editing }] => {
                assert_eq!(editing.phase, EditingPhase::Inactive);
                assert_eq!(editing.draft_markdown, None);
                assert_eq!(editing.draft_source, None);
            }
            other => panic!("unexpected cancel events: {other:?}"),
        }
    }

    #[test]
    fn edit_mode_lock_blocks_replacement_and_close_until_cancel_or_successful_save() {
        let fixture = TempFixture::new();
        let current = fixture.write_file("notes.md", edit_markdown());
        let other = fixture.write_file("other.md", "# other");
        let mut preview = WindowsPreviewLoop::new();
        let blocks = edit_block_mappings();

        open_visible_preview(&mut preview, &current);
        preview
            .request_edit_at_line(4, edit_markdown(), &blocks)
            .expect("inline editor should open");

        assert!(
            preview.dispatch_command(AppCommand::OutsideClick, &[]).is_empty()
        );
        assert!(preview.dispatch_command(AppCommand::Escape, &[]).is_empty());
        assert!(
            preview
                .observe_probe_outputs(
                    4_000,
                    &non_explorer_frontmost_json(),
                    None,
                    None,
                )
                .expect("frontmost probe should classify")
                .is_empty()
        );
        assert!(
            preview
                .observe_probe_outputs(
                    4_000,
                    &explorer_frontmost_json(),
                    Some(&hovered_item_json(&other, "exact-item-under-pointer")),
                    Some(&coordinate_json(420.0, 220.0)),
                )
                .expect("probe outputs should classify")
                .is_empty()
        );

        let (persisted_markdown, _) = preview
            .save_current_edit(edit_markdown(), "updated block", &blocks)
            .expect("save request should be emitted");
        assert!(
            preview.dispatch_command(AppCommand::OutsideClick, &[]).is_empty()
        );
        assert!(preview.dispatch_command(AppCommand::Escape, &[]).is_empty());

        preview.complete_save(true, Some(persisted_markdown), None);
        assert_eq!(
            preview.dispatch_command(AppCommand::Escape, &[]),
            vec![AppEvent::PreviewWindowHidden {
                reason: CloseReason::Escape,
            }]
        );
    }

    #[test]
    fn width_tier_command_uses_the_same_windows_width_model_and_repositions_before_shrinking() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        preview
            .observe_probe_outputs(
                0,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json_for_visible_frame(
                    2_150.0, 600.0, 2_200.0, 1_200.0,
                )),
            )
            .expect("probe outputs should classify");
        preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json_for_visible_frame(
                    2_150.0, 600.0, 2_200.0, 1_200.0,
                )),
            )
            .expect("probe outputs should classify");

        let events = preview.dispatch_command(
            AppCommand::AdjustWidthTier {
                delta: 1,
                monitor: None,
            },
            &[],
        );

        assert_eq!(
            events[0],
            AppEvent::WidthTierChanged {
                selected_width_tier_index: 1,
                requested_width_px: 960,
            }
        );
        match &events[1] {
            AppEvent::PreviewWindowRequested { request } => {
                assert_eq!(request.selected_width_tier_index, 1);
                assert_eq!(request.requested_width_px, 960);
                assert_eq!(request.frame.width, 960.0);
                assert_eq!(request.frame.height, 720.0);
                assert!(request.frame.x < 2_150.0);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn width_tier_command_only_shrinks_when_the_requested_four_by_three_size_cannot_fit() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.md", "# hello");
        let mut preview = WindowsPreviewLoop::new();

        preview
            .observe_probe_outputs(
                0,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json_for_visible_frame(
                    500.0, 400.0, 1_000.0, 800.0,
                )),
            )
            .expect("probe outputs should classify");
        preview
            .observe_probe_outputs(
                1_000,
                &explorer_frontmost_json(),
                Some(&hovered_item_json(&path, "exact-item-under-pointer")),
                Some(&coordinate_json_for_visible_frame(
                    500.0, 400.0, 1_000.0, 800.0,
                )),
            )
            .expect("probe outputs should classify");

        let events = preview.dispatch_command(
            AppCommand::AdjustWidthTier {
                delta: 3,
                monitor: None,
            },
            &[],
        );

        assert_eq!(
            events[0],
            AppEvent::WidthTierChanged {
                selected_width_tier_index: 3,
                requested_width_px: 1_920,
            }
        );
        match &events[1] {
            AppEvent::PreviewWindowRequested { request } => {
                assert_eq!(request.selected_width_tier_index, 3);
                assert_eq!(request.requested_width_px, 1_920);
                assert_eq!(request.frame.width, 976.0);
                assert_eq!(request.frame.height, 732.0);
                assert!((request.frame.width / request.frame.height - (4.0 / 3.0)).abs() < 0.0001);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
