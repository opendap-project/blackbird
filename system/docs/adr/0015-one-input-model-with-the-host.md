# ADR 0015: One input model with the host

- **Status:** Accepted (2026-10-27), with [RFC 0020](https://github.com/opendap-project/rfcs/blob/main/text/0020-input-wire-model.md).
- **Applies to:** `opendap-bus-model` (new), `opendap-device`, `opendap-input`, the composer; the host's `opendap-model` (cli) consumes the crate.

## Context

A control's role was defined three times on the device and once on the host:

- `opendap-device` had seven roles and `opendap-input`'s semantic layer four.
- The host's `opendap-model` took any string.
- `opendap-input`'s navigation model also named its node kinds `Role`.

The events of `opendap.input/v1` were described only on the host.

## Decision

1. **`opendap-bus-model` holds the vocabulary and the wire.** It holds:
   - `Role` (`action`, `volume-up`, `volume-down`, `main`, `back`, `select`, `menu`; `Copy`; read from its name, an unknown name refused with the known ones);
   - `InputEvent` (`opendap.input/v1`);
   - the schema identifiers.

   It is MPL-2.0, as a library others build on.
2. **The device uses it.** `opendap-device` re-exports `Role` and takes its schema identifier from the crate. `opendap-input`'s semantic layer maps every role:
   - `back` to the new `back` action;
   - `select` to `activate`;
   - `menu` to none until it has a meaning (input events §3.4).

   The navigation model's kinds are `NavRole`.
3. **The vocabulary is closed and versioned.** A new kind of control adds a role to the crate (a minor version; the schema stays `v1`). A device declares the roles it has (SYS-040).

## Consequences

- One definition of what the host sends and the device accepts. A host that does not know a role refuses it before sending.
- The crate becomes `blackbird-bus-model` at the `blackbird` cut (RFC 0017).
