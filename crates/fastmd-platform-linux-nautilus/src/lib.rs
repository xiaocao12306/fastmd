#![forbid(unsafe_code)]

//! Ubuntu 24.04 GNOME Files / Nautilus adapter seams for FastMD Stage 2.
//!
//! This crate is intentionally scoped to the Stage 2 Linux lane defined in
//! `Docs/Stage2_Blueprint.md`. It encodes the parity target explicitly:
//! Ubuntu 24.04 + GNOME Files / Nautilus must reproduce the current macOS
//! behavior under `apps/macos` rather than invent Linux-specific product rules.

pub mod adapter;
pub mod backends;
pub mod diagnostics;
pub mod error;
pub mod filter;
pub mod frontmost;
pub mod geometry;
pub mod hover;
pub mod live_probes;
pub mod probes;
pub mod target;
pub mod validation;

pub use adapter::{FrontmostGate, NautilusPlatformAdapter, ResolvedHover};
pub use diagnostics::{
    display_server_label, frontmost_gate_pending_note, hovered_item_pending_note,
    DIAGNOSTIC_STATUS_EMITTED, DIAGNOSTIC_STATUS_PENDING_LIVE_PROBE, EDIT_LIFECYCLE_POLICY,
    EDIT_LIFECYCLE_RUNTIME_NOTE, MONITOR_SELECTION_POLICY, MONITOR_SELECTION_RUNTIME_NOTE,
    PREVIEW_PLACEMENT_POLICY, PREVIEW_PLACEMENT_RUNTIME_NOTE,
};
pub use error::AdapterError;
pub use filter::{
    AcceptedMarkdownPath, HoverCandidate, HoverCandidateRejection, HoverCandidateSource,
    LinuxMarkdownFilter,
};
pub use frontmost::{
    api_stack_for_display_server, resolve_frontmost_surface, FrontmostNautilusSurface,
    FrontmostSurfaceRejection, NautilusFrontmostApi, NautilusFrontmostApiStack,
    NautilusSurfaceIdentity, WAYLAND_FRONTMOST_API_STACK, X11_FRONTMOST_API_STACK,
};
pub use geometry::{Monitor, MonitorLayout, ScreenPoint, ScreenRect};
pub use hover::{
    build_hovered_item_snapshot, classify_hovered_item_snapshot,
    hovered_item_api_stack_for_display_server, HoverResolutionScope, HoveredEntityKind,
    HoveredItemObservation, HoveredItemProbeOutcome, HoveredItemResolutionRejection,
    HoveredItemSnapshot, NautilusHoveredItemApi, NautilusHoveredItemApiStack,
    WAYLAND_HOVERED_ITEM_API_STACK, X11_HOVERED_ITEM_API_STACK,
};
pub use live_probes::{classify_live_frontmost_gate, live_frontmost_gate, LiveFrontmostProbe};
pub use probes::{
    FrontmostAppProbe, FrontmostAppSnapshot, HoveredItemProbe, MonitorProbe, NautilusProbeSuite,
    SessionProbe,
};
pub use target::{
    supported_surface_label, DisplayServerKind, SessionContext, MACOS_REFERENCE_ROOT,
    TARGET_DESKTOP, TARGET_DISTRO_NAME, TARGET_DISTRO_VERSION_PREFIX, TARGET_FILE_MANAGER,
};
pub use validation::{crate_slice_validation_notes, ValidationNote, ValidationStatus};
