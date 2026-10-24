# Contributing to Rune

Thank you for contributing to Rune! We welcome contributions, bug fixes, and feature enhancements.

## Code of Conduct & Principles

1. **Safety First:** Rune is architected for strict safety. Any PR introducing path handling must strictly avoid directory traversal attacks, symlink escapes, or unbounded allocations.
2. **Document Decisions:** If you are proposing an architectural change, framework selection, or protocol update, submit an Architecture Decision Record (ADR) under `docs/decisions/` along with your PR.
3. **No Flaky Tests:** All features must include comprehensive unit, integration, or property tests (`proptest`).

---

## Development Setup

### Prerequisites

- [Rust Toolchain](https://www.rust-lang.org/) (Rust 1.70+, 2021 edition)
- `cargo-clippy` and `rustfmt`

```bash
# Check compiler installation
rustc --version
cargo --version

# Add rustfmt and clippy components
rustup component add rustfmt clippy
```

### Workspace Layout

- `rune-core/`: Core VCS data structures (Merkle DAG, SHA-256 hashing, Zstandard codecs, object store, refs, index, merge engine, remote protocols).
- `rune-cli/`: CLI entry point, subcommand parsing with `clap`, and interactive workflows.
- `rune-web/`: Runestone Axum-based read-only browser UI, route handlers, and security middleware.

---

## Quality & Testing Guidelines

Before opening a pull request, ensure all checks pass locally:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Linter check
cargo clippy --workspace --all-targets -- -D warnings

# 3. Test suite
cargo test --workspace
```

### Writing Tests

- **Unit Tests:** Place unit tests in the same file or in a corresponding `tests/` module.
- **Security & Adversarial Tests:** When touching path resolution, decompression, or route handlers, add adversarial test cases (e.g., verifying rejection of `../`, null bytes, or oversized payloads).
- **Property Tests:** Use `proptest` when testing parsers, merge algorithms, or diff edge cases.

---

## Proposing Architecture Changes (ADRs)

If your pull request introduces or changes an architectural decision:

1. Copy the template in [`docs/decisions/README.md`](docs/decisions/README.md).
2. Create a new markdown file `docs/decisions/ADR-XXX-<kebab-case-title>.md` using the next sequential number.
3. Detail the **Context**, **Decision**, **Alternatives Considered** (with explicit rejection rationale), and **Consequences**.
4. Update the index table in `docs/decisions/README.md`.

---

## Submitting Pull Requests

1. Fork the repository and create a descriptive branch: `git checkout -b feature/my-feature` or `git checkout -b fix/issue-description`.
2. Commit your changes with clear commit messages.
3. Push to your fork and submit a Pull Request targeting the `main` branch.
4. Ensure CI tests pass.
