"""Finding the other repositories a virtual device needs, and the host-wide state and keys.

A repository is found through ODAP_<NAME>_DIR (for example ODAP_BLACKBIRD_OS_DIR), else through the
workspace's repos.tsv (the folder above this checkout that lists every repository), never by a
guessed relative path (RFC 0008). A repository merged into the blackbird root (RFC 0017) is its folder
there: in this same checkout, else under ODAP_BLACKBIRD_DIR or the root's repos.tsv row.
"""
import os
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent


# Repositories merged into a root (RFC 0017 §7): name → (root, folder).
MERGED = {"blackbird-os": ("blackbird", "os"), "blackbird-ui": ("blackbird", "ui"),
          "system": ("blackbird", "system"), "virtual-devices": ("blackbird", "emulator")}


class NotFound(Exception):
    pass


def workspace():
    for d in [REPO, *REPO.parents]:
        if (d / "repos.tsv").is_file():
            return d
    return None


def _env_dir(var):
    p = pathlib.Path(os.environ[var]).expanduser()
    if not p.is_dir():
        raise NotFound(f"{var}={p} is not a directory")
    return p


def _row(ws, name):
    for line in (ws / "repos.tsv").read_text().splitlines()[1:]:
        path, _, slug = line.partition("\t")
        if slug.strip().split("/")[-1] == name and (ws / path).is_dir():
            return ws / path
    return None


def _merged(name, ws):
    """A merged repository's folder: in the checkout that holds this one, else in its root's checkout."""
    root, folder = MERGED[name]
    top = next((d for d in [REPO, *REPO.parents] if (d / ".git").exists()), None)
    var = "ODAP_" + root.upper() + "_DIR"
    base = _env_dir(var) if os.environ.get(var) else (_row(ws, root) if ws else None)
    for cand in (top and top / folder, base and base / folder):
        if cand and cand.is_dir():
            return cand
    return None


def repo_dir(name):
    """The checkout of the GitHub repository `name` (for example "blackbird-os")."""
    var = "ODAP_" + name.upper().replace("-", "_") + "_DIR"
    if os.environ.get(var):
        return _env_dir(var)
    ws = workspace()
    found = (_row(ws, name) if ws else None) or (_merged(name, ws) if name in MERGED else None)
    if found:
        return found
    raise NotFound(f"no {name} checkout: set {var}, or run scripts/bootstrap.sh in the workspace")


def state_dir(model):
    base = pathlib.Path(os.environ.get("XDG_STATE_HOME", pathlib.Path.home() / ".local/state"))
    return base / "opendap" / "vdev" / model


def dev_key():
    """The user's SSH key for virtual devices: ODAP_DEV_KEY, else ~/.local/share/opendap/dev-ssh.
    Maintainers' images trust it (odapkit dev-key creates it); device pairing will add it to others."""
    if os.environ.get("ODAP_DEV_KEY"):
        return pathlib.Path(os.environ["ODAP_DEV_KEY"]).expanduser()
    base = pathlib.Path(os.environ.get("XDG_DATA_HOME", pathlib.Path.home() / ".local/share"))
    return base / "opendap" / "dev-ssh" / "id_ed25519"
