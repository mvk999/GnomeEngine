# ADR-0001: Keep Video Work Outside GNOME Shell

Status: Accepted

## Context

GNOME Shell is responsible for the interactive desktop. Media decode and
rendering can be resource-intensive and can fail on malformed media or codec
plugins.

## Decision

Video decoding and playback run in a process external to GNOME Shell. A future
extension may bridge lifecycle and compositor integration but must not decode
video or perform heavy media work.

## Consequences

Positive: media failures are isolated from Shell, and renderer lifecycle can be
measured and controlled separately.

Trade-off: safely presenting an external renderer's content in the desktop
background requires a compositor surface integration mechanism that remains
unresolved.

## Alternatives considered

Decoding inside the Shell extension was rejected because it couples the most
failure-prone and expensive workload to the desktop shell.
