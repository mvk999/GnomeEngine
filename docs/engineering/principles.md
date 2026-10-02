# Engineering Principles

## Small releases

Keep each change small enough to understand, review, test, and revert on its
own. Split unrelated formatting, refactoring, features, and documentation
cleanup into separate changes.

## TDD where it fits

For separable behavior, use red-green-refactor: write a test that demonstrates
the missing behavior, implement the smallest fix, then improve structure while
the test stays green. Do not force test-first development onto graphics or
compositor behavior that cannot be represented faithfully without the target
desktop session.

## Continuous refactoring

Refactor duplication, unclear names, oversized responsibilities, and misleading
abstractions while the relevant checks are green. Avoid a future rewrite by
keeping each improvement incremental.

## YAGNI and scope

Implement the smallest correct solution for GNOME and Wayland. Do not build a
generic desktop abstraction, plugin framework, distributed protocol, or future
platform adapter before a current requirement needs it.

## Pair programming with an agent

The human owns product direction and consequential trade-offs. The agent
investigates the repository and current technical sources, proposes the
simplest fitting implementation, validates it, and surfaces material risks.
When evidence changes a plan, update the plan and explain why rather than
following an obsolete design literally.

## Dependency discipline

Before adding a crate, package, or tool, evaluate necessity, maintenance,
security, binary/build/runtime cost, and existing Rust/GLib alternatives.

## Definition of done

As applicable, behavior is implemented, errors and cleanup are handled,
regression tests exist, `./scripts/check.sh` passes, docs are current, and
performance and security effects are considered. Report environmental limits
instead of implying validation occurred.
