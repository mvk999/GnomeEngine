# ADR-0003: Use GStreamer for the Video Proof of Concept

Status: Accepted for the prototype

## Context

The project needs to validate local video decode and GTK display without
implementing media demuxing, codec selection, or frame timing itself.

## Decision

The prototype uses GStreamer `playbin` with `gtk4paintablesink`; audio is sent
to `fakesink`. GStreamer autoplugging selects the available decode path.

Implementation note: where the optional GTK4 paintable sink is not packaged,
the renderer uses GTK's `GtkVideo` media backend (also backed by GStreamer) to
preserve the same muted, looped playback and lifecycle controls.

## Consequences

The prototype reuses the system media stack and keeps codec logic out of the
project. Actual hardware acceleration, DMA-BUF import, zero-copy behavior, and
resource use remain unmeasured. The GTK paintable is currently displayed in a
normal window and does not solve Shell background integration.
