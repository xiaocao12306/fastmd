#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Windows 11 + Explorer adapter seams for FastMD Stage 2.
//!
//! This crate deliberately tracks parity against the current macOS app under
//! `apps/macos`. It only claims what is implemented here today and keeps the
//! remaining Windows work behind explicit adapter seams.

pub mod adapter;
pub mod filter;
pub mod frontmost;
pub mod parity;
pub mod validation;

pub use adapter::{AdapterError, ExplorerAdapter, FrontmostSurfaceProbe, HostApi, HostCallState};
pub use filter::{
    AcceptedMarkdownPath, HoverCandidate, HoverCandidateRejection, HoverCandidateSource,
    WindowsMarkdownFilter,
};
pub use frontmost::{
    resolve_frontmost_surface, FrontmostSurfaceRejection, FrontmostWindowSnapshot,
    WindowsFrontmostApi, WindowsFrontmostApiStack, EXPLORER_WINDOW_CLASSES,
    WINDOWS_FRONTMOST_API_STACK,
};
pub use parity::{
    BackgroundMode, BackgroundToggleKey, BackgroundToggleReference, ClosePolicyReference,
    CoordinateSpaceReference, EditEntryReference, EditModeReference, FrontmostFileManagerReference,
    HintChipReference, HoverResolutionReference, InteractionReference, MacOsReferenceBehavior,
    MathDelimiterReference, MultiMonitorReference, PagingReference, PlacementBoundsReference,
    PreviewGeometryReference, RenderingChromeReference, RenderingLayoutReference,
    RenderingReference, RenderingRuntimeReference, RenderingThemeReference,
    RenderingTypographyReference, WindowsExplorerStage2Target, MACOS_REFERENCE_BEHAVIOR,
    WINDOWS_EXPLORER_FRONTMOST_REFERENCE, WINDOWS_EXPLORER_STAGE2_TARGET,
};
pub use validation::{
    AdapterValidationFeature, AdapterValidationManifest, FeatureStatus, WINDOWS_VALIDATION_FEATURES,
};
