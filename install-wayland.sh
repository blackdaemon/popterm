#!/usr/bin/env bash
set -euo pipefail

uuid='popterm-wayland-bridge@blackdaemon'
source_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/gnome-extension"
extension_dir="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$uuid"

mkdir -p "$extension_dir"
install -m 0644 "$source_dir/extension.js" "$source_dir/metadata.json" "$extension_dir/"
cargo install --path "$(dirname -- "$source_dir")" --bin popterm-wayland

if gnome-extensions enable "$uuid"; then
    echo "Popterm Wayland is installed and enabled."
else
    echo "Installed, but GNOME Shell has not loaded the new extension yet."
    echo "Log out and back in, then run: gnome-extensions enable $uuid"
fi
