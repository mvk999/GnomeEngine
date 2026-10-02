# Performance Engineering

Performance is a functional requirement. No low-resource, hardware-decode,
zero-copy, or battery claim is valid without evidence from a recorded run.

## Review questions

For renderer or lifecycle changes, consider whether they:

- Wake the CPU continuously or add polling.
- Copy frames between GPU and system memory unnecessarily.
- Keep a pipeline or surface alive while invisible or stopped.
- Create duplicate pipelines, renderer processes, or Shell work.
- Change source frame pacing or force conversion to a higher frame rate.

Prefer compositor/session events to polling, GStreamer pacing to custom frame
timers, lazy initialization, and full cleanup when playback ends.

## Measurement protocol

Compare GNOME idle, wallpaper playing, and wallpaper paused using the same
machine, session, media, and sample duration. Record:

- Date, distribution, GNOME/Mutter version, Wayland session, CPU, GPU, and
  driver.
- Video codec, dimensions, frame rate, duration, and bitrate when known.
- Renderer average and peak CPU, RSS, selected decoder, and hardware
  acceleration path when observable.
- GNOME Shell impact, GPU use, and wakeups when practical.
- Tool names, sample duration, state, and limitations.

Do not add benchmark dependencies to the developer machine without approval.
Do not commit large test videos; document the required media characteristics.

## Results

The measurement record and blank template live in
[docs/performance.md](../performance.md).
