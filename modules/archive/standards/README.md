# Standards reference

Verbatim layout-preserved text extracts of the standards DAO defers to. The PDFs are the canonical source; the `.md` siblings exist for fast grep/cross-reference from `dao-discovery.md` and from future-Claude sessions.

| Standard | Citation prefix | File |
|---|---|---|
| OAIS Reference Model (ISO 14721:2012 / CCSDS 650.0-M-3, Dec 2024) | `OAIS §<n.n.n>` | [`ISO 14721 - oais-v3.md`](./ISO%2014721%20-%20oais-v3.md) |
| PREMIS Data Dictionary v3.0 | `PREMIS DD §<n.n>` or `PREMIS DD: <semanticUnitName>` | [`premis-3-data-dictionary.md`](./premis-3-data-dictionary.md) |
| PREMIS 3.0 OWL Ontology Guidelines | `PREMIS OWL §<n.n>` | [`premis-3-owl-guidelines.md`](./premis-3-owl-guidelines.md) |
| **CoreTrustSeal Requirements 2026-2028 v01.00** — the entry-tier trustworthy-repository requirements (16 R-numbers) | `CTS R<nn>` | [`CoreTrustSeal-Requirements-2026-2028_v01.00.md`](./CoreTrustSeal-Requirements-2026-2028_v01.00.md) |
| **CoreTrustSeal Requirements 2026-2028 Extended Guidance v01.00** — companion guidance for each R-number | `CTS Guidance R<nn>` | [`CoreTrustSeal-Requirements-2026-2028-ExtendedGuidance_v01.00.md`](./CoreTrustSeal-Requirements-2026-2028-ExtendedGuidance_v01.00.md) |
| **CoreTrustSeal Requirements 2026-2028 Glossary v01.00** — terminology used in the catalog | `CTS Glossary: <term>` | [`CoreTrustSeal- Requirements-Glossary-2026-2028-v01.00.md`](./CoreTrustSeal-%20Requirements-Glossary-2026-2028-v01.00.md) |
| **CoreTrustSeal Curation & Preservation Levels Position Paper v3.0 (2024)** — Board position paper; recommends the **Z / D / C / A** taxonomy for curation commitments (Z = unattended storage; D = deposit compliance; C = initial curation; A = active preservation, required for CTS scope) | `CTS Levels: <Z\|D\|C\|A>` | [`CoreTrustSeal-CurationPreservationLevels-PositionPaper_v03_00.md`](./CoreTrustSeal-CurationPreservationLevels-PositionPaper_v03_00.md) |
| **nestor Kriterienkatalog vertrauenswürdige digitale Langzeitarchive v2 (2008)** — German-language; nestor Working Group "Trusted Archives — Certification" | `nestor §<n.n>` | [`nestor-kriterien-v2-2008.md`](./nestor-kriterien-v2-2008.md) |
| Audit and Certification of Trustworthy Digital Repositories (ISO 16363:2012 / CCSDS 652.0-M-2, Dec 2024) | `ISO 16363 §<n.n.n>` | [`ISO 16363 - certification.md`](./ISO%2016363%20-%20certification.md) |
| Requirements for Bodies Providing Audit and Certification of Candidate Trustworthy Digital Repositories (ISO 16919:2014 / CCSDS 652.1-M-3, Dec 2024) | `ISO 16919 §<n.n.n>` | [`ISO 16919 - requirements for auditors.md`](./ISO%2016919%20-%20requirements%20for%20auditors.md) |

**Note on ISO vs. CCSDS numbering.** The ISO numbers are stable identifiers used in citations; the actual document content we have checked in is the December 2024 CCSDS revision (which is what ISO publishes under the ISO number). Section numbers in citations refer to the **current CCSDS revision**, which is what the `.md` extract contains; the 2012/2014 ISO versions may have slightly different numbering in places.

**Certification pyramid (difficulty, lowest → highest).**

| Tier | Standard | Effort | Notes |
|---|---|---|---|
| 1 — entry | **CoreTrustSeal (CTS)** | low; community-run self-assessment + peer review | 16 requirements (R01-R16) in the **2026-2028 catalog** (the catalog is refreshed every 3 years; previous 2023-2025 catalog used a different ordering of R-numbers); good first target |
| 2 — middle | **nestor Seal** | medium; documented self-assessment + nestor peer review | 14 main criteria; German-language; sits between CTS and ISO 16363 |
| 3 — top | **ISO 16363 audit** | high; formal external audit by an ISO-16919-conforming body | The same criteria as nestor in substance, but with formal-audit rigour and evidence demands |

**Working principle: preserve optionality for tier 3 even when only implementing tier 1.** The architecture should not foreclose ISO 16363 certification at later dates, even where current decisions defer the supporting evidence/automation. Specifically, where a higher-tier requirement implies a technical capability we cannot implement immediately, the architecture must remain *open* to adding that capability as an additive layer.

## Working policy (per decision 27)

Every design choice in `dao-discovery.md` is aligned with OAIS, PREMIS, ISO 16363 (or nestor where the German formulation is sharper), and ISO 16919 where relevant, **or** carries a documented deviation with reason. Deviations are recorded inline in the relevant section of the discovery doc and surface in the decision-log row for that decision.

**ISO 16363** is the substantive standard the Archive must align with for tier-3 trustworthy-repository certification. **nestor Kriterienkatalog** is its closely-aligned German precursor; the criteria correspond and the German formulation is sometimes sharper. **ISO 16919** is the auditor-facing companion to ISO 16363: it specifies what an audit body must do when certifying a repository. We cite ISO 16363 to defend design choices; we cite nestor for German-language clarity or where the nestor formulation tightens an ISO requirement; we cite ISO 16919 when reasoning about what an auditor will require us to produce as evidence.

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
