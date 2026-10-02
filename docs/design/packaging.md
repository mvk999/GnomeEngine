# Packaging Direction

No installer or package metadata exists yet. The first intended native package
is an Ubuntu/Debian `.deb`; build and runtime dependencies must be explicit and
the package must not require Cargo, Node.js, or root access at runtime beyond
normal system installation.

Before adding package files, verify current Debian/Ubuntu filesystem,
AppStream, desktop-entry, GSettings, D-Bus activation, and GNOME Shell extension
conventions. The package must not remove user wallpaper content or personal
settings on uninstall. Release builds should be reproducible from repository
scripts and must not install software or mutate the build host.

Flatpak, Snap, and AppImage are out of scope until their desktop integration
constraints are designed.
