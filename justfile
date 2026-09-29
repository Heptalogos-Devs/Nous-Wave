set windows-shell := ["powershell", "-NoProfile", "-Command"]

default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clean-build:
    cargo clean

clean-test-temp:
    powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1

check:
    cargo check --workspace --all-targets --all-features

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features -- --test-threads=1

nextest:
    cargo nextest run --workspace --all-features --test-threads 1

feature-check:
    cargo hack check --workspace --each-feature --no-dev-deps

lint-maintainability:
    cargo clippy --no-deps --workspace --exclude nous-protocol --lib --bins --all-features -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr -D clippy::allow_attributes_without_reason -D warnings

dupes:
    cargo dupes check

dupehound:
    dupehound scan .

typos:
    typos

osv:
    osv-scanner scan -r .

coverage:
    cargo llvm-cov --workspace --all-features --summary-only

mutants path:
    cargo mutants --file "{{path}}"

deny:
    cargo deny check

deps:
    cargo shear --deny-warnings

verify: fmt-check check lint lint-maintainability test deny deps
    @echo "Nous Wave verification passed."
