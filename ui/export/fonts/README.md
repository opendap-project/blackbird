# Fonts

IBM Plex, the design system's type (design system §9), vendored so that the product host compiles it in and every build draws the same glyphs. The device does not rely on system fonts for its own text.

| File | Family | Source |
|---|---|---|
| `IBMPlexSans-Variable.ttf` | IBM Plex Sans, variable (`wdth`, `wght`) | [google/fonts `ofl/ibmplexsans`](https://github.com/google/fonts/tree/main/ofl/ibmplexsans) (`IBMPlexSans[wdth,wght].ttf`, version 3.201, upstream [googlefonts/plex](https://github.com/googlefonts/plex)) |
| `IBMPlexMono-Regular.ttf`, `IBMPlexMono-Medium.ttf` | IBM Plex Mono | [google/fonts `ofl/ibmplexmono`](https://github.com/google/fonts/tree/main/ofl/ibmplexmono) (upstream [googlefonts/plex](https://github.com/googlefonts/plex)) |
| `IBMPlexSansSC-Regular.otf`, `IBMPlexSansSC-SemiBold.otf` | IBM Plex Sans SC (hinted OTF, 1.1.0) | [IBM/plex](https://github.com/IBM/plex) `packages/plex-sans-sc/fonts/complete/otf/hinted/`, commit `71f558992315a12ca1f06a2215d57962a1992162` |
| `IBMPlexSansArabic-Regular.otf`, `IBMPlexSansArabic-SemiBold.otf` | IBM Plex Sans Arabic (OTF, 1.1.0) | [IBM/plex](https://github.com/IBM/plex) `packages/plex-sans-arabic/fonts/complete/otf/`, same commit |
| `IBMPlexSansDevanagari-Regular.otf`, `IBMPlexSansDevanagari-SemiBold.otf` | IBM Plex Sans Devanagari (OTF, 1.1.0) | [IBM/plex](https://github.com/IBM/plex) `packages/plex-sans-devanagari/fonts/complete/otf/`, same commit |

- **Licence:** SIL Open Font License 1.1 ([`LICENSES/OFL-1.1.txt`](../../LICENSES/OFL-1.1.txt)), © 2017–2019 IBM Corp., with Reserved Font Name "Plex"; upstream's licence file is kept as [`OFL.txt`](OFL.txt). The files are unmodified, as downloaded on 2026-10-23 (Sans, Mono) and 2026-10-25 (the companions).
- **Script companions** (design system §9.4: the launch scripts' families, Regular and SemiBold, the weights the type scale uses), stored with Git LFS. Slint falls back only to fonts it loads at run time, never to fonts compiled in, so they are not imported by `slint/fonts.slint`: the image installs this folder at `/usr/share/fonts/blackbird` and the session points `SLINT_FONT_PATH` at it, one copy for every process that draws text. SHA-256, as downloaded on 2026-10-25:

  | File | SHA-256 |
  |---|---|
  | `IBMPlexSansSC-Regular.otf` | `fc42109cec4e0af7efd4d4d7f6de138d00e87b8b48987432808e43af33580530` |
  | `IBMPlexSansSC-SemiBold.otf` | `de742347a08883a5dc8328da496e45e7b661900204e307cfa4c07920a403b260` |
  | `IBMPlexSansArabic-Regular.otf` | `e6e883f90f8feb9bca05186b56b5425f31bb4e21bde7b2111ad981f1f54dbe39` |
  | `IBMPlexSansArabic-SemiBold.otf` | `4796e4a97485d7109c5508c6148f721e611411cdd902e23c0c1c05a85ae40dc4` |
  | `IBMPlexSansDevanagari-Regular.otf` | `ae123bae0fb95306189b4a9c240b76caffbf6adf33c294aec5080c090219c13a` |
  | `IBMPlexSansDevanagari-SemiBold.otf` | `5cea20c195e9e02370c94d246843b1e155f51404d57ed6070b5d1aadb6ed2f05` |
