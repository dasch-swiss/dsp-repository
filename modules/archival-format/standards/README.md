# Standards reference

Verbatim layout-preserved text extracts of the standards DAO defers to. The PDFs are the canonical source; the `.md` siblings exist for fast grep/cross-reference from `dao-discovery.md` and from future-Claude sessions.

| Standard | Citation prefix | File |
|---|---|---|
| OAIS Reference Model (CCSDS 650.0-M-3, Dec 2024) | `OAIS §<n.n.n>` | [`oais-v3.md`](./oais-v3.md) |
| PREMIS Data Dictionary v3.0 | `PREMIS DD §<n.n>` or `PREMIS DD: <semanticUnitName>` | [`premis-3-data-dictionary.md`](./premis-3-data-dictionary.md) |
| PREMIS 3.0 OWL Ontology Guidelines | `PREMIS OWL §<n.n>` | [`premis-3-owl-guidelines.md`](./premis-3-owl-guidelines.md) |

## Working policy (per decision 27)

Every design choice in `dao-discovery.md` is aligned with OAIS and PREMIS, **or** carries a documented deviation with reason. Deviations are recorded inline in the relevant section of the discovery doc and surface in the decision-log row for that decision.

## How to cite

Inline in the discovery doc:

> Bitstreams are addressed by content hash (see `PREMIS DD: objectCharacteristics > fixity` and `OAIS §4.2.4` on Fixity Information).

A reader can `grep -n "objectCharacteristics" premis-3-data-dictionary.md` to land on the exact wording.

## How the extracts were produced

```sh
nix shell nixpkgs#poppler-utils --command \
  pdftotext -layout <pdf> <txt>
```

Re-run if the PDFs are updated. Page numbers and running headers from the PDFs leak into the text streams — this is normal for `pdftotext -layout` and is acceptable for a reference index.
