---
description: Create a plan for a task using a specified subagent.
argument-hint: Description of the task to explore.
---

$ARGUMENTS

Ultrathink and make a detailed plan to accomplish this using the appropriate subagent.

How will we implement only the functionality we need right now?

Identify files that need to be changed.

Do not include plans for legacy fallback unless required or explicitly requested.
Explicitly mention to the user if functionality will be removed.

Write a short overview of what you are about to do:
- What components will be changed, added or.
- What markdoc tags will be changed, added or removed.
- What content pages will be changed, added, or removed.

Furthermore, make sure to decide and communicate which subagent will be used to execute the plan after users approval.

IMPORTANT: Even if the general purpose agent could be used to execute the task, start a subagent (even a general purpose one) so keep the context focused.
