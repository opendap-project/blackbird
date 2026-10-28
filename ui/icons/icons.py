#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: MPL-2.0
"""OpenDAP Icons: semantic names over the Tabler Icons family (ADR 0001).

Each name in CATALOGUE gets its glyph from Tabler Icons (MIT), vendored at a pinned version in
vendor/tabler-icons/ and chosen in map.toml, used as upstream draws it. OpenDAP draws a glyph only
where Tabler has none (OWN_24), in Tabler's style.

It writes:
  ../export/icons/24/<name>.svg  interface icons (2 px stroke at 24 px)
  ../export/icons/16/<name>.svg  status-bar icons (the same glyphs with a 1.5 stroke, 1 px at 16 px,
                                 each moved onto the pixel grid: pixelgrid.py)
  icons.json         the catalogue: name, grid, category, default accessible label, style, source
  ../renders/web/icons.html  the preview at real sizes

Run: python3 icons.py                 write svg/, icons.json and the preview (no dependencies)
     python3 icons.py --check         exit 1 if any of them is not up to date
     python3 icons.py --check DIR     validate a theme's replacement icons in DIR/<grid>/<name>.svg
"""
import json
import pathlib
import re
import tomllib

import pixelgrid

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE.parent / "export" / "icons"
VENDOR = HERE / "vendor" / "tabler-icons"
# REUSE-IgnoreStart
HEADER = "<!-- SPDX-FileCopyrightText: 2026 The OpenDAP Project -->\n<!-- SPDX-License-Identifier: MPL-2.0 -->\n"
HEADER_TABLER = "<!-- SPDX-FileCopyrightText: 2020-2026 Paweł Kuna (Tabler Icons) -->\n<!-- SPDX-License-Identifier: MIT -->\n"
# REUSE-IgnoreEnd

# Tabler's own presentation attributes.
OUTLINE = 'fill="none" stroke="currentColor" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"'
FILLED = 'fill="currentColor"'

# Every 24 px icon: (name, category, default accessible label).
CATALOGUE = [
    # transport
    ("play", "transport", "Play"),
    ("pause", "transport", "Pause"),
    ("previous", "transport", "Previous"),
    ("next", "transport", "Next"),
    ("stop", "transport", "Stop"),
    ("shuffle", "transport", "Shuffle"),
    ("repeat", "transport", "Repeat"),
    ("repeat-one", "transport", "Repeat one"),
    ("back-15", "transport", "Back 15 seconds"),
    ("forward-30", "transport", "Forward 30 seconds"),
    ("eject", "transport", "Eject"),
    # library
    ("album", "library", "Album"),
    ("artist", "library", "Artist"),
    ("song", "library", "Song"),
    ("playlist", "library", "Playlist"),
    ("queue", "library", "Queue"),
    ("folder", "library", "Folder"),
    ("podcast", "library", "Podcast"),
    ("audiobook", "library", "Audiobook"),
    ("recording", "library", "Recording"),
    ("lyrics", "library", "Lyrics"),
    ("like", "library", "Like"),
    ("like-filled", "library", "Liked"),
    # sound
    ("volume", "sound", "Volume"),
    ("volume-muted", "sound", "Muted"),
    ("headphones", "sound", "Headphones"),
    ("output-jack", "sound", "Headphone output"),
    ("output-balanced", "sound", "Balanced output"),
    ("speaker", "sound", "Speaker"),
    ("signal-path", "sound", "Signal path"),
    ("dac", "sound", "DAC"),
    ("tone", "sound", "Tone"),
    ("compare", "sound", "Compare"),
    ("hearing", "sound", "Hearing"),
    ("profile", "sound", "Profile"),
    # device
    ("devices", "device", "Play on"),
    ("this-device", "device", "This T1"),
    ("bluetooth", "device", "Bluetooth"),
    ("wifi", "device", "Wi-Fi"),
    ("usb", "device", "USB"),
    ("pair", "device", "Pair"),
    ("sync", "device", "Sync"),
    ("hat", "device", "HAT"),
    ("battery", "device", "Battery"),
    ("storage", "device", "Storage"),
    ("power", "device", "Power"),
    ("restart", "device", "Restart"),
    ("lock", "device", "Lock"),
    ("brightness", "device", "Brightness"),
    ("sleep", "device", "Sleep timer"),
    ("update", "device", "Software update"),
    # action
    ("back", "action", "Back"),
    ("forward", "action", "Forward"),
    ("down", "action", "Down"),
    ("up", "action", "Up"),
    ("search", "action", "Search"),
    ("add", "action", "Add"),
    ("close", "action", "Close"),
    ("done", "action", "Done"),
    ("more", "action", "More"),
    ("reorder", "action", "Reorder"),
    ("delete", "action", "Delete"),
    ("undo", "action", "Undo"),
    # state
    ("waiting", "state", "Waiting"),
    ("error", "state", "Error"),
    ("warning", "state", "Warning"),
    ("info", "state", "Information"),
    # added with the Tabler base (2026-10-23)
    ("library", "library", "Library"),
    ("history", "library", "History"),
    ("index", "library", "A to Z"),
    ("add-to-queue", "library", "Add to queue"),
    ("meters", "sound", "Meters"),
    ("waveform", "sound", "Waveform"),
    ("computer", "device", "Computer"),
    ("phone", "device", "Phone"),
    ("nfc", "device", "NFC"),
    ("qr-code", "device", "QR code"),
    ("sync-error", "device", "Sync problem"),
    ("microsd", "device", "microSD card"),
    ("battery-charging", "device", "Charging"),
    ("battery-low", "device", "Battery low"),
    ("unlock", "device", "Unlock"),
    ("recovery", "device", "Recovery"),
    ("household", "device", "Household"),
    ("member", "device", "Member"),
    ("settings", "device", "Settings"),
    ("notifications", "device", "Notifications"),
    ("language", "device", "Language"),
    ("keyboard", "device", "Keyboard"),
    ("theme", "device", "Theme"),
    ("capability", "device", "Capability"),
    ("developer", "device", "Developer"),
    ("screenshot", "device", "Screenshot"),
    ("edit", "action", "Edit"),
    ("restore", "action", "Restore"),
    ("loading", "state", "Loading"),
]

# Status-bar icons, 16 px: (name, default accessible label). Battery levels are separate names, so the
# bar shows the level with the family's own glyphs.
STATUS = [
    ("status-microsd", "microSD card"),
    ("status-volume", "Volume"),
    ("status-bluetooth", "Bluetooth"),
    ("status-wifi", "Wi-Fi"),
    ("status-battery-0", "Battery empty"),
    ("status-battery-1", "Battery low"),
    ("status-battery-2", "Battery half"),
    ("status-battery-3", "Battery high"),
    ("status-battery-4", "Battery full"),
    ("status-charging", "Charging"),
]

# Drawn by OpenDAP, in Tabler's style, only where Tabler has no glyph: the headphone outputs, which
# a DAP must tell apart (3.5 mm and 4.4 mm balanced).
_O = OUTLINE.format(w=2)
OWN_24 = {
    "output-jack": f'<g {_O}><path d="M9 5a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v7a2 2 0 0 1 -2 2h-2a2 2 0 0 1 -2 -2z"/><path d="M9 8h6"/><path d="M12 14v7"/></g>',
    "output-balanced": f'<g {_O}><path d="M9 5a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v7a2 2 0 0 1 -2 2h-2a2 2 0 0 1 -2 -2z"/><path d="M9 7h6"/><path d="M9 10h6"/><path d="M12 14v7"/></g>',
}


def load_map():
    data = tomllib.loads((HERE / "map.toml").read_text())
    return data["base"], data["icons"], data["status"]


def tabler_paths(style, glyph):
    """The drawing elements of a vendored Tabler glyph, without its transparent bounding box."""
    text = (VENDOR / style / f"{glyph}.svg").read_text()
    return "".join(p for p in re.findall(r"<path\b[^>]*/>", text) if 'stroke="none"' not in p)


def glyphs():
    """(grid, name, category, label, body, source) for every icon, in catalogue order."""
    base, icons, status = load_map()
    names24 = {n for n, *_ in CATALOGUE}
    assert not set(icons) - names24, f"map.toml [icons] not in the catalogue: {sorted(set(icons) - names24)}"
    assert not set(status) - {n for n, _ in STATUS}, "map.toml [status] not in STATUS"
    assert not set(OWN_24) & set(icons), "a name is both mapped and drawn"
    out = []
    for name, category, label in CATALOGUE:
        if name in icons:
            style, glyph = icons[name]
            attrs = OUTLINE.format(w=2) if style == "outline" else FILLED
            out.append((24, name, category, label, f"<g {attrs}>{tabler_paths(style, glyph)}</g>",
                        f'{base["family"]}:{style}/{glyph}@{base["version"]}'))
        else:
            assert name in OWN_24, f"{name}: no glyph (map it in map.toml)"
            out.append((24, name, category, label, OWN_24[name], "opendap"))
    for name, label in STATUS:
        entry = status[name]
        glyph = status_glyph(entry)
        paths = tabler_paths("outline", glyph)
        # 1.5 units of the 24 grid are 1 px at 16 px: whole-pixel strokes in the status bar, moved
        # as a whole onto the pixel grid (pixelgrid.py), Tabler's geometry unchanged.
        dx, dy = pixelgrid.offset(re.findall(r'\bd="([^"]*)"', paths))
        move = f' transform="translate({dx:g} {dy:g})"' if (dx, dy) != (0, 0) else ""
        body = f"<g {OUTLINE.format(w=1.5)}{move}>{paths}</g>"
        source = f'{base["family"]}:outline/{glyph}@{base["version"]}'
        if isinstance(entry, dict):
            body = badged(name, entry, paths, dx, dy)
            source = f'{base["family"]}:outline/{glyph}+filled/{entry["badge"]}@{base["version"]}'
        out.append((16, name, "status", label, body, source))
    return out


def status_glyph(entry):
    """The outline glyph a [status] entry draws: the name itself, or a badged glyph's base."""
    return entry if isinstance(entry, str) else entry["glyph"]


def badged(name, entry, paths, dx, dy):
    """A status glyph with a solid Tabler badge inside it: the badge scaled about the centre, and
    cut free of the outline by `gap` grid units (a mask), so the glyph stays one colour."""
    k, gap = entry["scale"], entry["gap"]
    badge = tabler_paths("filled", entry["badge"])
    at = f"translate({dx:g} {dy:g}) translate(12 12) scale({k:g}) translate(-12 -12)"
    cut = f"{name}-cut"
    return (f'<defs><mask id="{cut}" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24">'
            f'<rect width="24" height="24" fill="white"/><g transform="{at}" fill="black" stroke="black" '
            f'stroke-width="{2 * gap / k:g}" stroke-linejoin="round">{badge}</g></mask></defs>'
            f'<g {OUTLINE.format(w=1.5)} transform="translate({dx:g} {dy:g})" mask="url(#{cut})">{paths}</g>'
            f'<g transform="{at}" {FILLED}>{badge}</g>')


def svg(grid, body, source):
    header = HEADER if source == "opendap" else HEADER_TABLER
    return f'{header}<svg xmlns="http://www.w3.org/2000/svg" width="{grid}" height="{grid}" viewBox="0 0 24 24">{body}</svg>\n'


def outputs():
    """Every generated file and its content: the SVGs, icons.json and the preview."""
    files, catalogue = {}, []
    for grid, name, category, label, body, source in glyphs():
        files[OUT / str(grid) / f"{name}.svg"] = svg(grid, body, source)
        style = "filled" if body.startswith(f"<g {FILLED}") else "outline"
        catalogue.append({"name": name, "grid": grid, "category": category, "label": {"en": label},
                          "style": style, "source": source})
    names = [c["name"] for c in catalogue]
    assert len(names) == len(set(names)), "duplicate icon names"
    files[HERE / "icons.json"] = json.dumps({"schema": "opendap.icons/v1", "icons": catalogue}, indent=2, ensure_ascii=False) + "\n"
    files[HERE.parent / "renders" / "web" / "icons.html"] = preview()
    return files, len(catalogue)


def main():
    files, count = outputs()
    for path, text in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    for stale in set(OUT.glob("*/*.svg")) - set(files):
        stale.unlink()
    print(f"{count} icons written")


def check_outputs():
    """Exit status 1 when a generated file is missing, differs or is left over."""
    import sys
    files, _ = outputs()
    stale = [p for p, text in files.items() if not p.exists() or p.read_text() != text]
    stale += sorted(set(OUT.glob("*/*.svg")) - set(files))
    if stale:
        print("out of date (run icons/icons.py): " + ", ".join(str(p.relative_to(HERE.parent)) for p in stale), file=sys.stderr)
        return 1
    return 0


def preview():
    """renders/web/icons.html: every icon at its real sizes, on the dark surface."""
    cells = []
    for grid, name, category, label, body, source in glyphs():
        sizes = (16, 20, 24, 32, 48) if grid == 24 else (16, 24, 32)
        imgs = "".join(f'<svg viewBox="0 0 24 24" width="{px}" height="{px}">{body}</svg>' for px in sizes)
        cells.append(f'<figure><div class="sz">{imgs}</div><figcaption><b>{name}</b> {label} · {category} · {source}</figcaption></figure>')
    html = f"""<!DOCTYPE html>
<!-- SPDX-FileCopyrightText: 2026 The OpenDAP Project -->
<!-- SPDX-License-Identifier: MPL-2.0 -->
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>OpenDAP Icons</title>
<style>
:root{{--bg:#111315;--surface:#191C1F;--text:#F5F6F4;--muted:#939BA3;}}
*{{box-sizing:border-box;margin:0}}
body{{background:var(--bg);color:var(--text);font:14px/20px system-ui,sans-serif;padding:32px 24px}}
h1{{font-size:24px;line-height:32px;font-weight:600}} p{{color:var(--muted);max-width:820px;margin-top:6px}}
main{{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:12px;margin-top:24px}}
figure{{background:var(--surface);border-radius:12px;padding:12px}}
.sz{{display:flex;gap:12px;align-items:flex-end;color:var(--text)}} .sz svg{{display:block}}
figcaption{{margin-top:8px;font-size:12px;line-height:16px;color:var(--muted)}} figcaption b{{color:var(--text);font-weight:600}}
</style></head><body>
<h1>OpenDAP Icons</h1>
<p>Semantic names over Tabler Icons (MIT), as upstream draws them, at 16, 20, 24, 32 and 48 px; status-bar icons at 16, 24 and 32 px with a 1 px stroke at 16. OpenDAP draws only what Tabler lacks. Generated by <code>icons/icons.py</code>: do not edit by hand.</p>
<main>{''.join(cells)}</main></body></html>
"""
    return html


def check(theme_dir):
    """Validate replacement icons: known names and grids, a 24 unit canvas, no hard-coded colours."""
    import sys
    known = {(g, n) for g, n, *_ in glyphs()}
    problems = 0
    for path in sorted(pathlib.Path(theme_dir).glob("*/*.svg")):
        grid, name = int(path.parent.name), path.stem
        text = path.read_text()
        errors = []
        if (grid, name) not in known:
            errors.append("unknown name or grid (replacements may only redraw catalogue icons)")
        if 'viewBox="0 0 24 24"' not in text:
            errors.append("viewBox must be 0 0 24 24")
        if re.search(r'(fill|stroke)="#', text):
            errors.append("use currentColor, never a fixed colour (the theme's tokens colour icons)")
        for e in errors:
            print(f"{path}: {e}", file=sys.stderr)
        problems += len(errors)
    print("ok" if not problems else f"{problems} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    import sys
    if len(sys.argv) == 3 and sys.argv[1] == "--check":
        sys.exit(check(sys.argv[2]))
    if sys.argv[1:] == ["--check"]:
        sys.exit(check_outputs())
    main()
