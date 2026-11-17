"""`tailwind_css`: one stylesheet, built with the Tailwind standalone CLI."""

def _tailwind_css_impl(ctx):
    if ctx.attr.hashed:
        out = ctx.actions.declare_directory(ctx.label.name)
        dest = out.path + "/app.css"
    else:
        out = ctx.actions.declare_file(ctx.label.name + "/app.css")
        dest = out.path

    # The hashed form is the one `just css-release` writes: the first eight hex digits of the
    # stylesheet's SHA-256 in the name. The servers find it by scanning their asset directory.
    rename = """
if command -v sha256sum >/dev/null 2>&1; then h=$(sha256sum "$3" | cut -c1-8); else h=$(shasum -a 256 "$3" | cut -c1-8); fi
mv "$3" "$(dirname "$3")/app.$h.css"
""" if ctx.attr.hashed else ""

    ctx.actions.run_shell(
        inputs = [ctx.file.src] + ctx.files.srcs,
        outputs = [out],
        tools = [ctx.executable._tailwind],
        arguments = [ctx.executable._tailwind.path, ctx.file.src.path, dest],
        # The CLI reports its version and timing on stderr on every run; shown only on failure.
        command = """set -eu
log=$(mktemp)
"$1" -i "$2" -o "$3" --minify 2>"$log" || { cat "$log" >&2; exit 1; }
""" + rename,
        mnemonic = "TailwindCss",
        progress_message = "Building stylesheet %{label}",
    )
    return [DefaultInfo(files = depset([out]))]

tailwind_css = rule(
    implementation = _tailwind_css_impl,
    doc = """Builds `app.css` from a Tailwind entry file, minified.

The entry files use `source(none)`: Tailwind reads only the files listed in `srcs`, and a class
used in a file that is missing from them is silently left out of the stylesheet. `srcs` therefore
holds every file the entry imports and every file its `@source` globs match.""",
    attrs = {
        "src": attr.label(
            doc = "The entry stylesheet.",
            allow_single_file = [".css"],
            mandatory = True,
        ),
        "srcs": attr.label_list(
            doc = "The stylesheets the entry imports and the files its `@source` lines scan.",
            allow_files = True,
        ),
        "hashed": attr.bool(
            doc = "Output a directory holding `app.<hash8>.css` instead of a file `app.css`.",
        ),
        "_tailwind": attr.label(
            default = "//tools/tailwind",
            executable = True,
            cfg = "exec",
        ),
    },
)
