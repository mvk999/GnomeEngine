# Execution Plans

Create a living ExecPlan under `active/` for work that crosses components,
changes architecture, or needs multiple implementation slices. Small isolated
fixes do not need a plan.

Include the goal, reason, current state, scope/non-goals, relevant files,
architecture impact, implementation slices, test strategy, performance and
security implications, acceptance criteria, progress, decisions discovered,
and post-implementation notes. Update it when evidence changes direction.

Move a completed plan to `completed/`. Keep `tech-debt.md` for important
deferred problems with explicit revisit triggers, not minor cleanup.
