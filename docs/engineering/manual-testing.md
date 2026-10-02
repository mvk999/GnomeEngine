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

## M3 smart lifecycle checklist

Run only in a disposable or nested supported GNOME 50+ Wayland session. The
current GTK preview is not a desktop background, so successful lifecycle
signals do not establish desktop wallpaper visibility.

- [ ] renderer service starts and exports its D-Bus API
- [ ] video playback starts and manual Pause/Resume preserves position
- [ ] fullscreen automatically pauses and leaving fullscreen resumes
- [ ] a visible secondary rendered output prevents global fullscreen pause
- [ ] lock pauses; repeated lock/unlock does not stick or duplicate reasons
- [ ] battery transition pauses; AC reconnect resumes when appropriate
- [ ] display power changes pause/resume on supported built-in panel hardware
- [ ] suspend pauses; resume removes only `system-sleep`
- [ ] monitor hotplug recomputes fullscreen/output state without a crash
- [ ] duplicate reason updates do not emit duplicate reason-change signals
- [ ] removing one of multiple reasons never resumes early
- [ ] Stop while auto-paused remains Stopped after the condition clears
- [ ] applying while an automatic reason is active remains paused
- [ ] renderer restart receives a current Shell lifecycle snapshot
- [ ] renderer disappearance/crash does not crash GNOME Shell
- [ ] disabling/re-enabling the extension leaves no unintended stale reason
- [ ] Alt+Tab, Overview, workspace switching, and input behavior are unchanged
- [ ] measure playing, paused, and stopped CPU/RSS with identical media/session

Record unavailable battery, display, suspend, multi-monitor, or performance
tests explicitly; never infer them from unit tests.
