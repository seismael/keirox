# Keirox Automation & Build Scripts

Utility scripts for environment bootstrap, formatting checks, benchmark harness, and CI/CD operations.

---

## ⚡ Available Scripts

| Script | Purpose |
|---|---|
| `scripts/check.sh` | Runs `cargo fmt`, `cargo clippy`, and `cargo test` across all workspace crates. |
| `scripts/bench.sh` | Executes standard benchmark profiles P1 through P6 via `keirox-bench`. |
| `scripts/audit.sh` | Executes dependency vulnerability audit via `cargo audit`. |
