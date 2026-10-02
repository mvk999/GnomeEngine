# Product Vision

GnomeEngine aims to provide a complete live-wallpaper experience comparable in
product scope to Wallpaper Engine and PrimoEngine, with its own identity,
implementation, and GNOME-focused interaction model. These products are
references for user journeys and capabilities only; their code, assets, visual
identity, and formats are not part of this project.

The product is intended to be:

- Native to GNOME and focused on Wayland.
- Extremely considerate of CPU, memory, GPU, battery, wakeups, and Shell
  responsiveness.
- Intended to be open-source, local-first, and usable without accounts,
  analytics, or network access. The project's license decision is pending.
- Focused on preserving the user's normal desktop workflow.

> The wallpaper should enhance the desktop without becoming a workload.

The long-term direction may include a local library, video, shader and web
wallpapers, multi-monitor assignment, smart pausing, custom parameters,
playlists, scheduling, creator tools, and a community catalog. These are
product goals, not claims about current implementation. See the
[roadmap](roadmap.md) and [architecture map](../../ARCHITECTURE.md) for status.

## Initial platform boundary

The initial target is GNOME 50+ on Wayland, with Ubuntu as the primary
development and distribution platform. Other desktop environments, X11,
Windows, and macOS are outside the current support scope.

## Product priorities

1. Keep GNOME stable.
2. Preserve the user's workflow.
3. Minimize resource use when a wallpaper is visible and especially when it is
   not visible.
4. Prefer correct, maintainable mechanisms over visual features that add
   persistent work.
