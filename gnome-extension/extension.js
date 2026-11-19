import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'org.popterm.Toggle';
const OBJECT_PATH = '/org/popterm/Toggle';

// Stable sequence numbers of windows Popterm has already positioned. This lives
// at module level because GNOME Shell disables extensions while the screen is
// locked, and unlocking must not reposition a window the user has arranged.
const placedWindows = new Set();

const INTERFACE_XML = `
<node>
  <interface name="org.popterm.Toggle">
    <method name="Toggle">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="b" name="found" direction="out"/>
    </method>
    <method name="Show">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="b" name="found" direction="out"/>
    </method>
    <method name="TogglePositioned">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="s" name="position" direction="in"/>
      <arg type="i" name="top_margin" direction="in"/>
      <arg type="i" name="x_offset" direction="in"/>
      <arg type="b" name="found" direction="out"/>
    </method>
    <method name="ShowPositioned">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="s" name="position" direction="in"/>
      <arg type="i" name="top_margin" direction="in"/>
      <arg type="i" name="x_offset" direction="in"/>
      <arg type="b" name="found" direction="out"/>
    </method>
    <method name="Geometry">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="b" name="found" direction="out"/>
      <arg type="i" name="x" direction="out"/>
      <arg type="i" name="y" direction="out"/>
      <arg type="i" name="width" direction="out"/>
      <arg type="i" name="height" direction="out"/>
      <arg type="i" name="work_x" direction="out"/>
      <arg type="i" name="work_y" direction="out"/>
      <arg type="i" name="work_width" direction="out"/>
      <arg type="i" name="work_height" direction="out"/>
    </method>
  </interface>
</node>`;

function windowMatches(window, wanted) {
    const identifiers = [
        window.get_gtk_application_id(),
        window.get_wm_class(),
        window.get_wm_class_instance(),
    ];
    return identifiers.some(value => value !== null && value === wanted);
}

export default class PoptermExtension extends Extension {
    enable() {
        this._watchedWindow = null;
        this._watchedWindowUnmanagedId = 0;
        this._focusCheckId = 0;
        this._focusChangedId = global.display.connect(
            'notify::focus-window',
            () => this._queueFocusCheck()
        );

        this._dbusObject = Gio.DBusExportedObject.wrapJSObject(INTERFACE_XML, {
            Toggle: appId => this._toggle(appId),
            Show: appId => this._show(appId),
            TogglePositioned: (appId, position, topMargin, xOffset) =>
                this._toggle(appId, position, topMargin, xOffset),
            ShowPositioned: (appId, position, topMargin, xOffset) =>
                this._show(appId, position, topMargin, xOffset),
            Geometry: appId => this._geometry(appId),
        });
        this._dbusObject.export(Gio.DBus.session, OBJECT_PATH);
        this._nameOwnerId = Gio.bus_own_name_on_connection(
            Gio.DBus.session,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            null,
            null
        );
    }

    disable() {
        if (this._focusChangedId) {
            global.display.disconnect(this._focusChangedId);
            this._focusChangedId = 0;
        }
        if (this._focusCheckId) {
            GLib.source_remove(this._focusCheckId);
            this._focusCheckId = 0;
        }
        this._watchWindow(null);

        if (this._nameOwnerId) {
            Gio.bus_unown_name(this._nameOwnerId);
            this._nameOwnerId = 0;
        }
        this._dbusObject?.unexport();
        this._dbusObject = null;
    }

    _findWindow(appId) {
        // NORMAL_ALL is in most-recently-used order and covers all workspaces.
        const windows = global.display.get_tab_list(Meta.TabList.NORMAL_ALL, null);
        return windows.find(candidate => windowMatches(candidate, appId));
    }

    _placeWindow(window, position, topMargin, xOffset) {
        if (!position)
            return;
        if (position !== 'top-center')
            throw new Error(`unsupported Popterm position: ${position}`);

        const frame = window.get_frame_rect();
        const workArea = window.get_work_area_current_monitor();
        const x = workArea.x + Math.floor((workArea.width - frame.width) / 2) + xOffset;
        const y = workArea.y + Math.max(0, topMargin);
        window.move_frame(true, x, y);
    }

    _queuePlacement(window, position, topMargin, xOffset) {
        if (!position)
            return;

        // Position each window only once so that later moves and resizes by
        // the user survive hiding and showing it again.
        const sequence = window.get_stable_sequence();
        if (placedWindows.has(sequence))
            return;
        placedWindows.add(sequence);

        const place = () => {
            if (!window.minimized)
                this._placeWindow(window, position, topMargin, xOffset);
            return GLib.SOURCE_REMOVE;
        };

        // Move once as soon as the window is available, then again after
        // window-management extensions (notably PaperWM) have classified a
        // newly mapped scratch window and applied their initial placement.
        GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, place);
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, place);
    }

    _toggle(appId, position = '', topMargin = 0, xOffset = 0) {
        const window = this._findWindow(appId);
        if (!window)
            return false;

        this._watchWindow(window);
        if (window.minimized) {
            window.unminimize();
            window.activate(global.get_current_time());
            this._queuePlacement(window, position, topMargin, xOffset);
        } else {
            window.minimize();
        }
        return true;
    }

    _show(appId, position = '', topMargin = 0, xOffset = 0) {
        const window = this._findWindow(appId);
        if (!window)
            return false;

        this._watchWindow(window);
        if (window.minimized)
            window.unminimize();
        window.activate(global.get_current_time());
        this._queuePlacement(window, position, topMargin, xOffset);
        return true;
    }

    _geometry(appId) {
        const window = this._findWindow(appId);
        if (!window)
            return [false, 0, 0, 0, 0, 0, 0, 0, 0];
        const frame = window.get_frame_rect();
        const workArea = window.get_work_area_current_monitor();
        return [
            true,
            frame.x, frame.y, frame.width, frame.height,
            workArea.x, workArea.y, workArea.width, workArea.height,
        ];
    }

    _watchWindow(window) {
        if (this._watchedWindow === window)
            return;

        if (this._watchedWindow && this._watchedWindowUnmanagedId)
            this._watchedWindow.disconnect(this._watchedWindowUnmanagedId);

        this._watchedWindow = window;
        this._watchedWindowUnmanagedId = 0;
        if (window) {
            this._watchedWindowUnmanagedId = window.connect('unmanaged', () => {
                this._watchedWindow = null;
                this._watchedWindowUnmanagedId = 0;
            });
        }
    }

    _queueFocusCheck() {
        if (this._focusCheckId)
            return;

        // Let an activate/unminimize operation finish before examining focus.
        this._focusCheckId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._focusCheckId = 0;
            const window = this._watchedWindow;
            if (window && !window.minimized && global.display.focus_window !== window)
                window.minimize();
            return GLib.SOURCE_REMOVE;
        });
    }
}
