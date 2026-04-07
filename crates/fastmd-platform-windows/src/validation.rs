/// Current validation status for a Windows parity item in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureStatus {
    ImplementedInThisCrate,
    ImplementedViaSharedContractsCoreRender,
    PendingAdapterWork,
    PendingSharedCore,
}

impl FeatureStatus {
    pub fn is_complete(self) -> bool {
        matches!(
            self,
            Self::ImplementedInThisCrate | Self::ImplementedViaSharedContractsCoreRender
        )
    }
}

/// One Windows parity requirement and the honest status of this crate against it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdapterValidationFeature {
    pub blueprint_item: &'static str,
    pub status: FeatureStatus,
    pub evidence: &'static str,
}

/// Crate-local validation manifest for the Windows lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterValidationManifest {
    pub target: &'static str,
    pub reference_surface: &'static str,
    pub features: &'static [AdapterValidationFeature],
}

pub static WINDOWS_VALIDATION_FEATURES: [AdapterValidationFeature; 26] = [
    AdapterValidationFeature {
        blueprint_item: "Restrict Windows support target to Windows 11 plus Explorer only",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "Encoded in README, VALIDATION.md, and WINDOWS_EXPLORER_STAGE2_TARGET.",
    },
    AdapterValidationFeature {
        blueprint_item: "Create `fastmd-platform-windows` as a buildable crate",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "Independent Cargo manifest plus crate-local cargo check coverage.",
    },
    AdapterValidationFeature {
        blueprint_item: "Identify the authoritative Windows host API stack for frontmost Explorer detection",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "The crate now fixes the frontmost stack to GetForegroundWindow + GetWindowThreadProcessId + QueryFullProcessImageNameW + GetClassNameW + IShellWindows + IWebBrowserApp::HWND.",
    },
    AdapterValidationFeature {
        blueprint_item: "Resolve the active Explorer surface to a stable Explorer identity instead of a generic foreground-window check",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "FrontmostWindowSnapshot now resolves a stable surface identity from the matched Explorer shell HWND plus owner process id before it produces a FrontSurface.",
    },
    AdapterValidationFeature {
        blueprint_item: "Reject non-Explorer foreground windows with the same strict gating semantics as macOS Finder",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "The Windows-only frontmost probe now executes the authoritative foreground-window/process/class lookup and ShellWindows HWND bridge, then rejects any snapshot that fails the strict Explorer classifier.",
    },
    AdapterValidationFeature {
        blueprint_item: "Identify the authoritative Windows host API stack for hovered Explorer item resolution",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "The hover lane now fixes the stack to UI Automation ElementFromPoint + ControlViewWalker + Current.Name + IShellWindows + IWebBrowserApp::HWND + Folder.ParseName + FolderItem.Path.",
    },
    AdapterValidationFeature {
        blueprint_item: "Reject non-Markdown files, directories, and unsupported items with the same semantics as macOS",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "The Explorer hover pipeline now routes exact-item and hovered-row probe results through the crate-local WindowsMarkdownFilter, which rejects relative paths, stale paths, directories, unsupported entities, and non-Markdown extensions before preview open.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement Windows frontmost Explorer detection with the same gating semantics as macOS Finder",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "ExplorerAdapter::probe_frontmost_surface now runs a live Windows-only PowerShell probe that captures the foreground HWND, owning process image, window class, and matching ShellWindows HWND before producing a FrontSurface.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement Windows hovered-item resolution so the actual hovered `.md` item is resolved",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "ExplorerAdapter::resolve_hovered_item now probes UI Automation at the pointer, allows only exact-item or hovered-row evidence, reconstructs an absolute path through the matched Explorer shell window, and rejects nearby / first-visible fallbacks.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement Windows multi-monitor coordinate handling with the same placement semantics as macOS",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "ExplorerAdapter::translate_coordinates now probes Screen.AllScreens plus Cursor.Position, converts Windows top-left desktop coordinates into the shared y-up desktop space, preserves Screen.WorkingArea as the macOS-visible-frame equivalent, and uses fastmd_core::select_monitor_for_anchor to prefer the containing monitor before falling back to the nearest visible frame.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement preview opening on 1-second hover with the same semantics as macOS",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "WindowsPreviewLoop now feeds frontmost Explorer gating, exact hovered-item resolution, and translated monitor context into fastmd_core::observe_hover, so the first PreviewWindowRequested event only appears after the same 1000 ms debounce the macOS reference requires.",
    },
    AdapterValidationFeature {
        blueprint_item: "Wire Windows host signals into the shared hover debounce and replacement lifecycle",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "WindowsPreviewLoop preserves the current frontmost Explorer surface even when the gate closes, then proves through probe-driven tests that non-Explorer foreground windows block preview open, stationary same-item hovers do not reopen, different Markdown documents replace after a fresh debounce, and same-document pointer motion does not dismiss the preview.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same four width tiers, requested-width binding, and 4:3 reposition-before-shrink placement policy as macOS",
        status: FeatureStatus::ImplementedInThisCrate,
        evidence: "WindowsPreviewLoop now dispatches shared AppCommand::AdjustWidthTier into fastmd_core, and probe-driven tests prove that 560/960/1440/1920 tier requests preserve 4:3 placement, reposition before shrinking on roomy work areas, and shrink only when the requested tier cannot fit the selected Windows work area.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same compact hint-chip behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "fastmd-render locks the single-chip copy to MACOS_REFERENCE_BEHAVIOR, and crate-owned tests validate that the shared frontend template keeps the same compact width / Tab / paging chip for Windows without diverging labels.",
    },
    AdapterValidationFeature {
        blueprint_item: "Keep the Windows preview chrome free of Windows-only helper text that would diverge from macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "The shared preview shell template keeps the macOS eyebrow and hint-chip copy and crate-owned validation asserts that the shell source does not introduce Windows-, Explorer-, or Finder-specific helper text into the preview chrome.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same hot interaction-surface behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "Shared contracts require the preview to become hot on open, and the Windows preview loop now proves that opened previews immediately accept hot-surface commands without any extra host-specific priming.",
    },
    AdapterValidationFeature {
        blueprint_item: "Keep the preview keyboard-hot without forcing the user to re-hover inside the preview",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "WindowsPreviewLoop tests now open a preview once, then immediately dispatch Tab and paging commands through the shared core without any follow-up re-hover or SetInteractionHot call.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same `Tab` background toggle behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "The Windows preview loop now validates that AppCommand::ToggleBackgroundMode flips the preview from white to black immediately after open and updates the tracked preview request exactly through the shared core contract.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same mouse-wheel and touchpad scrolling behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "WindowsPreviewLoop tests now cover both precise and non-precise scroll inputs and prove they normalize to the same 84 px shared-core scroll motion the macOS reference uses.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same `Space`, `Shift+Space`, `Page Up`, and `Page Down` paging behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "The Windows preview loop now dispatches all four paging inputs through shared AppCommand::PagePreview and validates the same forward/backward semantics the macOS reference contract requires.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same sticky eased paging motion as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "Windows paging tests now assert the macOS-matching shared-core page fraction, overshoot factor, overshoot cap, and two-stage easing durations for every supported paging key.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same close-on-outside-click behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "WindowsPreviewLoop now validates that AppCommand::OutsideClick hides an open preview with CloseReason::OutsideClick through the shared close policy.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same close-on-app-switch behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "WindowsPreviewLoop already proves that losing frontmost Explorer state drives the shared core to hide the preview with CloseReason::AppSwitch.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same close-on-Escape behavior as macOS",
        status: FeatureStatus::ImplementedViaSharedContractsCoreRender,
        evidence: "WindowsPreviewLoop now validates that AppCommand::Escape hides an open preview with CloseReason::Escape through the shared close policy.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same inline block editing entry rule, edit source mapping behavior, edit save and cancel behavior, and edit-mode lock behavior as macOS",
        status: FeatureStatus::PendingAdapterWork,
        evidence: "Shared contracts/core already define the edit lifecycle, but the Windows lane still needs crate-local end-to-end wiring and Windows-specific validation evidence for edit entry, block mapping, save/cancel flow, and lock semantics.",
    },
    AdapterValidationFeature {
        blueprint_item: "Implement the same runtime diagnostics coverage as macOS where host APIs permit",
        status: FeatureStatus::PendingAdapterWork,
        evidence: "Diagnostics seam exists but host-backed emission is not implemented.",
    },
];

pub fn windows_validation_manifest() -> AdapterValidationManifest {
    AdapterValidationManifest {
        target: "Windows 11 + Explorer only",
        reference_surface: "apps/macos",
        features: &WINDOWS_VALIDATION_FEATURES,
    }
}

#[cfg(test)]
mod tests {
    use super::{FeatureStatus, windows_validation_manifest};

    #[test]
    fn validation_manifest_stays_explicit_about_target_and_reference() {
        let manifest = windows_validation_manifest();

        assert_eq!(manifest.target, "Windows 11 + Explorer only");
        assert_eq!(manifest.reference_surface, "apps/macos");
    }

    #[test]
    fn validation_manifest_separates_direct_shared_and_pending_windows_features() {
        let manifest = windows_validation_manifest();

        let implemented_in_crate = manifest
            .features
            .iter()
            .filter(|feature| feature.status == FeatureStatus::ImplementedInThisCrate)
            .count();
        let implemented_via_shared = manifest
            .features
            .iter()
            .filter(|feature| {
                feature.status == FeatureStatus::ImplementedViaSharedContractsCoreRender
            })
            .count();
        let completed = manifest
            .features
            .iter()
            .filter(|feature| feature.status.is_complete())
            .count();

        assert_eq!(implemented_in_crate, 13);
        assert_eq!(implemented_via_shared, 11);
        assert_eq!(completed, 24);
        assert!(
            manifest
                .features
                .iter()
                .any(|feature| feature.status == FeatureStatus::PendingAdapterWork)
        );
        assert!(
            manifest
                .features
                .iter()
                .all(|feature| feature.status != FeatureStatus::PendingSharedCore)
        );
    }
}
