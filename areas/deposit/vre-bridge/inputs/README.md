# The VRE bridge's shipped inputs

One directory per project, named by its shortcode (`0803/`), holding the files the bridge copies into each of the
project's `media` generations besides what it downloads: a CPE project's `config.kdl`, `curation.csv` and narrative
pages, or a DaTEI project's `datei.toml`, `file-layout.xml` and `file-mapping.json` (which the
bridge also reads, and which needs an `EDITOR_VRE_BRIDGE_IIIF_BASES` entry for the project's server). Their digest is part of the Ready token, so a change
here refreshes the project on the next nightly run after the release that ships it.

These files are the one authoritative copy and are edited here. `PROVENANCE` says where each came from, and which
committed fixture follows them.

The image places this directory at `/app/vre-bridge/inputs` (`EDITOR_VRE_BRIDGE_INPUTS_DIR`). This file keeps the
directory in the image whichever projects ship inputs; it and `PROVENANCE` are not read, since only `<shortcode>/`
directories are. See `docs/src/vre-bridge/operations.md`.
