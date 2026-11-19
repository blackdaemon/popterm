//
// popterm.rs
// Copyright (C) 2025 Pavel Vitis <pavelvitis@gmail.com>
//
// Distributed under terms of the MIT license.
//

use std::process::{exit, Command};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

const DEFAULT_WM_CLASS: &str = "org.wezfurlong.wezterm";

// EWMH atom names
const _NET_WM_STATE: &str = "_NET_WM_STATE";
const _NET_WM_STATE_HIDDEN: &str = "_NET_WM_STATE_HIDDEN";
const _NET_WM_STATE_MINIMIZED: &str = "_NET_WM_STATE_MINIMIZED";
const WM_CLASS: &str = "WM_CLASS";

fn get_window_id_by_wm_class(conn: &RustConnection, wm_class: &str) -> Result<u32, String> {
    let screen = conn.setup().roots[0].root;
    
    // Get all child windows
    let tree = conn.query_tree(screen)
        .map_err(|e| format!("failed to query tree: {:?}", e))?
        .reply()
        .map_err(|e| format!("failed to get reply: {:?}", e))?;
    
    // Convert WM_CLASS string to atom
    let wm_class_atom = conn.intern_atom(false, WM_CLASS.as_bytes())
        .map_err(|e| format!("failed to intern WM_CLASS atom: {:?}", e))?
        .reply()
        .map_err(|e| format!("failed to get WM_CLASS atom reply: {:?}", e))?
        .atom;
    
    // Search through all windows
    for &window in &tree.children {
        // Get WM_CLASS property
        let prop = conn.get_property(
            false,
            window,
            wm_class_atom,
            AtomEnum::STRING,
            0,
            u32::MAX,
        );
        
        match prop {
            Ok(cookie) => {
                if let Ok(reply) = cookie.reply() {
                    if let Ok(class_str) = String::from_utf8(reply.value) {
                        // WM_CLASS format is "instance\0class\0"
                        let parts: Vec<&str> = class_str.split('\0').collect();
                        if parts.len() >= 2 {
                            let instance = parts[0];
                            let class = parts[1];
                            
                            // Check if WM_CLASS matches (instance or class)
                            if instance == wm_class || class == wm_class {
                                println!("{}", window);
                                return Ok(window);
                            }
                        }
                    }
                }
            }
            Err(_) => continue, // Window might not have WM_CLASS, skip it
        }
    }
    
    Err(format!("no window found with WM_CLASS '{}'", wm_class))
}

// Check if the window is minimized using _NET_WM_STATE
fn is_window_minimized(conn: &RustConnection, win_id: u32) -> Result<bool, String> {
    // Get _NET_WM_STATE atom
    let net_wm_state_atom = conn.intern_atom(false, _NET_WM_STATE.as_bytes())
        .map_err(|e| format!("failed to intern _NET_WM_STATE atom: {:?}", e))?
        .reply()
        .map_err(|e| format!("failed to get _NET_WM_STATE atom reply: {:?}", e))?
        .atom;
    
    // Get _NET_WM_STATE_HIDDEN atom
    let net_wm_state_hidden_atom = conn.intern_atom(false, _NET_WM_STATE_HIDDEN.as_bytes())
        .map_err(|e| format!("failed to intern _NET_WM_STATE_HIDDEN atom: {:?}", e))?
        .reply()
        .map_err(|e| format!("failed to get _NET_WM_STATE_HIDDEN atom reply: {:?}", e))?
        .atom;
    
    // Get _NET_WM_STATE_MINIMIZED atom
    let net_wm_state_minimized_atom = conn.intern_atom(false, _NET_WM_STATE_MINIMIZED.as_bytes())
        .map_err(|e| format!("failed to intern _NET_WM_STATE_MINIMIZED atom: {:?}", e))?
        .reply()
        .map_err(|e| format!("failed to get _NET_WM_STATE_MINIMIZED atom reply: {:?}", e))?
        .atom;
    
    // Get _NET_WM_STATE property
    let prop = conn.get_property(
        false,
        win_id,
        net_wm_state_atom,
        AtomEnum::ATOM,
        0,
        u32::MAX,
    )
    .map_err(|e| format!("failed to get _NET_WM_STATE property: {:?}", e))?;
    
    let reply = prop.reply()
        .map_err(|e| format!("failed to get _NET_WM_STATE reply: {:?}", e))?;
    
    // Check if window state contains HIDDEN or MINIMIZED
    let states: Vec<u32> = reply.value32()
        .map(|iter| iter.collect())
        .unwrap_or_default();
    let is_hidden = states.contains(&net_wm_state_hidden_atom);
    let is_minimized = states.contains(&net_wm_state_minimized_atom);
    
    // Also check window map state (unmapped windows are effectively minimized)
    let attrs = conn.get_window_attributes(win_id)
        .map_err(|e| format!("failed to get window attributes: {:?}", e))?
        .reply()
        .map_err(|e| format!("failed to get window attributes reply: {:?}", e))?;
    
    let is_unmapped = attrs.map_state == MapState::UNMAPPED;
    
    Ok(is_hidden || is_minimized || is_unmapped)
}

fn toggle_window(conn: &RustConnection, wm_class: &str, run_if_not_found: Option<String>) {
    let win_id = match get_window_id_by_wm_class(conn, wm_class) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Window not found: {}", e);
            if let Some(ref program) = run_if_not_found {
                let parts: Vec<&str> = program.split_whitespace().collect();
                if !parts.is_empty() {
                    let (cmd, args) = (parts[0], &parts[1..]);
                    let _ = Command::new(cmd).args(args).spawn();
                }
            }
            exit(1);
        }
    };
    
    // Check window map state directly
    let attrs = conn.get_window_attributes(win_id)
        .ok()
        .and_then(|c| c.reply().ok());
    
    let is_mapped = attrs.map(|a| a.map_state == MapState::VIEWABLE).unwrap_or(false);
    
    let is_minimized = match is_window_minimized(conn, win_id) {
        Ok(minimized) => minimized,
        Err(e) => {
            eprintln!("Error checking window state: {}, using map state", e);
            !is_mapped
        }
    };
    
    eprintln!("Window {}: mapped={}, minimized={}", win_id, is_mapped, is_minimized);
    
    if !is_minimized {
        println!("Minimizing window...");
        let _ = Command::new("xdotool")
            .args(&["windowminimize", &win_id.to_string()])
            .output();
    } else {
        println!("Showing window...");
        
        // Use xdotool for restore - native X11 methods don't work reliably
        // with all window managers for restoring from iconic state
        let _ = Command::new("xdotool")
            .args(&["windowmap", &win_id.to_string()])
            .output();
        let _ = Command::new("xdotool")
            .args(&["windowactivate", &win_id.to_string()])
            .output();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let wm_class = args.get(1).map(|s| s.as_str()).unwrap_or(DEFAULT_WM_CLASS);
    let run_if_not_found = if args.len() > 2 {
        let s = args[2..].join(" ");
        let expanded = std::env::var("HOME").map(|home| s.replace('~', &home)).unwrap_or(s);
        Some(expanded)
    } else {
        None
    };
    
    let (conn, _) = match RustConnection::connect(None) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to connect to X11 server: {:?}", e);
            exit(1);
        }
    };
    
    toggle_window(&conn, wm_class, run_if_not_found);
}
