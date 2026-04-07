/// Validation status for this crate slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Implemented directly in this crate slice.
    ImplementedInSlice,
    /// Requires live Ubuntu validation after host probes are wired.
    NeedsUbuntuHostValidation,
    /// Blocked by lower Stage 2 layers outside this worker lane.
    BlockedByLowerLayers,
}

/// One parity-validation note for the Ubuntu adapter lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationNote {
    /// Blueprint checklist item or feature name.
    pub item: &'static str,
    /// Current status.
    pub status: ValidationStatus,
    /// Short explanation for the status.
    pub note: &'static str,
}

/// Returns the validation notes for this bounded worker slice.
pub fn crate_slice_validation_notes() -> Vec<ValidationNote> {
    vec![
        ValidationNote {
            item: "Restrict Linux support target to Ubuntu 24.04 plus GNOME Files / Nautilus only",
            status: ValidationStatus::ImplementedInSlice,
            note: "The crate rejects non-Ubuntu-24.04 or non-GNOME sessions at the adapter boundary.",
        },
        ValidationNote {
            item: "Create fastmd-platform-linux-nautilus as a buildable crate",
            status: ValidationStatus::ImplementedInSlice,
            note: "The crate now has a real Cargo manifest, module layout, and unit tests.",
        },
        ValidationNote {
            item: "Implement Wayland and X11 behavior handling without changing product semantics",
            status: ValidationStatus::ImplementedInSlice,
            note: "Wayland and X11 have separate backend plans with one shared semantic guardrail.",
        },
        ValidationNote {
            item: "Implement Ubuntu frontmost GNOME Files detection with the same gating semantics as macOS Finder",
            status: ValidationStatus::ImplementedInSlice,
            note: "The adapter now applies explicit Wayland/X11 frontmost API-stack metadata plus a stable Nautilus surface classifier before the gate opens.",
        },
        ValidationNote {
            item: "Identify the authoritative Ubuntu 24.04 GNOME host API stack for frontmost Nautilus detection",
            status: ValidationStatus::ImplementedInSlice,
            note: "Wayland now names AT-SPI focused-accessible + application-bus + GTK application-id inputs, while X11 names EWMH _NET_ACTIVE_WINDOW + application-bus + GTK application-id inputs.",
        },
        ValidationNote {
            item: "Resolve the active GNOME Files / Nautilus surface to a stable Nautilus identity instead of a generic active-window check",
            status: ValidationStatus::ImplementedInSlice,
            note: "The frontmost classifier now requires a stable surface id from the host snapshot and preserves it in the accepted Nautilus surface record.",
        },
        ValidationNote {
            item: "Reject non-Nautilus foreground windows with the same strict gating semantics as macOS Finder",
            status: ValidationStatus::ImplementedInSlice,
            note: "Frontmost classification now rejects non-Nautilus identifiers and missing stable surface ids before hover resolution can proceed.",
        },
        ValidationNote {
            item: "Validate frontmost Nautilus detection on a real Ubuntu 24.04 Wayland session",
            status: ValidationStatus::NeedsUbuntuHostValidation,
            note: "The classifier is implemented and unit-tested, but the live Wayland host probe still needs Ubuntu validation evidence.",
        },
        ValidationNote {
            item: "Validate frontmost Nautilus detection on a real Ubuntu 24.04 X11 session",
            status: ValidationStatus::NeedsUbuntuHostValidation,
            note: "The classifier is implemented and unit-tested, but the live X11 host probe still needs Ubuntu validation evidence.",
        },
        ValidationNote {
            item: "Implement Ubuntu hovered-item resolution so the actual hovered .md item is resolved rather than a nearby or first visible candidate",
            status: ValidationStatus::NeedsUbuntuHostValidation,
            note: "The acceptance rules are implemented and tested; live Nautilus probe wiring remains to be validated.",
        },
        ValidationNote {
            item: "Identify the authoritative Ubuntu 24.04 GNOME host API stack for hovered Nautilus item resolution",
            status: ValidationStatus::ImplementedInSlice,
            note: "Wayland and X11 now explicitly name AT-SPI Component.GetAccessibleAtPoint plus Accessible children, role, attributes, and text queries within Nautilus GTK list roles as the hover-resolution stack.",
        },
        ValidationNote {
            item: "Implement Ubuntu multi-monitor coordinate handling with the same placement semantics as macOS",
            status: ValidationStatus::NeedsUbuntuHostValidation,
            note: "Containing-monitor and nearest-monitor selection are implemented, and the shared Tauri shell now consumes Linux monitor work_area rectangles in desktop-space coordinates; real GNOME monitor snapshots still need Ubuntu validation.",
        },
        ValidationNote {
            item: "Implement the same hot interaction-surface behavior as macOS",
            status: ValidationStatus::ImplementedInSlice,
            note: "The shared Tauri shell reveals the preview with window focus, the shared frontend keeps the shell root focusable after re-renders, and Linux parity can now rely on one hot surface instead of pointer re-entry.",
        },
        ValidationNote {
            item: "Keep the preview keyboard-hot without forcing the user to re-hover inside the preview",
            status: ValidationStatus::ImplementedInSlice,
            note: "Linux desktop shells now combine Tauri reveal-focus behavior with shared-frontend shell focus retention so width, background, paging, and close keys stay active without an extra re-hover step.",
        },
        ValidationNote {
            item: "Implement the same mouse-wheel and touchpad scrolling behavior as macOS",
            status: ValidationStatus::ImplementedInSlice,
            note: "The shared frontend now normalizes wheel deltas into the same direct preview-scroll path used by the macOS reference instead of depending on browser-default scrolling behavior.",
        },
        ValidationNote {
            item: "Implement preview opening, rendering, editing, and close behavior parity",
            status: ValidationStatus::BlockedByLowerLayers,
            note: "Shared shell parity now covers width tiers, work-area-based 4:3 placement, hint-chip chrome, Tab toggle, paged scrolling, and Escape close; hover-driven opening, edit persistence, and host-driven close paths still depend on shared-core and live Nautilus wiring.",
        },
        ValidationNote {
            item: "Implement the same runtime diagnostics coverage as macOS where host APIs permit",
            status: ValidationStatus::BlockedByLowerLayers,
            note: "The shell now emits structured Ubuntu diagnostics, but full macOS-equivalent runtime coverage still depends on live Nautilus host probes and shared-core hover wiring.",
        },
        ValidationNote {
            item: "Emit Ubuntu-side diagnostics for frontmost gating, hovered-item resolution, monitor selection, preview placement, and edit lifecycle",
            status: ValidationStatus::ImplementedInSlice,
            note: "The Linux adapter now defines one diagnostics vocabulary consumed by the Tauri shell and shared UI, with live frontmost and hovered-item fields marked pending until host probes are wired.",
        },
    ]
}
