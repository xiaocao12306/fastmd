#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

use fastmd_contracts::{merged_preview_feature_coverage, MacOsPreviewFeature};
use fastmd_core::shared_core_preview_feature_coverage;
use fastmd_render::shared_render_preview_feature_coverage;

/// Stage 2 keeps macOS Finder as the behavioral reference implementation while
/// shared Rust/Tauri layers are introduced.
pub const STAGE2_REFERENCE_HOST: &str = "macOS Finder";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacOSAdapterState {
    ReferenceOnly,
    SharedCoreBridged,
}

impl Default for MacOSAdapterState {
    fn default() -> Self {
        Self::ReferenceOnly
    }
}

pub fn macos_reference_adapter_preview_feature_coverage() -> &'static [MacOsPreviewFeature] {
    &[
        MacOsPreviewFeature::ExactHoveredMarkdownResolution,
        MacOsPreviewFeature::AcceptedLocalMarkdownFilesOnly,
        MacOsPreviewFeature::MonitorSelectionAndCoordinateTranslation,
        MacOsPreviewFeature::RuntimeDiagnosticsCoverage,
    ]
}

pub fn macos_reference_preview_feature_coverage() -> Vec<MacOsPreviewFeature> {
    merged_preview_feature_coverage(&[
        shared_core_preview_feature_coverage(),
        shared_render_preview_feature_coverage(),
        macos_reference_adapter_preview_feature_coverage(),
    ])
}

pub const MACOS_REFERENCE_PRERENDER_EVIDENCE: &str = "The macOS reference app now warms both hover and selection-triggered Markdown documents before open, reuses the pre-rendered HTML snapshot when the debounce completes, and invalidates warmed entries when the source file changes so the displayed preview stays current without doing its full file read/render work on the open path.";

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{
        macos_reference_adapter_preview_feature_coverage, macos_reference_preview_feature_coverage,
        MacOSAdapterState, MACOS_REFERENCE_PRERENDER_EVIDENCE, STAGE2_REFERENCE_HOST,
    };
    use fastmd_contracts::{
        macos_preview_feature_list, preview_feature_coverage_matches_reference,
        preview_feature_gaps_against_reference, MacOsPreviewFeature,
    };
    use std::collections::BTreeSet;

    #[test]
    fn macos_reference_host_is_explicit() {
        assert_eq!(STAGE2_REFERENCE_HOST, "macOS Finder");
    }

    #[test]
    fn default_state_starts_as_reference_only() {
        assert_eq!(
            MacOSAdapterState::default(),
            MacOSAdapterState::ReferenceOnly
        );
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate dir should have a parent")
            .parent()
            .expect("workspace crates dir should have a parent")
            .to_path_buf()
    }

    fn macos_source(relative_path: &str) -> String {
        fs::read_to_string(repo_root().join(relative_path))
            .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
    }

    #[test]
    fn macos_reference_adapter_preview_feature_coverage_stays_explicit() {
        let features: BTreeSet<_> = macos_reference_adapter_preview_feature_coverage()
            .iter()
            .copied()
            .collect();

        assert_eq!(features.len(), 4);
        assert!(features.contains(&MacOsPreviewFeature::ExactHoveredMarkdownResolution));
        assert!(features.contains(&MacOsPreviewFeature::AcceptedLocalMarkdownFilesOnly));
        assert!(features.contains(&MacOsPreviewFeature::MonitorSelectionAndCoordinateTranslation));
        assert!(features.contains(&MacOsPreviewFeature::RuntimeDiagnosticsCoverage));
    }

    #[test]
    fn macos_reference_preview_feature_coverage_matches_the_reference_feature_list() {
        let actual: BTreeSet<_> = macos_reference_preview_feature_coverage()
            .into_iter()
            .collect();
        let expected: BTreeSet<_> = macos_preview_feature_list().iter().copied().collect();

        assert_eq!(actual, expected);
        assert!(preview_feature_coverage_matches_reference(&[
            fastmd_core::shared_core_preview_feature_coverage(),
            fastmd_render::shared_render_preview_feature_coverage(),
            macos_reference_adapter_preview_feature_coverage(),
        ]));
        assert!(preview_feature_gaps_against_reference(&[
            fastmd_core::shared_core_preview_feature_coverage(),
            fastmd_render::shared_render_preview_feature_coverage(),
            macos_reference_adapter_preview_feature_coverage(),
        ])
        .is_empty());
    }

    #[test]
    fn macos_reference_prerender_evidence_stays_backed_by_reference_source_hooks() {
        let coordinator = macos_source("apps/macos/Sources/FastMD/FinderHoverCoordinator.swift");
        let hover_monitor = macos_source("apps/macos/Sources/FastMD/HoverMonitorService.swift");
        let selection = macos_source("apps/macos/Sources/FastMD/FinderSelectionResolver.swift");
        let panel = macos_source("apps/macos/Sources/FastMD/PreviewPanelController.swift");

        assert!(MACOS_REFERENCE_PRERENDER_EVIDENCE.contains("warms both hover and selection-triggered Markdown documents before open"));
        assert!(hover_monitor.contains("var onHoverWarmup"));
        assert!(coordinator.contains("handleHoverWarmup"));
        assert!(coordinator.contains("pendingWarmedHoverItem"));
        assert!(coordinator.contains("prepareMarkdown(fileURL:"));
        assert!(selection.contains("var onSnapshotChanged"));
        assert!(panel.contains("func prepareMarkdown(fileURL: URL)"));
        assert!(panel.contains("WarmedPreviewLoader.load"));
        assert!(panel.contains("WarmedPreviewCache"));
        assert!(panel.contains("warmed=%@"));
    }
}
