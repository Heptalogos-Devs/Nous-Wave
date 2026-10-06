#!/usr/bin/env bash
# Copyright 2026 Aravine Zhu
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail
# Fixed compiler/runtime notice closure used by both Windows pack builders.
toolchain=$(realpath "$1")
output=$(realpath -m "$2")
data_root=$(realpath -m "$(dirname "${BASH_SOURCE[0]}")/../../data")
case "$output/" in "$data_root/"*) ;; *) echo "Output must be under repository data/" >&2; exit 1;; esac
revision=57b595039040eaa15bece85b7cc71d952281b269
mkdir -p "$output/licenses/mingw-runtime" "$output/licenses/llvm-runtime"
cp "$toolchain/LICENSE.TXT" "$output/licenses/llvm-runtime/LICENSE.TXT"
for source in COPYING.MinGW-w64/COPYING.MinGW-w64.txt COPYING.MinGW-w64-runtime/COPYING.MinGW-w64-runtime.txt mingw-w64-libraries/winpthreads/COPYING; do
  target=$(basename "$source")
  if test "$source" = mingw-w64-libraries/winpthreads/COPYING; then target=winpthreads-COPYING.txt; fi
  curl --fail --location --retry 2 --silent --show-error \
    "https://raw.githubusercontent.com/mingw-w64/mingw-w64/$revision/$source" \
    -o "$output/licenses/mingw-runtime/$target"
done
printf '%s\n' "https://github.com/mingw-w64/mingw-w64/tree/$revision" \
  'LLVM-MinGW 20260922 UCRT; toolchain archive SHA256 bb7bb7654b33d5aa8712acb837c963b2e0c56352560c76105270a3268c665c21' \
  > "$output/licenses/mingw-runtime/source.txt"
