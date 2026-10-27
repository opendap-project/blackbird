#!/bin/sh
# The command-line contract of the image's own commands and build scripts (command standard §5), run on the
# host without a device or the build VM: only what never reboots, never builds and never changes the host.
# Usage: tests/cli.sh   (exit 0 when every case passes; each failure is printed)
set -u
root="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
fails=0
fail() { echo "FAIL: $*" >&2; fails=$((fails + 1)); }
run() { out="$("$@" 2>"$tmp/err")"; rc=$?; err="$(cat "$tmp/err")"; }
reboot="$root/image/mkosi.extra/usr/bin/odap-reboot"

run "$reboot" --help
[ "$rc" = 0 ] && [ "$(printf '%s\n' "$out" | head -1 | cut -d: -f1)" = odap-reboot ] || fail "odap-reboot --help: exit $rc"
run "$reboot" --odw-no-such-flag
[ "$rc" = 2 ] || fail "odap-reboot unknown flag: exit $rc"
case "$err" in *"error input.invalid:"*"hint: "*"fix: odap-reboot --help"*) ;; *) fail "odap-reboot unknown flag: '$err'" ;; esac
# No such mode here (no boot-modes.d on the host): refused before anything restarts.
run "$reboot" no-such-mode
[ "$rc" = 2 ] || fail "odap-reboot unknown mode: exit $rc"
case "$err" in *"error mode.unknown:"*"fix: odap-reboot --list"*) ;; *) fail "odap-reboot unknown mode: '$err'" ;; esac
run "$reboot" ../x
[ "$rc" = 2 ] || fail "odap-reboot path as mode: exit $rc"

# Build scripts say on stderr what failed and what to run when a step fails (here: no /usr/lib/modules).
for s in "$root"/image/mkosi.profiles/*/scripts/*.chroot; do
  # shellcheck disable=SC2016  # the pattern matches the script's own $rc, unexpanded
  if ! grep -q '^trap on_exit EXIT' "$s" || ! grep -q '^  \[ "\$rc" = 0 \] || echo ".*" >&2$' "$s"; then
    fail "${s##*/}: no message on stderr"
  fi
done

[ "$fails" = 0 ] && echo "tests/cli.sh: every case passes" >&2
[ "$fails" = 0 ]
