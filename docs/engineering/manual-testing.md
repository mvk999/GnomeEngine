# Manual Testing

The current PoC only opens an ordinary GTK window. Manual playback checks do
not imply that desktop-background integration exists.

For a graphical validation run, record GNOME Shell version, distribution,
session type, GPU/driver, test video codec/resolution/FPS, and exact steps.

## Current prototype smoke check

1. Confirm the GTK4 `gtk4paintablesink` plugin is discoverable.
2. Run the renderer with a local video containing a video stream.
3. Confirm the GTK window displays video, audio is discarded, and playback
   seeks to the beginning after end-of-stream.
4. Close the window and confirm the process exits without leaving playback
   running.
5. Try an invalid path and a video with an unsupported codec; record the
   observed error.

## Future desktop integration checklist

Once a supported GNOME 50+ Wayland bridge exists, validate background
placement, Alt+Tab, Overview, workspaces, focus/input pass-through, fullscreen
pause/resume, lock/unlock, suspend/resume, extension disable/enable, app
reopen, Stop cleanup, and monitor hotplug. Run these in a disposable or nested
session where possible. Record any item that cannot be tested.
