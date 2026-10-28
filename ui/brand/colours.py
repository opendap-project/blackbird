"""The brand's colours, read from the design tokens and the backdrop data, never written here.

  token("color.white.base") -> "#F5F6F4"    a colour token's hex, aliases resolved
  LOGO                                      the four logo colours (README, Colour)
  BACKDROP                                  the graphite studio backdrop of the avatar and app icon
"""
import json
import pathlib
import tomllib

HERE = pathlib.Path(__file__).resolve().parent
TOKENS = HERE.parent / "tokens" / "opendap.tokens.json"


def _tokens():
    return json.loads(TOKENS.read_text())


def token(path, source=None):
    """The hex value of the colour token at `path` (dotted), following aliases."""
    node = source if source is not None else _tokens()
    for key in path.split("."):
        node = node[key]
    value = node["$value"]
    if isinstance(value, str) and value.startswith("{") and value.endswith("}"):
        return token(value[1:-1], source)
    return value["hex"]


def rgb(hex_value):
    """'#F5F6F4' -> (245, 246, 244)."""
    h = hex_value.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


_SOURCE = _tokens()
LOGO = {
    "light": token("color.white.base", _SOURCE),      # on graphite: the default
    "dark": token("color.graphite.950", _SOURCE),     # on light surfaces and on the orange fill
    "accent": token("color.orange.500", _SOURCE),     # the mark alone, on graphite only
    "current": "currentColor",                        # inline, taking the text colour
}
BACKDROP = tomllib.loads((HERE / "backdrop.toml").read_text())
