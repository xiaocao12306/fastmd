/// Stage 2 Windows target locked by this lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsExplorerStage2Target {
    pub operating_system: &'static str,
    pub file_manager: &'static str,
    pub parity_reference_surface: &'static str,
}

/// Preview color mode parity target taken from the current macOS app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    White,
    Black,
}

/// Geometry and timing values that the Windows adapter must eventually match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewGeometryReference {
    pub hover_trigger_ms: u16,
    pub width_tiers: [u16; 4],
    pub aspect_ratio: (u8, u8),
    pub edge_inset_px: u8,
    pub pointer_offset_px: u8,
}

/// Shared interaction rules observed in the current macOS implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InteractionReference {
    pub requires_frontmost_file_manager: bool,
    pub replaces_different_hovered_markdown: bool,
    pub suppresses_stationary_reopen: bool,
    pub keeps_hot_surface_while_visible: bool,
    pub supports_scroll_wheel_and_touchpad: bool,
    pub supports_space_and_page_keys: bool,
    pub supports_background_toggle: bool,
}

/// Editing behavior the Windows lane must eventually reproduce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditModeReference {
    pub enters_on_double_click_of_smallest_matching_block: bool,
    pub locks_preview_replacement_until_save_or_cancel: bool,
    pub locks_preview_dismissal_until_save_or_cancel: bool,
    pub save_writes_back_to_source: bool,
    pub cancel_preserves_source: bool,
}

/// Close behavior taken from the current macOS app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosePolicyReference {
    pub outside_click_closes_when_not_editing: bool,
    pub app_switch_closes_when_not_editing: bool,
    pub escape_closes_when_not_editing: bool,
    pub editing_blocks_non_forced_close: bool,
}

/// Consolidated macOS reference behavior for this adapter lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacOsReferenceBehavior {
    pub reference_surface: &'static str,
    pub frontmost_file_manager_bundle_id: &'static str,
    pub preview_geometry: PreviewGeometryReference,
    pub background_modes: [BackgroundMode; 2],
    pub interaction: InteractionReference,
    pub edit_mode: EditModeReference,
    pub close_policy: ClosePolicyReference,
}

pub static WINDOWS_EXPLORER_STAGE2_TARGET: WindowsExplorerStage2Target =
    WindowsExplorerStage2Target {
        operating_system: "Windows 11",
        file_manager: "Explorer",
        parity_reference_surface: "apps/macos",
    };

pub static MACOS_REFERENCE_BEHAVIOR: MacOsReferenceBehavior = MacOsReferenceBehavior {
    reference_surface: "apps/macos",
    frontmost_file_manager_bundle_id: "com.apple.finder",
    preview_geometry: PreviewGeometryReference {
        hover_trigger_ms: 1_000,
        width_tiers: [560, 960, 1_440, 1_920],
        aspect_ratio: (4, 3),
        edge_inset_px: 12,
        pointer_offset_px: 18,
    },
    background_modes: [BackgroundMode::White, BackgroundMode::Black],
    interaction: InteractionReference {
        requires_frontmost_file_manager: true,
        replaces_different_hovered_markdown: true,
        suppresses_stationary_reopen: true,
        keeps_hot_surface_while_visible: true,
        supports_scroll_wheel_and_touchpad: true,
        supports_space_and_page_keys: true,
        supports_background_toggle: true,
    },
    edit_mode: EditModeReference {
        enters_on_double_click_of_smallest_matching_block: true,
        locks_preview_replacement_until_save_or_cancel: true,
        locks_preview_dismissal_until_save_or_cancel: true,
        save_writes_back_to_source: true,
        cancel_preserves_source: true,
    },
    close_policy: ClosePolicyReference {
        outside_click_closes_when_not_editing: true,
        app_switch_closes_when_not_editing: true,
        escape_closes_when_not_editing: true,
        editing_blocks_non_forced_close: true,
    },
};

#[cfg(test)]
mod tests {
    use super::{BackgroundMode, MACOS_REFERENCE_BEHAVIOR, WINDOWS_EXPLORER_STAGE2_TARGET};

    #[test]
    fn target_is_explicitly_windows_11_plus_explorer_only() {
        assert_eq!(
            WINDOWS_EXPLORER_STAGE2_TARGET.operating_system,
            "Windows 11"
        );
        assert_eq!(WINDOWS_EXPLORER_STAGE2_TARGET.file_manager, "Explorer");
        assert_eq!(
            WINDOWS_EXPLORER_STAGE2_TARGET.parity_reference_surface,
            "apps/macos"
        );
    }

    #[test]
    fn macos_reference_keeps_current_hover_and_preview_geometry() {
        assert_eq!(
            MACOS_REFERENCE_BEHAVIOR.preview_geometry.hover_trigger_ms,
            1_000
        );
        assert_eq!(
            MACOS_REFERENCE_BEHAVIOR.preview_geometry.width_tiers,
            [560, 960, 1_440, 1_920]
        );
        assert_eq!(
            MACOS_REFERENCE_BEHAVIOR.preview_geometry.aspect_ratio,
            (4, 3)
        );
        assert_eq!(
            MACOS_REFERENCE_BEHAVIOR.background_modes[0],
            BackgroundMode::White
        );
        assert_eq!(
            MACOS_REFERENCE_BEHAVIOR.background_modes[1],
            BackgroundMode::Black
        );
    }
}
