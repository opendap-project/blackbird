# Engineered Matter — Typography

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §9" leads here.

## 9. Typography

Type communicates precision and stays comfortable for everyday reading.

### 9.1 Roles

| Role | Family | Use |
|---|---|---|
| Interface | IBM Plex Sans (and its script companions, §9.4) | All interface text |
| Technical | IBM Plex Mono | Values, timestamps, identifiers, formats and sample rates, diagnostics, tabular data |
| Fallback | System sans-serif and monospace | Web and Desktop only, when Plex is not available |

The technical face is for values that benefit from fixed-width comparison, not for every paragraph.

### 9.2 Hierarchy

| Role | Treatment |
|---|---|
| Screen title | Clear, medium or semibold |
| Primary value | Prominent, highly legible |
| Section label | Compact and restrained |
| Body text | Comfortable regular weight |
| Technical metadata | Monospaced where it improves comparison |
| Auxiliary label | Muted but readable (`text-muted`, never `graphite.500` for small text) |

Avoid wide tracking, tiny uppercase labels and condensed futuristic faces. Uppercase never applies to scripts without case, and is never forced by a template (DS-006).

### 9.3 Legibility

Consistent line height; no long uppercase passages; a clear difference between primary and secondary text; no small text over plasma or thermal effects; the minimum type size is tested on the target display.

The type scale (tokens `font.size.*` and `font.line-height.*`; revised 2026-10-23, to be checked on the prototype panel; DS-015). It is sized by visual angle for the handheld distance class, **35 cm** (other classes scale with their distance, §18.2): character height should be at least 16 minutes of arc where legibility matters and 20–22′ is preferred (ANSI/HFS 100, as quoted in the FAA Human Factors Design Standard). The first scale (14 px body, 11 px caption) fell below 16′ beyond about 33 cm. On the T1 one pixel is 0.153 mm (§18.2); cap heights are IBM Plex Sans's (0.698 em):

| Role | Size / line height (px) | Cap height | At 35 cm | Use |
|---|---|---|---|---|
| `display` | 32 / 40 | 3.4 mm | 33.6′ | Large numbers (volume overlay, readouts) |
| `headline` | 24 / 32 | 2.6 mm | 25.2′ | Screen titles that carry the screen |
| `title` | 20 / 28 | 2.1 mm | 21.0′ | Track title, screen titles |
| `body-large` | 17 / 24 | 1.8 mm | 17.8′ | Primary list rows, reading text |
| `body` | 15 / 22 | 1.6 mm | 15.7′ | Secondary lines, sentences |
| `label` | 13 / 18 | 1.4 mm | 13.6′ | Control labels, metadata, section labels |
| `caption` | 12 / 16 | 1.3 mm | 12.6′ | Status bar, timestamps, technical values (IBM Plex Mono); the minimum |

Nothing smaller than `caption` (12 px, the watchOS minimum; one point on the original iPhone panel is about one T1 pixel). Text that must be read while walking uses `body` or larger; `label` and `caption` carry identification that is not time-critical. Line heights are set explicitly on a 2 px grid, never "normal": Plex Sans SC's 1.0 em line gap would double them. Larger text sizes scale these roles nonlinearly (§13).

### 9.4 Script coverage

The UI ships with English, Mandarin Chinese, Hindi, Spanish and Arabic (SYS-028, DS-011). IBM Plex has a companion family for each launch script, all under the SIL Open Font License 1.1, checked in the [IBM/plex repository](https://github.com/IBM/plex) on 2026-10-22:

| Script | Languages | Family | Package version | Regular OTF size |
|---|---|---|---|---|
| Latin | English, Spanish | IBM Plex Sans; IBM Plex Mono | 1.1.0; 2.5.0 | — |
| Han (Simplified) | Mandarin Chinese | IBM Plex Sans SC | 1.1.0 | 6.98 MB (hinted) |
| Han (Traditional) | not a launch locale | IBM Plex Sans TC | 1.1.1 | — |
| Devanagari | Hindi | IBM Plex Sans Devanagari | 1.1.0 | 0.18 MB |
| Arabic | Arabic | IBM Plex Sans Arabic | 1.1.0 | 0.17 MB |

Each family ships eight weights (Thin to Bold, plus Text). Checked on 2026-10-23 against the package versions above (fontTools, HarfBuzz through uharfbuzz 0.56.2, HarfRust 0.15.0):

- **Chinese coverage.** Plex Sans SC maps all 6,763 hanzi of GB 2312 and 20,971 CJK unified ideographs in all; of GB 2312's 682 symbols only `§` is missing, and IBM Plex Sans has it, so the fallback chain covers it.
- **Shaping.** HarfRust 0.15.0, the shaper the device uses, produces the same glyph sequence as HarfBuzz for all 23 test strings (Devanagari conjuncts, reph, reordered i-matra; Arabic joining, lam-alef, Arabic-Indic digits; Chinese with full-width punctuation), with no missing or dotted-circle glyphs. Glyph positions and parley's line layout are not checked yet.
- **Digits.** Plex Mono and Plex Sans have only Western digits; Plex Sans Arabic adds Arabic-Indic and extended Arabic-Indic digits, Plex Sans Devanagari adds Devanagari digits. Western digits are tabular in every family (600 units); the script digits are proportional, and none of the families has a `tnum` feature.
- **Storage.** Plex Sans SC Regular is 6.97 MB as an OTF (a Noto Sans SC subset is 8.33 MB); a GB 2312 subset of it, with ASCII, CJK punctuation and full-width forms, is 2.34 MB.

Decided on 2026-10-23 from these results:

- **CJK font:** IBM Plex Sans SC, not a Noto subset: it covers the launch locale, matches the rest of Plex, and is smaller. The image ships it complete at the weights the type scale uses (Regular and SemiBold, about 14 MB), so rare characters in library metadata never fall back; no first-boot subset.
- **Technical values** (formats, sample rates, bit depths, timestamps in diagnostics) use Western digits in IBM Plex Mono in every locale, so they stay aligned. Other numbers (the time, track numbers, counts) follow the locale's digits through the script's own Plex family.

### 9.5 Right-to-left

Arabic makes right-to-left a requirement from the first release (SYS-025). Layouts mirror: leading and trailing swap, horizontal sequences reverse, directional icons (back, forward, chevrons, progress direction) flip. Media transport controls (previous, play/pause, next) and the seek bar's direction do not mirror: they show the direction of the tape, not of reading, and physical players look the same everywhere ([Material Design, bidirectionality](https://m2.material.io/go/design-bidirectionality), checked 2026-10-23). Volume, which stands for a physical order too, does not mirror either (§15.1). Slint does not mirror layouts yet ([#2294](https://github.com/slint-ui/slint/issues/2294)), so Blackbird UI does it: its rows are `DirRow`, a one-line `FlexboxLayout` reversed right to left by the `Layout` global (the focus keeps reading order, start to end), `Label` aligns to the layout's start (or its end, with `end`), and the components flip what they draw (chevrons, a capsule's ends, a toggle's knob, a progress fill). Tokens are written in logical terms (start, end), never left and right.

### 9.6 Per-script metrics

Per-locale size and line-height tokens, never per-string overrides (DS-016). Measured on the Plex files; renders on the T1's own renderer are still to be checked:

| Script | Minimum for captions and metadata | Line height | Note |
|---|---|---|---|
| Latin | 12 px | 1.3–1.4 em in rows, 1.45 em in sentences | — |
| Han (Plex Sans SC) | 12 px | 1.4–1.5 em | Digits from the Latin face (SC's are 630 units wide, Plex Sans's 600) |
| Devanagari | 13 px | 1.5–1.6 em | Matras: ink reaches about 1.25 em on common words |
| Arabic | 13 px | 1.5–1.6 em | Vowel marks reach about 1.34 em; layout mirrors (§9.5) |

### 9.7 Long text and translation

- **Cut, not scrolled** (DS-020). End ellipsis for titles, artists and albums; middle ellipsis for file and device names; the ellipsis at the logical end in right-to-left text. No automatic marquee: scrolling only the focused row after a delay is a setting, off by default (WCAG 2.2.2; driving guidelines reject automatically scrolling text). The Now Playing title wraps to two lines before it is cut; the full text is always reachable (the More view and the accessible name).
- **Expansion budgets** against English: 300 % for buttons and tabs of up to 10 characters (or two lines), 200 % for 11–20 characters, 160–180 % for sentences (W3C). Side-by-side buttons only where the longest translation fits; otherwise they stack.
- **Formats** from CLDR (grouping, decimal separator, digits, plurals, relative time), never concatenated; sentence case everywhere.
