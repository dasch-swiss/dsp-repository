# Incunabula assets

## cover.webp

Site-index teaser image for the Incunabula project: CPE's overview of its projects, at its base, shows it on
Incunabula's teaser, with `image-credit` and `eyebrow` of its `config.kdl` (`areas/access/media/data/0803/config.kdl`). The file is validated when the
project is loaded and is served with the project's other assets.

445x239, WebP. Matches the
teaser slot's `width:100%; height:6.5rem; object-fit:cover` and the dimensions of
`dsp-incubator cpe/projects/alice-in-daschland/assets/cover.webp` (the incubator's worked example).

**Source**: IIIF image
`https://iiif.stage.dasch.swiss/0803/4AdHz6QVKik-F9ed72rZlTe.jp2/full/!1600,1600/0/default.jpg`,
DSP project 0803 ("Die Bilderfolgen der Basler Frühdrucke: Spätmittelalterliche Didaxe als
Bild-Text-Lektüre"). The title page (a1r, Titelblatt recto) of *[Das] Narrenschiff (lat.) /
Stultifera navis*, Johann Bergmann de Olpe, Basel, 1497. Shelfmark: Basel UB, Sign: Ai II 22a.
Also listed as a home/site-index hero candidate in
`dsp-incubator cpe/projects/incunabula/DESIGN_ASSETS.md` §1.

**Fetch date**: 2026-09-03.

**Derivation**: crop of the ship-of-fools woodcut, then downscale to the teaser size.

```
curl -sSL -o /tmp/inc-cover-full.jpg "https://iiif.stage.dasch.swiss/0803/4AdHz6QVKik-F9ed72rZlTe.jp2/full/!1600,1600/0/default.jpg"
magick /tmp/inc-cover-full.jpg -crop 812x448+148+272 +repage -resize 445x239^ -gravity center -extent 445x239 /tmp/inc-cover.png
cwebp -q 75 /tmp/inc-cover.png -o areas/access/cpe/projects/incunabula/assets/cover.webp
```

**Rights**: the DaSCH repository page for the project,
`https://repository.dasch.swiss/dpe/projects/0803` (fetched 2026-09-03), states Access Rights
"Full Open Access", License **CC BY 4.0** (dated 2012-08-31), copyright holder the project
itself; the copy is held by Universitätsbibliothek Basel. Hence the
`image-credit "Universitätsbibliothek Basel · CC BY 4.0"` declared in `areas/access/media/data/0803/config.kdl`.

**Note — rejected DPE teaser**: the DaSCH repository page also carries its own teaser asset at
`https://repository.dasch.swiss/assets/images/0803.webp` (452x500). It was inspected and
deliberately NOT used: it depicts a handwritten French sermon dated 1533 from Lausanne, which
is not a Basel incunabulum and would misrepresent the project. Recorded here so nobody "fixes"
the cover back to it without deciding.
