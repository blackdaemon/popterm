# Popterm on GNOME Wayland

Wayland does not expose X11's global window IDs, `WM_CLASS`, or permission for
one normal client to minimize and activate another. `xdotool` consequently
cannot implement Popterm for native Wayland windows. The Wayland edition uses a
small GNOME Shell extension as the privileged window-management bridge and a
separate Rust binary as the command-line frontend.

The existing X11 program and `src/main.rs` are unchanged.

## Install on GNOME 50

```sh
chmod +x install-wayland.sh
./install-wayland.sh
```

If GNOME has not seen the extension before, log out and back in, then run:

```sh
gnome-extensions enable popterm-wayland-bridge@blackdaemon
```

## Use

Toggle an existing WezTerm window:

```sh
popterm-wayland
```

Toggle it, or start WezTerm when no matching window exists:

```sh
popterm-wayland org.wezfurlong.wezterm wezterm
```

Place the window at the top center of its monitor the first time it is shown:

```sh
popterm-wayland --position top-center --top-margin 20 --x-offset 80 \
  org.wezfurlong.wezterm wezterm
```

Inspect the current frame and monitor work-area geometry:

```sh
popterm-wayland --geometry org.wezfurlong.wezterm
```

Positioning is performed by the GNOME Shell extension so that it works for
native Wayland windows. The margin is measured in pixels from the top edge of
the monitor's usable work area. The optional horizontal offset is relative to
the centered position: positive values move right and negative values move left.
Placement is repeated briefly after a new window is mapped so that scratch-layer
window managers such as PaperWM cannot overwrite Popterm's requested position.
Each window is positioned only once, so if you move or resize it afterwards it
keeps that geometry when hidden and shown again. A newly opened window, for
example after closing WezTerm or logging in, is positioned again.

The first argument is matched exactly against the Wayland GTK application ID,
XWayland `WM_CLASS`, or window instance. The remaining arguments are executed
directly (without a shell) when no window exists. Bind that command to a GNOME
custom keyboard shortcut. Once Popterm shows a matched window, the extension
also watches it and minimizes it automatically whenever focus moves to another
window.

To discover an application's identifier, open GNOME Looking Glass with
`Alt+F2`, enter `lg`, choose **Windows**, and inspect the window's app ID/class.

## Remove

```sh
gnome-extensions disable popterm-wayland-bridge@blackdaemon
rm -r ~/.local/share/gnome-shell/extensions/popterm-wayland-bridge@blackdaemon
cargo uninstall popterm-wayland
```
