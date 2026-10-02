set windows-shell := ["powershell", "-NoProfile", "-Command"]
export POSTGRESQL_VERSION := "18.6.0"

default:
    @just --list

fmt:
    cargo fmt --all
    corepack pnpm format

check-fast:
    cargo fmt --all -- --check
    corepack pnpm check:fast
    cargo clippy --workspace --all-targets --all-features -- -D warnings

check: check-fast
    corepack pnpm test
    cargo test --workspace --all-features -- --test-threads=1

audit:
    cargo deny check
    cargo shear --deny-warnings
    corepack pnpm audit
    cargo dupes check
    osv-scanner scan source -r .

dev-prepare:
    corepack pnpm dev:prepare

smoke:
    cargo build -p nous-kernel
    corepack pnpm smoke

research *args:
    corepack pnpm research:retrieval-live {{args}}

release-prepare:
    powershell -NoProfile -File scripts/build-windows-kernel.ps1
    corepack pnpm release:notices

release:
    corepack pnpm assemble:portable

release-verify:
    corepack pnpm release:verify --bundle dist/portable/windows-x64/current.zip
    corepack pnpm release:verify --bundle dist/portable/windows-x64/current.zip --layout colocated
    corepack pnpm release:verify --bundle dist/portable/windows-x64/current.zip --layout locator --relocate

clean-test-temp:
    powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1
