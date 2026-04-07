use std::sync::Mutex;

use fastmd_platform_linux_nautilus::{
    api_stack_for_display_server, hovered_item_api_stack_for_display_server, DisplayServerKind,
    Monitor as PlatformMonitor, MonitorLayout as PlatformMonitorLayout,
    ScreenPoint as PlatformScreenPoint, ScreenRect as PlatformScreenRect,
};
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, Monitor as TauriMonitor, PhysicalPosition, PhysicalRect,
    PhysicalSize, Position, Size, State, WebviewWindow, WindowEvent,
};
use tauri_plugin_global_shortcut::Builder as GlobalShortcutBuilder;

const PREVIEW_WINDOW_LABEL: &str = "preview";
const SHELL_STATE_EVENT: &str = "fastmd://shell-state";
const HOST_CAPABILITIES_EVENT: &str = "fastmd://host-capabilities";
const CLOSE_REQUESTED_EVENT: &str = "fastmd://close-requested";
const WIDTH_TIERS: [u32; 4] = [560, 960, 1440, 1920];
const PREVIEW_ASPECT_RATIO: f64 = 4.0 / 3.0;
const PREVIEW_EDGE_INSET: f64 = 12.0;
const PREVIEW_POINTER_OFFSET: f64 = 18.0;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum BackgroundMode {
    White,
    Black,
}

impl BackgroundMode {
    fn toggled(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
enum RuntimeMode {
    Desktop,
    Fallback,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShellStatePayload {
    document_title: String,
    markdown: String,
    content_base_url: Option<String>,
    width_tiers: [u32; 4],
    selected_width_tier_index: usize,
    background_mode: BackgroundMode,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostCapabilitiesPayload {
    platform_id: &'static str,
    runtime_mode: RuntimeMode,
    accessibility_permission: &'static str,
    frontmost_file_manager: &'static str,
    preview_window_positioning: bool,
    global_shortcut_registered: bool,
    close_on_blur_enabled: bool,
    can_persist_preview_edits: bool,
    linux_probe_plans: Option<LinuxProbePlansPayload>,
    linux_preview_placement: Option<LinuxPreviewPlacementPayload>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    shell_state: ShellStatePayload,
    host_capabilities: HostCapabilitiesPayload,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenPoint {
    x: f64,
    y: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewGeometryPayload {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseRequestPayload {
    reason: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LinuxProbePlansPayload {
    wayland_frontmost_api_stack: String,
    x11_frontmost_api_stack: String,
    wayland_hovered_item_api_stack: String,
    x11_hovered_item_api_stack: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LinuxPreviewPlacementPayload {
    monitor_work_area_source: &'static str,
    monitor_selection_policy: &'static str,
    coordinate_space: &'static str,
    aspect_ratio: &'static str,
    edge_inset_px: u32,
    pointer_offset_px: u32,
}

struct ShellBridgeState {
    shell_state: Mutex<ShellStatePayload>,
    host_capabilities: Mutex<HostCapabilitiesPayload>,
    is_editing: Mutex<bool>,
    last_anchor: Mutex<Option<ScreenPoint>>,
}

impl ShellBridgeState {
    fn new() -> Self {
        let markdown = include_str!("../../../../README.md").to_owned();
        Self {
            shell_state: Mutex::new(ShellStatePayload {
                document_title: "README.md".to_owned(),
                markdown,
                content_base_url: None,
                width_tiers: WIDTH_TIERS,
                selected_width_tier_index: 0,
                background_mode: BackgroundMode::White,
            }),
            host_capabilities: Mutex::new(HostCapabilitiesPayload {
                platform_id: detected_platform_id(),
                runtime_mode: RuntimeMode::Desktop,
                accessibility_permission: "unknown",
                frontmost_file_manager: "unknown",
                preview_window_positioning: true,
                global_shortcut_registered: true,
                close_on_blur_enabled: true,
                can_persist_preview_edits: false,
                linux_probe_plans: linux_probe_plans_payload(),
                linux_preview_placement: linux_preview_placement_payload(),
            }),
            is_editing: Mutex::new(false),
            last_anchor: Mutex::new(None),
        }
    }

    fn bootstrap_payload(&self) -> BootstrapPayload {
        BootstrapPayload {
            shell_state: self.snapshot_shell_state(),
            host_capabilities: self.snapshot_host_capabilities(),
        }
    }

    fn snapshot_shell_state(&self) -> ShellStatePayload {
        self.shell_state.lock().unwrap().clone()
    }

    fn snapshot_host_capabilities(&self) -> HostCapabilitiesPayload {
        self.host_capabilities.lock().unwrap().clone()
    }
}

fn detected_platform_id() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "ubuntu"
    } else {
        "shell"
    }
}

fn linux_probe_plans_payload() -> Option<LinuxProbePlansPayload> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    Some(LinuxProbePlansPayload {
        wayland_frontmost_api_stack: api_stack_for_display_server(DisplayServerKind::Wayland)
            .diagnostic_summary(),
        x11_frontmost_api_stack: api_stack_for_display_server(DisplayServerKind::X11)
            .diagnostic_summary(),
        wayland_hovered_item_api_stack: hovered_item_api_stack_for_display_server(
            DisplayServerKind::Wayland,
        )
        .diagnostic_summary(),
        x11_hovered_item_api_stack: hovered_item_api_stack_for_display_server(
            DisplayServerKind::X11,
        )
        .diagnostic_summary(),
    })
}

fn linux_preview_placement_payload() -> Option<LinuxPreviewPlacementPayload> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    Some(LinuxPreviewPlacementPayload {
        monitor_work_area_source: "tauri-runtime-wry linux monitor.work_area via GDK/GNOME workarea",
        monitor_selection_policy: "containing-work-area-then-nearest",
        coordinate_space: "desktop-space physical pixels",
        aspect_ratio: "4:3",
        edge_inset_px: PREVIEW_EDGE_INSET as u32,
        pointer_offset_px: PREVIEW_POINTER_OFFSET as u32,
    })
}

fn emit_shell_state(app: &AppHandle, state: &ShellBridgeState) -> Result<(), String> {
    app.emit(SHELL_STATE_EVENT, state.snapshot_shell_state())
        .map_err(|error| error.to_string())
}

fn emit_host_capabilities(app: &AppHandle, state: &ShellBridgeState) -> Result<(), String> {
    app.emit(HOST_CAPABILITIES_EVENT, state.snapshot_host_capabilities())
        .map_err(|error| error.to_string())
}

fn current_anchor_or_monitor_center(
    window: &WebviewWindow,
    state: &ShellBridgeState,
) -> Result<ScreenPoint, String> {
    if let Some(anchor) = *state.last_anchor.lock().unwrap() {
        return Ok(anchor);
    }

    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor is available for preview geometry.".to_owned())?;
    let work_area = *monitor.work_area();

    Ok(ScreenPoint {
        x: f64::from(work_area.position.x) + f64::from(work_area.size.width) / 2.0,
        y: f64::from(work_area.position.y) + f64::from(work_area.size.height) / 2.0,
    })
}

fn platform_screen_point(point: ScreenPoint) -> PlatformScreenPoint {
    PlatformScreenPoint {
        x: point.x,
        y: point.y,
    }
}

fn platform_screen_rect_from_tauri_rect(rect: PhysicalRect<i32, u32>) -> PlatformScreenRect {
    PlatformScreenRect {
        x: f64::from(rect.position.x),
        y: f64::from(rect.position.y),
        width: f64::from(rect.size.width),
        height: f64::from(rect.size.height),
    }
}

fn monitor_identity_key(monitor: &TauriMonitor) -> (i32, i32, u32, u32) {
    let position = monitor.position();
    let size = monitor.size();
    (position.x, position.y, size.width, size.height)
}

fn platform_monitor_layout_from_tauri(
    monitors: &[TauriMonitor],
    primary_identity: Option<(i32, i32, u32, u32)>,
) -> PlatformMonitorLayout {
    PlatformMonitorLayout {
        monitors: monitors
            .iter()
            .enumerate()
            .map(|(index, monitor)| {
                let full_frame = PlatformScreenRect {
                    x: f64::from(monitor.position().x),
                    y: f64::from(monitor.position().y),
                    width: f64::from(monitor.size().width),
                    height: f64::from(monitor.size().height),
                };
                PlatformMonitor {
                    id: format!("monitor-{index}"),
                    frame: full_frame,
                    work_area: platform_screen_rect_from_tauri_rect(*monitor.work_area()),
                    primary: primary_identity
                        .is_some_and(|identity| identity == monitor_identity_key(monitor)),
                }
            })
            .collect(),
    }
}

fn selected_work_area_for_anchor(
    layout: &PlatformMonitorLayout,
    anchor: ScreenPoint,
) -> Option<PlatformScreenRect> {
    layout
        .monitor_for_point(platform_screen_point(anchor))
        .map(|monitor| monitor.work_area)
}

fn preview_work_area_for_anchor(
    window: &WebviewWindow,
    anchor: ScreenPoint,
) -> Result<PlatformScreenRect, String> {
    let monitors = window
        .available_monitors()
        .map_err(|error| error.to_string())?;
    if monitors.is_empty() {
        return Err("No monitor is available for preview geometry.".to_owned());
    }

    let primary_identity = window
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(monitor_identity_key);
    let layout = platform_monitor_layout_from_tauri(&monitors, primary_identity);

    selected_work_area_for_anchor(&layout, anchor)
        .ok_or_else(|| "No monitor work area is available for preview geometry.".to_owned())
}

fn compute_preview_geometry(
    anchor: ScreenPoint,
    work_area: PlatformScreenRect,
    requested_width: u32,
) -> PreviewGeometryPayload {
    let min_x = work_area.x + PREVIEW_EDGE_INSET;
    let min_y = work_area.y + PREVIEW_EDGE_INSET;
    let available_width = (work_area.width - PREVIEW_EDGE_INSET * 2.0).max(320.0);
    let available_height = (work_area.height - PREVIEW_EDGE_INSET * 2.0).max(240.0);
    let max_fit_width = available_width.min(available_height * PREVIEW_ASPECT_RATIO);
    let max_fit_height = max_fit_width / PREVIEW_ASPECT_RATIO;

    let requested_width = requested_width as f64;
    let requested_height = requested_width / PREVIEW_ASPECT_RATIO;
    let width = requested_width.min(max_fit_width);
    let height = requested_height.min(max_fit_height);

    let max_x = work_area.x + work_area.width - width - PREVIEW_EDGE_INSET;
    let max_y = work_area.y + work_area.height - height - PREVIEW_EDGE_INSET;

    let mut origin_x = anchor.x + PREVIEW_POINTER_OFFSET;
    let mut origin_y = anchor.y - height - PREVIEW_POINTER_OFFSET;

    if origin_x > max_x {
        origin_x = anchor.x - width - PREVIEW_POINTER_OFFSET;
    }
    if origin_x < min_x {
        origin_x = min_x;
    }
    if origin_x > max_x {
        origin_x = max_x;
    }

    if origin_y < min_y {
        origin_y = anchor.y + PREVIEW_POINTER_OFFSET;
    }
    if origin_y > max_y {
        origin_y = max_y;
    }
    if origin_y < min_y {
        origin_y = min_y;
    }

    PreviewGeometryPayload {
        x: origin_x.round() as i32,
        y: origin_y.round() as i32,
        width: width.round() as u32,
        height: height.round() as u32,
    }
}

fn apply_preview_geometry_internal(
    window: &WebviewWindow,
    state: &ShellBridgeState,
    anchor: Option<ScreenPoint>,
) -> Result<PreviewGeometryPayload, String> {
    let effective_anchor = match anchor {
        Some(anchor) => {
            *state.last_anchor.lock().unwrap() = Some(anchor);
            anchor
        }
        None => current_anchor_or_monitor_center(window, state)?,
    };
    let work_area = preview_work_area_for_anchor(window, effective_anchor)?;

    let requested_width = {
        let shell_state = state.shell_state.lock().unwrap();
        shell_state.width_tiers[shell_state.selected_width_tier_index]
    };

    let geometry = compute_preview_geometry(effective_anchor, work_area, requested_width);

    window
        .set_size(Size::Physical(PhysicalSize::new(
            geometry.width,
            geometry.height,
        )))
        .map_err(|error| error.to_string())?;
    window
        .set_position(Position::Physical(PhysicalPosition::new(
            geometry.x, geometry.y,
        )))
        .map_err(|error| error.to_string())?;

    Ok(geometry)
}

fn reveal_preview_window(window: &WebviewWindow, state: &ShellBridgeState) -> Result<(), String> {
    let _ = apply_preview_geometry_internal(window, state, None)?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn bootstrap_shell(state: State<'_, ShellBridgeState>) -> BootstrapPayload {
    state.bootstrap_payload()
}

#[tauri::command]
fn set_editing_state(
    app: AppHandle,
    state: State<'_, ShellBridgeState>,
    editing: bool,
) -> Result<(), String> {
    *state.is_editing.lock().unwrap() = editing;
    {
        let mut host_capabilities = state.host_capabilities.lock().unwrap();
        host_capabilities.close_on_blur_enabled = !editing;
    }
    emit_host_capabilities(&app, &state)
}

#[tauri::command]
fn adjust_width_tier(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ShellBridgeState>,
    delta: i32,
) -> Result<ShellStatePayload, String> {
    {
        let mut shell_state = state.shell_state.lock().unwrap();
        let current = shell_state.selected_width_tier_index as i32;
        let next = (current + delta).clamp(0, shell_state.width_tiers.len() as i32 - 1);
        shell_state.selected_width_tier_index = next as usize;
    }
    let _ = apply_preview_geometry_internal(&window, &state, None)?;
    emit_shell_state(&app, &state)?;
    Ok(state.snapshot_shell_state())
}

#[tauri::command]
fn toggle_background_mode(
    app: AppHandle,
    state: State<'_, ShellBridgeState>,
) -> Result<ShellStatePayload, String> {
    {
        let mut shell_state = state.shell_state.lock().unwrap();
        shell_state.background_mode = shell_state.background_mode.toggled();
    }
    emit_shell_state(&app, &state)?;
    Ok(state.snapshot_shell_state())
}

#[tauri::command]
fn replace_preview_markdown(
    app: AppHandle,
    state: State<'_, ShellBridgeState>,
    markdown: String,
    content_base_url: Option<String>,
) -> Result<ShellStatePayload, String> {
    {
        let mut shell_state = state.shell_state.lock().unwrap();
        shell_state.markdown = markdown;
        shell_state.content_base_url = content_base_url;
    }
    emit_shell_state(&app, &state)?;
    Ok(state.snapshot_shell_state())
}

#[tauri::command]
fn request_preview_close(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ShellBridgeState>,
    reason: String,
) -> Result<(), String> {
    if *state.is_editing.lock().unwrap() {
        return Ok(());
    }
    window.hide().map_err(|error| error.to_string())?;
    app.emit(
        CLOSE_REQUESTED_EVENT,
        CloseRequestPayload {
            reason: reason.clone(),
        },
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn apply_preview_geometry(
    window: WebviewWindow,
    state: State<'_, ShellBridgeState>,
    anchor: Option<ScreenPoint>,
) -> Result<PreviewGeometryPayload, String> {
    apply_preview_geometry_internal(&window, &state, anchor)
}

#[tauri::command]
fn reveal_preview(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ShellBridgeState>,
) -> Result<(), String> {
    reveal_preview_window(&window, &state)?;
    emit_shell_state(&app, &state)?;
    emit_host_capabilities(&app, &state)
}

fn main() {
    let shell_state = ShellBridgeState::new();
    let global_shortcut_plugin = GlobalShortcutBuilder::new()
        .with_shortcut("CmdOrCtrl+Shift+P")
        .expect("failed to register the FastMD preview re-open shortcut")
        .with_handler(|app, _, _| {
            if let Some(window) = app.get_webview_window(PREVIEW_WINDOW_LABEL) {
                if let Some(state) = app.try_state::<ShellBridgeState>() {
                    let _ = reveal_preview_window(&window, &state);
                    let _ = emit_shell_state(app, &state);
                    let _ = emit_host_capabilities(app, &state);
                }
            }
        })
        .build();

    tauri::Builder::default()
        .manage(shell_state)
        .plugin(global_shortcut_plugin)
        .invoke_handler(tauri::generate_handler![
            bootstrap_shell,
            set_editing_state,
            adjust_width_tier,
            toggle_background_mode,
            replace_preview_markdown,
            request_preview_close,
            apply_preview_geometry,
            reveal_preview,
        ])
        .on_window_event(|window, event| {
            if window.label() != PREVIEW_WINDOW_LABEL {
                return;
            }

            if let WindowEvent::Focused(false) = event {
                let Some(state) = window.app_handle().try_state::<ShellBridgeState>() else {
                    return;
                };
                if !state
                    .host_capabilities
                    .lock()
                    .unwrap()
                    .close_on_blur_enabled
                {
                    return;
                }
                let _ = window.hide();
                let _ = window.app_handle().emit(
                    CLOSE_REQUESTED_EVENT,
                    CloseRequestPayload {
                        reason: "focus-lost".to_owned(),
                    },
                );
            }
        })
        .setup(|app| {
            let window = app
                .get_webview_window(PREVIEW_WINDOW_LABEL)
                .ok_or_else(|| std::io::Error::other("The preview window is not configured."))?;
            let state = app.state::<ShellBridgeState>();
            reveal_preview_window(&window, &state).map_err(std::io::Error::other)?;
            emit_shell_state(app.handle(), &state).map_err(std::io::Error::other)?;
            emit_host_capabilities(app.handle(), &state).map_err(std::io::Error::other)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run the FastMD desktop Tauri shell");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_tiers_match_the_macos_reference_values() {
        assert_eq!(WIDTH_TIERS, [560, 960, 1440, 1920]);
    }

    #[test]
    fn tauri_shell_bootstraps_reference_width_tiers() {
        let shell_state = ShellBridgeState::new();

        assert_eq!(shell_state.snapshot_shell_state().width_tiers, WIDTH_TIERS);
    }

    #[test]
    fn tauri_shell_bootstraps_in_desktop_runtime_mode() {
        let shell_state = ShellBridgeState::new();

        assert_eq!(
            shell_state.snapshot_host_capabilities().runtime_mode,
            RuntimeMode::Desktop
        );
    }

    #[test]
    fn linux_probe_plans_are_only_advertised_on_linux_targets() {
        let shell_state = ShellBridgeState::new();

        assert_eq!(
            shell_state
                .snapshot_host_capabilities()
                .linux_probe_plans
                .is_some(),
            cfg!(target_os = "linux")
        );
    }

    #[test]
    fn linux_preview_placement_metadata_is_only_advertised_on_linux_targets() {
        let shell_state = ShellBridgeState::new();

        assert_eq!(
            shell_state
                .snapshot_host_capabilities()
                .linux_preview_placement
                .is_some(),
            cfg!(target_os = "linux")
        );
    }

    #[test]
    fn shell_monitor_selection_prefers_containing_work_area_then_nearest() {
        let layout = PlatformMonitorLayout {
            monitors: vec![
                PlatformMonitor {
                    id: "primary".to_owned(),
                    frame: PlatformScreenRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1920.0,
                        height: 1080.0,
                    },
                    work_area: PlatformScreenRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1920.0,
                        height: 1040.0,
                    },
                    primary: true,
                },
                PlatformMonitor {
                    id: "secondary".to_owned(),
                    frame: PlatformScreenRect {
                        x: 1920.0,
                        y: 0.0,
                        width: 2560.0,
                        height: 1440.0,
                    },
                    work_area: PlatformScreenRect {
                        x: 1920.0,
                        y: 0.0,
                        width: 2560.0,
                        height: 1400.0,
                    },
                    primary: false,
                },
            ],
        };

        let containing = selected_work_area_for_anchor(
            &layout,
            ScreenPoint { x: 2200.0, y: 300.0 },
        )
        .unwrap();
        assert_eq!(
            containing,
            PlatformScreenRect {
                x: 1920.0,
                y: 0.0,
                width: 2560.0,
                height: 1400.0,
            }
        );

        let nearest =
            selected_work_area_for_anchor(&layout, ScreenPoint { x: 5000.0, y: 5000.0 }).unwrap();
        assert_eq!(
            nearest,
            PlatformScreenRect {
                x: 1920.0,
                y: 0.0,
                width: 2560.0,
                height: 1400.0,
            }
        );
    }

    #[test]
    fn preview_geometry_repositions_before_shrinking_when_requested_tier_still_fits() {
        let geometry = compute_preview_geometry(
            ScreenPoint { x: 980.0, y: 500.0 },
            PlatformScreenRect {
                x: 0.0,
                y: 0.0,
                width: 1200.0,
                height: 900.0,
            },
            960,
        );

        assert_eq!(geometry.width, 960);
        assert_eq!(geometry.height, 720);
        assert_eq!(geometry.x, 2);
        assert_eq!(geometry.y, 168);
    }

    #[test]
    fn preview_geometry_shrinks_only_when_requested_tier_exceeds_work_area_capacity() {
        let geometry = compute_preview_geometry(
            ScreenPoint { x: 320.0, y: 240.0 },
            PlatformScreenRect {
                x: 0.0,
                y: 0.0,
                width: 700.0,
                height: 500.0,
            },
            960,
        );

        assert_eq!(geometry.width, 635);
        assert_eq!(geometry.height, 476);
        assert_eq!(geometry.x, 12);
        assert_eq!(geometry.y, 12);
    }
}
