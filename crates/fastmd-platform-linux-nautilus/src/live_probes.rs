use std::process::Command;

use serde::Deserialize;

use crate::adapter::FrontmostGate;
use crate::error::AdapterError;
use crate::frontmost::{api_stack_for_display_server, resolve_frontmost_surface};
use crate::probes::FrontmostAppSnapshot;
use crate::target::{DisplayServerKind, SessionContext};

const LINUX_FRONTMOST_PROBE: &str = "linux-frontmost-nautilus";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveFrontmostProbe {
    pub backend: String,
    pub session: SessionContext,
    pub snapshot: FrontmostAppSnapshot,
}

pub fn live_frontmost_gate() -> Result<LiveFrontmostProbe, AdapterError> {
    let session = current_session_context()?;
    let snapshot = probe_frontmost_app(&session)?;
    let backend = match session.display_server {
        DisplayServerKind::Wayland => "live-atspi-wayland".to_owned(),
        DisplayServerKind::X11 => "live-atspi+xprop-x11".to_owned(),
    };

    Ok(LiveFrontmostProbe {
        backend,
        session,
        snapshot,
    })
}

pub fn classify_live_frontmost_gate() -> Result<(LiveFrontmostProbe, FrontmostGate), AdapterError> {
    let probe = live_frontmost_gate()?;
    let api_stack = api_stack_for_display_server(probe.session.display_server);

    let gate = match resolve_frontmost_surface(probe.session.display_server, &probe.snapshot) {
        Ok(surface) => FrontmostGate {
            session: probe.session.clone(),
            frontmost_app: probe.snapshot.clone(),
            detected_surface: Some(surface),
            rejection: None,
            api_stack,
            is_open: true,
        },
        Err(rejection) => FrontmostGate {
            session: probe.session.clone(),
            frontmost_app: probe.snapshot.clone(),
            detected_surface: None,
            rejection: Some(rejection),
            api_stack,
            is_open: false,
        },
    };

    Ok((probe, gate))
}

fn current_session_context() -> Result<SessionContext, AdapterError> {
    let os_release =
        std::fs::read_to_string("/etc/os-release").map_err(|error| AdapterError::ProbeFailure {
            probe: LINUX_FRONTMOST_PROBE,
            detail: format!("failed to read /etc/os-release: {error}"),
        })?;
    let distro_name =
        parse_os_release_field(&os_release, "NAME").unwrap_or_else(|| "Linux".to_owned());
    let distro_version =
        parse_os_release_field(&os_release, "VERSION_ID").unwrap_or_else(|| "unknown".to_owned());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("DESKTOP_SESSION").ok())
        .unwrap_or_else(|| "unknown".to_owned());
    let display_server = match std::env::var("XDG_SESSION_TYPE").ok().as_deref() {
        Some("wayland") => DisplayServerKind::Wayland,
        Some("x11") => DisplayServerKind::X11,
        _ if std::env::var_os("WAYLAND_DISPLAY").is_some() => DisplayServerKind::Wayland,
        _ if std::env::var_os("DISPLAY").is_some() => DisplayServerKind::X11,
        _ => {
            return Err(AdapterError::ProbeFailure {
                probe: LINUX_FRONTMOST_PROBE,
                detail: "unable to resolve Linux display server from XDG_SESSION_TYPE, WAYLAND_DISPLAY, or DISPLAY".to_owned(),
            })
        }
    };

    Ok(SessionContext {
        distro_name,
        distro_version,
        desktop,
        display_server,
    })
}

fn probe_frontmost_app(session: &SessionContext) -> Result<FrontmostAppSnapshot, AdapterError> {
    let atspi = probe_atspi_frontmost(session.display_server)?;
    let mut snapshot = FrontmostAppSnapshot {
        app_id: atspi.application_id.clone(),
        desktop_entry: atspi
            .application_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("{value}.desktop")),
        window_class: None,
        executable: atspi.executable.clone(),
        window_title: atspi.window_title.clone(),
        process_id: atspi.process_id,
        stable_surface_id: atspi.stable_surface_id.clone(),
    };

    if session.display_server == DisplayServerKind::X11 {
        let x11 = probe_x11_window_metadata()?;
        if x11.process_id.is_some() {
            snapshot.process_id = x11.process_id;
        }
        if x11.window_title.is_some() {
            snapshot.window_title = x11.window_title;
        }
        if x11.window_class.is_some() {
            snapshot.window_class = x11.window_class;
        }
        if x11.application_id.is_some() {
            snapshot.app_id = x11.application_id.clone();
            snapshot.desktop_entry = x11.application_id.map(|value| format!("{value}.desktop"));
        }
        if x11.stable_surface_id.is_some() {
            snapshot.stable_surface_id = x11.stable_surface_id;
        }
    }

    if snapshot.executable.is_none() {
        snapshot.executable = atspi.executable;
    }

    Ok(snapshot)
}

fn probe_atspi_frontmost(
    display_server: DisplayServerKind,
) -> Result<AtspiFrontmostProbeOutput, AdapterError> {
    run_command(
        LINUX_FRONTMOST_PROBE,
        "python3",
        &[
            "-c",
            AT_SPI_FRONTMOST_PROBE_SCRIPT,
            display_server_label(display_server),
        ],
    )
    .and_then(|stdout| parse_atspi_frontmost_probe_output(&stdout))
}

fn probe_x11_window_metadata() -> Result<X11WindowProbeOutput, AdapterError> {
    let active_window_stdout = run_command(
        LINUX_FRONTMOST_PROBE,
        "xprop",
        &["-root", "_NET_ACTIVE_WINDOW"],
    )?;
    let window_id = parse_x11_active_window_id(&active_window_stdout).ok_or_else(|| {
        AdapterError::ProbeFailure {
            probe: LINUX_FRONTMOST_PROBE,
            detail: format!(
                "xprop did not return an _NET_ACTIVE_WINDOW id: {}",
                active_window_stdout.trim()
            ),
        }
    })?;

    let properties_stdout = run_command(
        LINUX_FRONTMOST_PROBE,
        "xprop",
        &[
            "-id",
            &window_id,
            "WM_CLASS",
            "_GTK_APPLICATION_ID",
            "_NET_WM_PID",
            "_NET_WM_NAME",
        ],
    )?;

    parse_x11_window_probe_output(&properties_stdout, &window_id)
}

fn run_command(probe: &'static str, program: &str, args: &[&str]) -> Result<String, AdapterError> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new(program).args(args).output().map_err(|error| {
            AdapterError::ProbeFailure {
                probe,
                detail: format!("{program} failed to launch: {error}"),
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(AdapterError::ProbeFailure {
                probe,
                detail: format!(
                    "{program} exited with status {}: stderr={} stdout={}",
                    output.status,
                    stderr.trim(),
                    stdout.trim()
                ),
            });
        }

        String::from_utf8(output.stdout).map_err(|error| AdapterError::ProbeFailure {
            probe,
            detail: format!("{program} returned non-UTF8 stdout: {error}"),
        })
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (probe, program, args);
        Err(AdapterError::ProbeFailure {
            probe,
            detail: "live Linux probe is only available when this crate is built for Linux"
                .to_owned(),
        })
    }
}

fn parse_os_release_field(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let (field, raw_value) = line.split_once('=')?;
        if field.trim() != key {
            return None;
        }

        Some(raw_value.trim().trim_matches('"').to_owned())
    })
}

fn parse_atspi_frontmost_probe_output(
    raw: &str,
) -> Result<AtspiFrontmostProbeOutput, AdapterError> {
    let probe: AtspiFrontmostProbeOutput =
        serde_json::from_str(raw).map_err(|error| AdapterError::ProbeFailure {
            probe: LINUX_FRONTMOST_PROBE,
            detail: format!("failed to parse AT-SPI frontmost JSON: {error}"),
        })?;

    if let Some(error) = probe
        .error
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Err(AdapterError::ProbeFailure {
            probe: LINUX_FRONTMOST_PROBE,
            detail: error.to_owned(),
        });
    }

    Ok(probe)
}

fn parse_x11_active_window_id(raw: &str) -> Option<String> {
    raw.split_once('#')
        .map(|(_, value)| value.trim())
        .filter(|value| value.starts_with("0x"))
        .map(ToOwned::to_owned)
}

fn parse_x11_window_probe_output(
    raw: &str,
    window_id: &str,
) -> Result<X11WindowProbeOutput, AdapterError> {
    let mut output = X11WindowProbeOutput {
        stable_surface_id: Some(format!("x11:{window_id}")),
        window_class: None,
        application_id: None,
        process_id: None,
        window_title: None,
    };

    for line in raw.lines().map(str::trim) {
        if let Some((_, value)) = line.split_once("_GTK_APPLICATION_ID") {
            output.application_id = parse_quoted_last_value(value);
            continue;
        }

        if let Some((_, value)) = line.split_once("WM_CLASS") {
            output.window_class = parse_quoted_last_value(value);
            continue;
        }

        if let Some((_, value)) = line.split_once("_NET_WM_PID") {
            output.process_id = value
                .split('=')
                .nth(1)
                .and_then(|raw_pid| raw_pid.trim().parse::<u32>().ok());
            continue;
        }

        if let Some((_, value)) = line.split_once("_NET_WM_NAME") {
            output.window_title = parse_quoted_last_value(value);
        }
    }

    Ok(output)
}

fn parse_quoted_last_value(raw: &str) -> Option<String> {
    raw.split('"')
        .filter(|segment| !segment.trim().is_empty() && !segment.contains('='))
        .last()
        .map(|value| value.trim().to_owned())
}

fn display_server_label(display_server: DisplayServerKind) -> &'static str {
    match display_server {
        DisplayServerKind::Wayland => "wayland",
        DisplayServerKind::X11 => "x11",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AtspiFrontmostProbeOutput {
    #[serde(default)]
    application_id: Option<String>,
    #[serde(default)]
    executable: Option<String>,
    #[serde(default)]
    process_id: Option<u32>,
    #[serde(default)]
    stable_surface_id: Option<String>,
    #[serde(default)]
    window_title: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct X11WindowProbeOutput {
    stable_surface_id: Option<String>,
    window_class: Option<String>,
    application_id: Option<String>,
    process_id: Option<u32>,
    window_title: Option<String>,
}

const AT_SPI_FRONTMOST_PROBE_SCRIPT: &str = r#"
import json
import os
import sys

display_server = sys.argv[1] if len(sys.argv) > 1 else "unknown"

try:
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi
except Exception as error:
    print(json.dumps({"error": f"failed to import gi.repository.Atspi: {error}"}))
    sys.exit(0)

def safe_call(obj, name, *args):
    if obj is None:
        return None
    method = getattr(obj, name, None)
    if not callable(method):
        return None
    try:
        return method(*args)
    except Exception:
        return None

def state_contains(accessible, state):
    state_set = safe_call(accessible, "get_state_set")
    if state_set is None:
        return False
    try:
        return bool(state_set.contains(state))
    except Exception:
        return False

def children(accessible):
    count = safe_call(accessible, "get_child_count") or 0
    try:
        count = int(count)
    except Exception:
        count = 0
    for index in range(count):
        child = safe_call(accessible, "get_child_at_index", index)
        if child is not None:
            yield child

def find_focus(accessible, depth=0):
    if accessible is None or depth > 10:
        return None
    if state_contains(accessible, Atspi.StateType.FOCUSED) or state_contains(accessible, Atspi.StateType.ACTIVE):
        return accessible
    for child in children(accessible):
        focused = find_focus(child, depth + 1)
        if focused is not None:
            return focused
    return None

def first_named_window(accessible):
    cursor = accessible
    for _ in range(16):
        role_name = (safe_call(cursor, "get_role_name") or "").lower()
        name = safe_call(cursor, "get_name")
        if name and any(token in role_name for token in ("window", "frame", "dialog")):
            return name
        parent = safe_call(cursor, "get_parent")
        if parent is None or parent == cursor:
            break
        cursor = parent
    return safe_call(accessible, "get_name")

def executable_name(process_id):
    if process_id is None:
        return None
    try:
        return os.path.basename(os.readlink(f"/proc/{process_id}/exe"))
    except Exception:
        return None

Atspi.init()
desktop = Atspi.get_desktop(0)
focused = find_focus(desktop)

if focused is None:
    print(json.dumps({"error": f"no focused accessible found for {display_server}"}))
    sys.exit(0)

application = safe_call(focused, "get_application") or focused
process_id = safe_call(application, "get_process_id")
application_id = safe_call(application, "get_id")
window_title = first_named_window(focused) or safe_call(application, "get_name")
focused_name = safe_call(focused, "get_name")
stable_surface_id = safe_call(focused, "get_accessible_id")
if not stable_surface_id:
    stable_surface_id = f"atspi:{display_server}:pid={process_id or 'unknown'}:name={window_title or focused_name or 'unknown'}"

print(json.dumps({
    "application_id": application_id,
    "executable": executable_name(process_id),
    "process_id": process_id,
    "stable_surface_id": stable_surface_id,
    "window_title": window_title,
}))
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_field_parser_reads_quoted_values() {
        let raw = "NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\n";

        assert_eq!(
            parse_os_release_field(raw, "NAME"),
            Some("Ubuntu".to_owned())
        );
        assert_eq!(
            parse_os_release_field(raw, "VERSION_ID"),
            Some("24.04".to_owned())
        );
    }

    #[test]
    fn atspi_frontmost_probe_output_parses_live_fields() {
        let raw = r#"{
  "application_id": "org.gnome.Nautilus",
  "executable": "nautilus",
  "process_id": 4201,
  "stable_surface_id": "atspi:wayland:pid=4201:name=Docs",
  "window_title": "Docs"
}"#;

        let parsed = parse_atspi_frontmost_probe_output(raw).unwrap();

        assert_eq!(parsed.application_id, Some("org.gnome.Nautilus".to_owned()));
        assert_eq!(parsed.executable, Some("nautilus".to_owned()));
        assert_eq!(parsed.process_id, Some(4201));
    }

    #[test]
    fn x11_active_window_parser_extracts_hex_window_id() {
        let raw = "_NET_ACTIVE_WINDOW(WINDOW): window id # 0x4200011\n";

        assert_eq!(
            parse_x11_active_window_id(raw),
            Some("0x4200011".to_owned())
        );
    }

    #[test]
    fn x11_property_parser_extracts_class_app_id_pid_and_title() {
        let raw = r#"
WM_CLASS(STRING) = "org.gnome.Nautilus", "org.gnome.Nautilus"
_GTK_APPLICATION_ID(STRING) = "org.gnome.Nautilus"
_NET_WM_PID(CARDINAL) = 4202
_NET_WM_NAME(UTF8_STRING) = "Projects"
"#;

        let parsed = parse_x11_window_probe_output(raw, "0x4200011").unwrap();

        assert_eq!(parsed.window_class, Some("org.gnome.Nautilus".to_owned()));
        assert_eq!(parsed.application_id, Some("org.gnome.Nautilus".to_owned()));
        assert_eq!(parsed.process_id, Some(4202));
        assert_eq!(parsed.window_title, Some("Projects".to_owned()));
        assert_eq!(parsed.stable_surface_id, Some("x11:0x4200011".to_owned()));
    }

    #[test]
    fn quoted_value_parser_prefers_the_last_string_value() {
        assert_eq!(
            parse_quoted_last_value(r#"(STRING) = "Files", "org.gnome.Nautilus""#),
            Some("org.gnome.Nautilus".to_owned())
        );
    }
}
