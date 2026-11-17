# Data and static files are not executable: mode 0644, as a Dockerfile `COPY` of a checkout gives
# them. tar.bzl's `mtree_spec` writes 0755 for every entry. Runs after tar.bzl's default pipeline.
/ type=file / { sub(/mode=[0-7]+/, "mode=0644") }
