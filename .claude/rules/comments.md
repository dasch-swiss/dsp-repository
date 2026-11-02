# Comments

Default: no comment. Code, names, types and tests say what the code does; the surrounding comment density is not a reason to add one.

Write one only when a reader would otherwise get it wrong: a non-obvious why, an invariant, a surprising behaviour of an external API. One or two lines.

Never: restate the code; describe history or the session ("now", "previously", "fixed", "changed to", plan steps, review findings); leave commented-out code, banners, or a TODO without an issue id.

Doc comments follow the same rule, unless a linter or a published API requires them.

In the code you change, delete a comment that restates it or narrates history; keep a why or an invariant unless your change makes it false, then fix or delete it. Tool directives and license headers are not comments.
