# Contributing

Thanks for your interest. Bug reports and PRs are welcome.

## Process

1. Fork the repo
2. Create a feature branch (`fix/<concern>` or `feat/<feature>`)
3. Add tests for new behavior
4. Run `cargo test --workspace` and `cargo clippy --all-targets`
5. Open a PR with a clear description + validation evidence
6. Address review feedback within 30 days

## Scope

- **Hardening** (sanitization, no telemetry, fail-closed defaults) is in scope
- **Upstream features** are accepted but should be small + reviewable
- **Large refactors** need prior discussion in an issue

## Code style

- `cargo fmt --all` before commit
- `cargo clippy --all-targets -- -D warnings` clean
- Doc comments on public APIs (`cargo doc --no-deps` clean)

## License

By contributing, you agree your contributions are licensed under the
upstream LICENSE (MIT for paperforge, Apache-2.0 for storage-mountguardian
and SnapPipe).