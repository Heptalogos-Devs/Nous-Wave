set windows-shell := ["powershell", "-NoProfile", "-Command"]

default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

check:
    cargo check --workspace --all-targets --all-features

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features

nextest:
    cargo nextest run --workspace --all-features

feature-check:
    cargo hack check --workspace --each-feature --no-dev-deps

lint-strict:
    cargo clippy --no-deps --workspace --exclude nous-protocol --all-targets --all-features -- -W clippy::pedantic -W clippy::nursery -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr -D clippy::allow_attributes -D clippy::allow_attributes_without_reason -D warnings

dupes:
    cargo dupes check

dupehound:
    dupehound scan .

typos:
    typos

osv:
    $osv = Join-Path (go env GOPATH) "bin/osv-scanner.exe"; if (-not (Test-Path -LiteralPath $osv)) { throw "osv-scanner is not installed at $osv" }; & $osv scan -r .

coverage:
    cargo llvm-cov --workspace --all-features --summary-only

mutants:
    cargo mutants --workspace --test-tool nextest --test-workspace true

deny:
    cargo deny check

deps:
    cargo shear --deny-warnings

structure:
    python scripts/check_source_shape.py

verify: fmt-check check lint test deny deps structure
    @echo "Nous Wave verification passed."
