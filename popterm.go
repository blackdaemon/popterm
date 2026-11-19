//
// popterm.go
// Copyright (C) 2025 Pavel Vitis <pavelvitis@gmail.com>
//
// Distributed under terms of the MIT license.
//

package main

import (
	"bytes"
	"fmt"
	"os/exec"
	"strings"
)


// Set the window title or a part of it
const wmClass = "org.wezfurlong.wezterm"


func getWindowIDByWMClass() (string, error) {
	// Use xdotool to find window ID by WM_CLASS
	cmd := exec.Command("bash", "-c", fmt.Sprintf("xdotool search --class %s", wmClass))
	var out bytes.Buffer
	cmd.Stdout = &out
	err := cmd.Run()
	if err != nil {
		return "", fmt.Errorf("failed to find window: %v", err)
	}

	// Get the first window ID
	winIDs := strings.Fields(strings.TrimSpace(out.String()))
	if len(winIDs) == 0 {
		return "", fmt.Errorf("no window found with WM_CLASS '%s'", wmClass)
	}

	fmt.Println(winIDs[0])

	return winIDs[0], nil
}

// Check if the window is minimized using xprop and _NET_WM_STATE(ATOM)
func isWindowMinimized(winID string) bool {
	// Use xprop to get window states
	cmd := exec.Command("bash", "-c", fmt.Sprintf("xprop -id %s | grep '_NET_WM_STATE'", winID))
	var out bytes.Buffer
	cmd.Stdout = &out
	err := cmd.Run()
	if err != nil {
		fmt.Println("Error checking window state:", err)
		return false
	}
	fmt.Println(out.String())

	return strings.Contains(out.String(), "_NET_WM_STATE_MINIMIZED") || strings.Contains(out.String(), "_NET_WM_STATE_HIDDEN") 
}

func toggleWindow() {
	winID, err := getWindowIDByWMClass()
	if err != nil {
		fmt.Println("Window not found:", err)
		return
	}

	// Check if window is currently visible
	cmd := exec.Command("bash", "-c", fmt.Sprintf("xwininfo -id %s | grep 'Map State'", winID))
	var out bytes.Buffer
	cmd.Stdout = &out
	err = cmd.Run()
	if err != nil {
		fmt.Println("Error checking window state:", err)
		return
	}

	//output := out.String()
	if !isWindowMinimized(winID) {
		fmt.Println("Hiding window...")
		exec.Command("xdotool", "windowminimize", winID).Run()
	} else {
		fmt.Println("Showing window...")
		exec.Command("xdotool", "windowmap", winID).Run()
		exec.Command("xdotool", "windowactivate", winID).Run()
	}
}

func main() {
	toggleWindow()
}

