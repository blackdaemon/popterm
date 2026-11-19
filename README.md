# Popterm

Toggle a drop-down style terminal (WezTerm by default) with a single keyboard
shortcut. If the window is visible, Popterm minimizes it. If it is hidden,
Popterm shows and focuses it. If no matching window exists, it can launch one.

Popterm ships in two editions:

| Edition | Binary | Session | Mechanism |
|---------|--------|---------|-----------|
| X11 | `popterm` | X11 / XWayland | Talks to the X server via `x11rb`, uses `xdotool` to minimize, map and activate |
| GNOME Wayland | `popterm-wayland` | GNOME Shell on Wayland | Calls a small GNOME Shell extension over D-Bus |

A legacy Go prototype (`popterm.go`) is also included. It is not part of the
Cargo build.

## Requirements

- Rust toolchain (`cargo`)
- X11 edition: `xdotool`
- Wayland edition: GNOME Shell and the `gnome-extensions` tool

## Install

### X11

```sh
./build.sh
```

This runs `cargo build --release && cargo install --path .`.

### GNOME Wayland

```sh
./install-wayland.sh
```

This installs the GNOME Shell extension, installs the `popterm-wayland` binary
and enables the extension. If GNOME has not seen the extension before, log out
and back in, then run:

```sh
gnome-extensions enable popterm-wayland-bridge@blackdaemon
```

See [WAYLAND.md](WAYLAND.md) for details.

## Usage

### X11

```sh
popterm [WM_CLASS [COMMAND...]]
```

- `WM_CLASS` is matched against the window's `WM_CLASS` instance or class.
  The default is `org.wezfurlong.wezterm`.
- `COMMAND...` is run when no matching window exists. A `~` is expanded to
  `$HOME`.

```sh
popterm                                              # toggle WezTerm
popterm org.wezfurlong.wezterm wezterm               # toggle, or start WezTerm
```

### GNOME Wayland

```sh
popterm-wayland [OPTIONS] [APP_ID [COMMAND...]]
popterm-wayland --geometry [APP_ID]
```

| Option | Description |
|--------|-------------|
| `--position top-center` | Position the window the first time it is shown |
| `--top-margin PIXELS` | Offset below the top edge of the monitor work area |
| `--x-offset PIXELS` | Horizontal offset from center (positive is right) |
| `--geometry` | Print the current frame and work-area geometry |

```sh
popterm-wayland org.wezfurlong.wezterm wezterm
popterm-wayland --position top-center --top-margin 20 --x-offset 80 \
  org.wezfurlong.wezterm wezterm
```

On Wayland, a shown window is also minimized automatically when focus moves to
another window.

### Keyboard shortcut

Bind the command to a global shortcut in your desktop environment, for example
a GNOME custom keyboard shortcut, so one key press toggles the terminal.

## Project layout

```
src/main.rs                X11 edition
src/bin/popterm-wayland.rs Wayland command-line frontend
gnome-extension/           GNOME Shell extension used by the Wayland edition
build.sh                   Build and install the X11 edition
install-wayland.sh         Install the Wayland edition and extension
popterm.go                 Legacy Go prototype
```

## License

MIT. Copyright (C) 2025 Pavel Vitis.
