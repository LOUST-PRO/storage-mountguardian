# License

`storage-mountguardian` is dual-licensed under the Apache License,
Version 2.0. The companion `LICENSE-FORK.md` documents the
hardening changes made by this fork relative to upstream.

## Apache-2.0

The full Apache License, Version 2.0 text is available at:

- [LICENSE](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/LICENSE)

In summary, you may use, modify, and distribute this software (in
source or binary form) for any purpose, commercial or non-commercial,
provided you preserve the copyright notice and the licence terms.
See the full text for the disclaimer of warranty and limitation of
liability clauses.

## Hardening Addendum (this fork)

The fork ships a hardened systemd unit plus several
defensive-against-bug features (`MAX_LINE_BYTES`, `DEDUP_WINDOW`,
sysfs-based partition discovery). The full addendum is at:

- [LICENSE-FORK.md](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/LICENSE-FORK.md)

## Contributing

Bug reports, security disclosures, and pull requests are tracked
on GitHub:

- [CONTRIBUTING.md](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/CONTRIBUTING.md)
- [SECURITY.md](https://github.com/LOUST-PRO/storage-mountguardian/blob/master/SECURITY.md)

The maintainer reviews contributions under the
**[Apache-2.0 Contributor Agreement](https://www.apache.org/licenses/icla-FAQ.html)**:
by submitting a pull request, you agree to license your contribution
under Apache-2.0 alongside the rest of the project.

## Trademarks

"Storage Mount Guardian" and the `storage-mountguardian` crate name
are not registered trademarks. Any product names mentioned in the
documentation belong to their respective owners and are referenced
for compatibility purposes only.
