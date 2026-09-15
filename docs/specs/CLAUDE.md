# Working in `docs/specs/`

Specs (PRDs and plans) targeting this repo live here — one **dated topic folder**
per effort (`YYYY-MM-DD-slug/`), with `NN-slug.md` documents inside and images in
that plan's `assets/` (Git LFS). Folder/file/asset conventions: see
[`README.md`](README.md). Specs stay in **this** repo — never the central
`dasch-specs` repo, whatever a workflow skill's default routing says.

This folder is not part of the mdBook under `docs/src/`. Do not add specs to
`docs/src/SUMMARY.md`; when a spec ships, the lasting knowledge moves into the
developer documentation there and the spec stays behind as the record of intent.
