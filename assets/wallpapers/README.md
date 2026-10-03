# Official built-in wallpapers

Each immediate subdirectory is one built-in wallpaper. The directory name is
for organization; the stable manifest `id` identifies the wallpaper across
package upgrades.

A complete entry contains:

- `manifest.json` using the GnomeEngine wallpaper manifest schema;
- the thumbnail named by the manifest (PNG remains the user-import default;
  official assets may use WebP);
- the local video file named by `content.entry`.

Do not add copyrighted media unless GnomeEngine has explicit rights to
redistribute it. Before a public release, fill in the actual title, author,
copyright holder, and license in the manifest. The media license is separate
from the GnomeEngine source-code license; never infer or default it to GPL.

For the first asset, prefer a short seamless, video-only H.264 MP4 loop around
1920×1080 at 30 FPS, roughly 8–15 seconds, with a reasonable bitrate. This is
guidance, not an automatic transcoding requirement.

Incomplete directories are ignored by development builds and package builds.
When `manifest.json`, its referenced thumbnail, and its referenced video are
all present, the next Debian package build includes the whole directory
automatically under `/usr/share/gnomeengine/wallpapers/`.
