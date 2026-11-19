// Wayland/GNOME frontend for popterm.
//
// GNOME Shell owns window management on Wayland, so this command asks the
// companion Shell extension to toggle the matching window over D-Bus.

use std::env;
use std::process::{exit, Command};
use std::thread;
use std::time::Duration;

const DEFAULT_APP_ID: &str = "org.wezfurlong.wezterm";
const BUS_NAME: &str = "org.popterm.Toggle";
const OBJECT_PATH: &str = "/org/popterm/Toggle";
const INTERFACE: &str = "org.popterm.Toggle";

fn usage(program: &str) {
    eprintln!("Usage: {program} [OPTIONS] [APP_ID] [COMMAND [ARG ...]]");
    eprintln!("       {program} --geometry [APP_ID]");
    eprintln!("Options:");
    eprintln!("  --position top-center  Position the window the first time it is shown");
    eprintln!("  --top-margin PIXELS    Offset below the monitor work area's top edge");
    eprintln!("  --x-offset PIXELS      Horizontal offset from center (positive is right)");
    eprintln!("  --geometry             Print current frame and work-area geometry");
}

fn parse_reply(reply: &str) -> Result<bool, String> {
    match reply.trim() {
        "(true,)" => Ok(true),
        "(false,)" => Ok(false),
        other => Err(format!("unexpected D-Bus reply: {other}")),
    }
}

fn dbus_output(method: &str, arguments: &[&str]) -> Result<String, String> {
    let qualified_method = format!("{INTERFACE}.{method}");
    let mut command = Command::new("gdbus");
    command.args([
        "call",
        "--session",
        "--dest",
        BUS_NAME,
        "--object-path",
        OBJECT_PATH,
        "--method",
        &qualified_method,
    ]);
    command.args(arguments);
    let output = command
        .output()
        .map_err(|e| format!("could not run gdbus: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if stderr.is_empty() {
            format!("gdbus exited with {}", output.status)
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn dbus_call(method: &str, arguments: &[&str]) -> Result<bool, String> {
    parse_reply(&dbus_output(method, arguments)?)
}

fn positioned_call(
    method: &str,
    fallback_method: &str,
    arguments: &[&str],
    app_id: &str,
) -> Result<bool, String> {
    match dbus_call(method, arguments) {
        Err(error) if error.contains("UnknownMethod") => dbus_call(fallback_method, &[app_id]),
        result => result,
    }
}

fn parse_geometry(reply: &str) -> Result<[i32; 8], String> {
    let fields: Vec<_> = reply
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect();
    if fields.first() != Some(&"true") || fields.len() != 9 {
        return Err(format!(
            "window not found or unexpected geometry reply: {reply}"
        ));
    }
    let values = fields[1..]
        .iter()
        .map(|field| field.parse::<i32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("invalid geometry reply '{reply}': {error}"))?;
    values
        .try_into()
        .map_err(|_| format!("unexpected geometry reply: {reply}"))
}

fn print_geometry(app_id: &str) -> Result<(), String> {
    let [x, y, width, height, work_x, work_y, work_width, work_height] =
        parse_geometry(&dbus_output("Geometry", &[app_id])?)?;
    println!("window:    x={x} y={y} width={width} height={height}");
    println!("work-area: x={work_x} y={work_y} width={work_width} height={work_height}");
    println!("top-margin: {}", y - work_y);
    let centered_x = work_x + (work_width - width) / 2;
    println!("center-offset: {}", x - centered_x);
    Ok(())
}

fn spawn(command: &[String]) -> Result<(), String> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| "no launch command supplied".to_owned())?;
    Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to launch {program}: {e}"))
}

fn extension_hint(error: &str) -> ! {
    eprintln!("popterm-wayland: GNOME Shell extension is unavailable: {error}");
    eprintln!("Install and enable it with: ./install-wayland.sh");
    exit(2)
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let program = args.remove(0);
    let mut position = None;
    let mut top_margin = 0_i32;
    let mut x_offset = 0_i32;
    let mut geometry = false;
    while let Some(option) = args.first().map(String::as_str) {
        match option {
            "-h" | "--help" => {
                usage(&program);
                return;
            }
            "--geometry" => {
                geometry = true;
                args.remove(0);
            }
            "--position" => {
                args.remove(0);
                let Some(value) = args.first() else {
                    eprintln!("popterm-wayland: --position requires a value");
                    exit(2);
                };
                if value != "top-center" {
                    eprintln!("popterm-wayland: unsupported position '{value}'");
                    exit(2);
                }
                position = Some(args.remove(0));
            }
            "--top-margin" => {
                args.remove(0);
                let Some(value) = args.first() else {
                    eprintln!("popterm-wayland: --top-margin requires a value");
                    exit(2);
                };
                top_margin = value.parse().unwrap_or_else(|_| {
                    eprintln!("popterm-wayland: invalid top margin '{value}'");
                    exit(2);
                });
                args.remove(0);
            }
            "--x-offset" => {
                args.remove(0);
                let Some(value) = args.first() else {
                    eprintln!("popterm-wayland: --x-offset requires a value");
                    exit(2);
                };
                x_offset = value.parse().unwrap_or_else(|_| {
                    eprintln!("popterm-wayland: invalid x offset '{value}'");
                    exit(2);
                });
                args.remove(0);
            }
            "--" => {
                args.remove(0);
                break;
            }
            value if value.starts_with('-') => {
                eprintln!("popterm-wayland: unknown option '{value}'");
                exit(2);
            }
            _ => break,
        }
    }

    let app_id = args.first().map(String::as_str).unwrap_or(DEFAULT_APP_ID);
    let launch_command = args.get(1..).unwrap_or_default();

    if geometry {
        if let Err(error) = print_geometry(app_id) {
            extension_hint(&error);
        }
        return;
    }

    let margin = top_margin.to_string();
    let horizontal_offset = x_offset.to_string();
    let (toggle_method, show_method, call_arguments): (&str, &str, Vec<&str>) =
        if let Some(position) = position.as_deref() {
            (
                "TogglePositioned",
                "ShowPositioned",
                vec![
                    app_id,
                    position,
                    margin.as_str(),
                    horizontal_offset.as_str(),
                ],
            )
        } else {
            ("Toggle", "Show", vec![app_id])
        };

    match positioned_call(toggle_method, "Toggle", &call_arguments, app_id) {
        Ok(true) => return,
        Ok(false) if launch_command.is_empty() => {
            eprintln!("popterm-wayland: no window matched '{app_id}'");
            exit(1);
        }
        Ok(false) => {}
        Err(error) => extension_hint(&error),
    }

    if let Err(error) = spawn(launch_command) {
        eprintln!("popterm-wayland: {error}");
        exit(1);
    }

    // A newly started application needs a moment to create its first window.
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(100));
        match positioned_call(show_method, "Show", &call_arguments, app_id) {
            Ok(true) => return,
            Ok(false) => continue,
            Err(error) => extension_hint(&error),
        }
    }

    eprintln!("popterm-wayland: launched the command, but no '{app_id}' window appeared");
    exit(1);
}

#[cfg(test)]
mod tests {
    use super::{parse_geometry, parse_reply};

    #[test]
    fn parses_gdbus_boolean_tuple() {
        assert_eq!(parse_reply("(true,)\n").unwrap(), true);
        assert_eq!(parse_reply("(false,)\n").unwrap(), false);
        assert!(parse_reply("nonsense").is_err());
    }

    #[test]
    fn parses_geometry_reply() {
        assert_eq!(
            parse_geometry("(true, 10, 20, 1200, 800, 0, 32, 3840, 2128)").unwrap(),
            [10, 20, 1200, 800, 0, 32, 3840, 2128]
        );
        assert!(parse_geometry("(false, 0, 0, 0, 0, 0, 0, 0, 0)").is_err());
    }
}
