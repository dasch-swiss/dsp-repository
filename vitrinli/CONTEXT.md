# Vitrinli

The media engine of the platform: a library that renders images through the IIIF Image API, serves any file's bytes with range support, and derives Service Files from source files. Vitrinli is sipi under its new name (Swiss German, "little showcase"); sipi is maintained separately today and is being rewritten from C++ to Rust. When it moves into this monorepo it lives at `vitrinli/` as a root peer of the areas (ADR-0002), depended on by a `media` capability in the Deposit Area and one in the Access Area (ADR-0003). It is neither a service nor a capability: it has no routes, no tables and no authentication of its own, and everything area-specific reaches it through the traits its interface accepts. This file is the seed that records, ahead of the code, what the `media` capabilities may rely on; the code brings its own vocabulary with it. The file vocabulary (Preservation File, Service File, Original) is defined in the root [`CONTEXT.md`](../CONTEXT.md) `## Shared` and only used here. Vitrinli serves Service Files — whole, as a download, or rendered as an IIIF image; what leaves the server for one request is a response, not a further kind of file.

## Language

**Media delivery**:
Vitrinli's first role: answering an IIIF Image API request (a region, a size, a rotation, a quality and a format) with a rendered image.
_Avoid_: IIIF server (one role, not the engine), image server.

**Bitstream delivery**:
Vitrinli's second role: serving a file's bytes on request, with HTTP range support, whatever the format.
_Avoid_: asset server, download service (the earlier design had a separate one; Vitrinli absorbed it so the read side has one store fewer).

**Derivation**:
Vitrinli's third role: producing a Service File from a source under a derivation rule (a pyramidal TIFF for IIIF, for example). Who calls it and from what source is the caller's decision: the Deposit Area's `media` derives from Originals; the Access Area's `media` never derives, because the archive already did.

**Media capability**:
The per-area capability (`areas/deposit/media`, `areas/access/media`) that depends on Vitrinli and gives it the area's rules: it owns the tables that say what is servable and where, the routes, the authorisation, and the implementations of Vitrinli's traits over the area's own store. The typed successor of the Lua scripts and configuration that shape sipi today.
_Avoid_: glue, plugin, sipi config, Vitrinli mounting (a retired framing in which Vitrinli itself was the capability).

**Byte source**:
The trait through which Vitrinli reads the bytes it serves or derives from; a `media` capability implements it over the Originals it holds (Deposit Area) or the Access bucket (Access Area).

**Authorisation check**:
The trait through which Vitrinli asks whether a request may see a file; implemented by the `media` capability against its area's session and rights model. Vitrinli holds no opinion of its own.

**Derivation sink**:
The trait through which Vitrinli hands a derived Service File back; implemented by the Deposit Area's `media`, which stores it and records it as servable.

**Access bucket**:
The object-storage location the archive publishes Service Files to; the Access Area's `media` reads from it and from nowhere else.

## Relationships

- One **Vitrinli** library, two dependants: the Deposit Area's and the Access Area's **Media capability**; each implements the **Byte source**, **Authorisation check** and, in the Deposit Area, the **Derivation sink**.
- The Deposit Area's **Media capability** holds **Originals**, has Vitrinli derive zero or one **Service File** from each, and serves either, whole or rendered.
- The Access Area's **Media capability** serves archive-made **Service Files** from the **Access bucket** only, whole or rendered.
- Neither path ever reads a **Preservation File**.
- Vitrinli depends on nothing in any area; the arrow is `media → vitrinli`.

## Boundary commitments

- Vitrinli depends on no area crate and knows no area's session, rights or storage; those arrive through its traits (ADR-0002, ADR-0003). Target enforcement: structure, via Bazel visibility.
- **Byte source** and **Authorisation check** have one implementation per area's `media`, so those seams are real. **Derivation sink** has the Deposit Area's alone and stays a hypothetical seam until a second deriving caller exists; it is kept as a trait so that derivation never reaches into an area's store directly.
- The Access Area's `media` reads only the Access bucket; the sealed store, Preservation Files and Originals are out of reach.
- The Deposit Area's `media` writes nothing into the archive; archiving is the Deposit Area's submission through the intent protocol, and a derived Service File never enters the archive.
- Staleness is legitimate on the access side: while the archive is unavailable, `media` serves what its tables already know.

## Example dialogue

> **Dev:** "The editor needs a preview of an uploaded image. Does the Deposit Area call the Access Area's Vitrinli?"
> **Domain expert:** "There is no such thing. Vitrinli is a library. The Deposit Area's **Media capability** holds the **Original**, asks Vitrinli to derive a **Service File** through its **Derivation sink**, and serves the preview behind the editor's own login through its own routes. The Access Area's `media` never sees an Original."

> **Dev:** "A consumer downloads a 4 GB video from DPE. Which file is that?"
> **Domain expert:** "The archive-made **Service File**, streamed with range support from the **Access bucket** by the Access Area's **Media capability**, which passed the request through its **Authorisation check** first. The **Preservation File** stays in the sealed store; user read paths never touch it."

## Flagged ambiguities

- **"sipi" vs "Vitrinli"**: the same code; "sipi" names the current, separately maintained codebase, "Vitrinli" the library and its home in this monorepo. Resolution: Vitrinli in all new prose; sipi only when pointing at the existing codebase.
- **"Vitrinli does X for the editor"**: Vitrinli does nothing for an area; the area's **Media capability** does, using Vitrinli. Resolution: name the `media` capability when the actor matters.
- **"asset"**: the VRE's word for a media file with its derivatives, an avoided alias for Representation in the archive, and this repository's name for static web files (`public/assets/`). Resolution: not used here; say Original, Service File, or "public assets".
- **"IIIF server"**: names one of Vitrinli's roles. Resolution: say Vitrinli, or **Media delivery** for the role.
