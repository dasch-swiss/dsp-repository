"""The application images and their layers.

An image is built for `//platforms:linux_x86_64_musl`: a static binary on distroless `static`.
Its layers carry `target_compatible_with = MUSL`, so `//...` on a host platform skips them; the
image reaches the musl platform through a transition, which leaves `oci_load` and `oci_push` on
the host, where their tools run. Every layer entry is owned by root with modification time 0, so
a layer's digest depends only on the files it holds.
"""

load("@bazel_lib//lib:copy_to_directory.bzl", "copy_to_directory")
load("@bazel_lib//lib:transitions.bzl", "platform_transition_filegroup")
load("@rules_oci//oci:defs.bzl", "oci_image", "oci_load", "oci_push")
load("@tar.bzl", "mutate", "tar")

MUSL = ["@llvm//constraints/libc:musl"]

def app_image(name, repository, scan_tag, **kwargs):
    """`name` (the image), `name_load` and `name_push`.

    `name_load` loads the image into Docker as `scan_tag`, the tag the Scout workflow scans; CI
    adds its publish tags with `-- --tag`. `name_push` pushes to `repository`, with the tags the
    caller passes (`-- --tag <tag>`). Both are `manual`: `//...` does not build the images. Push
    from CI only: the data layers take every file under their directories, so a checkout's
    untracked files would be published with them.

    Args:
        name: the image's name.
        repository: where `name_push` pushes.
        scan_tag: the tag `name_load` gives the image.
        **kwargs: passed to `oci_image`; `base` defaults to distroless `static`.
    """
    kwargs.setdefault("base", "@distroless_static")
    oci_image(
        name = name + "_musl",
        target_compatible_with = MUSL,
        visibility = ["//visibility:private"],
        **kwargs
    )
    platform_transition_filegroup(
        name = name,
        srcs = [":" + name + "_musl"],
        tags = ["manual"],
        target_platform = Label("//platforms:linux_x86_64_musl"),
    )
    oci_load(
        name = name + "_load",
        image = ":" + name,
        repo_tags = [scan_tag],
        tags = ["manual"],
    )
    oci_push(
        name = name + "_push",
        image = ":" + name,
        repository = repository,
        tags = ["manual"],
    )

def binary_layer(name, binary, path, **kwargs):
    """A layer holding one executable at `path`. The mtree lists the archive's entries, so the
    binary's runfiles stay out.

    Args:
        name: the layer's name.
        binary: the binary target.
        path: its absolute path in the image.
        **kwargs: passed to `tar`.
    """
    if not path.startswith("/"):
        fail("path must be absolute: " + path)
    parts = path[1:].split("/")
    lines = [
        "./{} type=dir mode=0755 uid=0 gid=0 time=0".format("/".join(parts[:i + 1]))
        for i in range(len(parts) - 1)
    ]
    lines.append(
        ".{} type=file mode=0755 uid=0 gid=0 time=0 content=$(execpath {})".format(path, binary),
    )
    tar(
        name = name,
        srcs = [binary],
        mtree = lines,
        **kwargs
    )

def files_layer(name, srcs, prefixes, **kwargs):
    """A layer of files, each placed by the first matching entry of `prefixes`.

    Args:
        name: the layer's name.
        srcs: the files, and directories such as a stylesheet's output.
        prefixes: maps a repository path prefix to the path in the image it moves to, without the
            leading `/`. A file under no prefix is not in the layer.
        **kwargs: passed to `tar`.
    """
    tree = name + "_tree"
    copy_to_directory(
        name = tree,
        srcs = srcs,
        include_srcs_patterns = [p + "/**" for p in prefixes],
        replace_prefixes = prefixes,
        root_paths = [],
        visibility = ["//visibility:private"],
    )
    tar(
        name = name,
        srcs = [":" + tree],
        mutate = mutate(
            includes = [Label("//tools/image:file_mode.awk")],
            mtime = "0",
            strip_prefix = native.package_name() + "/" + tree,
        ),
        **kwargs
    )
