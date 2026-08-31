# Optional convenience task runner (https://just.systems).
# Every command below works standalone -- just is never required.

# Our runtime, ahead of whatever the ambient PATH carries. WSL appends the
# Windows PATH by default, which put a broken `just` npm shim in front of the
# real one once already. Recipes resolve tools from our own install first, so
# an inherited PATH cannot decide which binary a gate runs.
#
# Derived, never hardcoded: `home_directory()` resolves on Windows, macOS and
# Linux alike, and the separator follows the OS rather than assuming Unix.
path_sep := if os_family() == "windows" { ";" } else { ":" }
export PATH := home_directory() / ".cargo" / "bin" + path_sep + home_directory() / ".local" / "bin" + path_sep + env('PATH')

# Install the toolchain this repository needs. Idempotent.
install:
    rustup toolchain install --profile minimal 1.98.0
    rustup component add clippy rustfmt llvm-tools
    cargo binstall -y prek cargo-deny cargo-machete cargo-llvm-cov

# Wire the local hooks. Both types come from default_install_hook_types.
setup:
    prek install --install-hooks

# What differs between the baseline and the organisation. Reads only.
# Exits non-zero when it finds drift, which is what the fleet audit reads.
plan:
    cargo run --quiet -- plan

# Correct the repository settings that drifted. Refuses without approval, and
# refuses outright on anything it cannot write -- branch rules and tracked
# files are reported, never PATCHed.
apply *FLAGS:
    cargo run --quiet -- apply {{FLAGS}}

# Run the quality gates. CI runs these same commands, not equivalents.
check:
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets
    cargo machete
    cargo deny check

# Coverage is reported, not gated, until there is behaviour to cover.
# See docs/quality-bar.md: llvm-cov reports no branch data on this toolchain,
# so a floor copied from the Python bar's 90% branch would claim a guarantee
# it cannot make.
coverage:
    cargo llvm-cov --summary-only

# Format in place. `check` only verifies.
fmt:
    cargo fmt --all

# Prove the gates do not depend on the ambient PATH.
doctor:
    @echo "just    $(command -v just)"
    @echo "cargo   $(command -v cargo)"
    @echo "prek    $(command -v prek)"
    @echo "rustc   $(rustc --version)"
