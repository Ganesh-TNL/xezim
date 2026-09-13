#!/usr/bin/env bash
# Profile-guided release build: instrument -> train -> rebuild with the
# profile. Measured on a large SoC CoreMark run: simulation loop -15%,
# host instructions -11.7%, cycles -6.1%, output bit-exact.
#
#   usage: scripts/build-pgo.sh <training-command...>
#
# The training command is run once against the instrumented binary and
# should resemble the workload that matters (a full benchmark iteration;
# an unrepresentative trainer can DEOPTIMIZE the paths you care about).
# Requirements: rustup component llvm-tools (for the toolchain-matched
# llvm-profdata — the system one usually version-mismatches and the merge
# fails). The profile degrades gracefully as sources change; rebuild it
# when perf numbers matter.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/.."
[ $# -ge 1 ] || { echo "usage: $0 <training-command...>" >&2; exit 2; }
# The profile format must match the compiler: use the ACTIVE toolchain's
# llvm-profdata (`rustup component add llvm-tools-preview`), never a system
# LLVM of a different version.
SYSROOT=$(rustc --print sysroot)
PROFDATA=$(ls "$SYSROOT"/lib/rustlib/*/bin/llvm-profdata 2>/dev/null | head -1)
[ -n "$PROFDATA" ] || { echo "llvm-profdata not found in $SYSROOT; rustup component add llvm-tools-preview" >&2; exit 2; }
# Feature set of the built binary: default none (interpreter + two-state
# path); PGO_FEATURES="--features jit" for the native-backend build.
FEATURES=${PGO_FEATURES:-}
# Separate target dir so the instrumented and optimized artifacts never
# clobber an ordinary release build.
TDIR=${PGO_TARGET_DIR:-pgo-target}
PDIR=$(mktemp -d)
trap 'rm -rf "$PDIR"' EXIT
echo "== instrumented build =="
RUSTFLAGS="-Cprofile-generate=$PDIR" ./scripts/cargo-local.sh build --release --target-dir "$TDIR" $FEATURES
echo "== training: $* =="
"$@"
"$PROFDATA" merge -o "$PDIR/merged.profdata" "$PDIR"/*.profraw
echo "== optimized build =="
RUSTFLAGS="-Cprofile-use=$PDIR/merged.profdata" ./scripts/cargo-local.sh build --release --target-dir "$TDIR" $FEATURES
echo "PGO build complete: $TDIR/release/xezim"
