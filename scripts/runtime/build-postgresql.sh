#!/usr/bin/env bash
# Copyright 2026 Aravine Zhu
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail
# Explicit Windows x64 release build from official PostgreSQL source.
# Arguments: source archive, verified LLVM-MinGW directory, output root, build root.
source_archive=$(realpath "$1")
toolchain=$(realpath "$2")
output=$(realpath -m "$3")
build=$(realpath -m "$4")
patch_file=$(realpath "$(dirname "${BASH_SOURCE[0]}")/postgresql-llvm-setjmp.patch")
background_patch=$(realpath "$(dirname "${BASH_SOURCE[0]}")/postgresql-background-processes.patch")
notices_script=$(realpath "$(dirname "${BASH_SOURCE[0]}")/build-notices.sh")
test "$(sha256sum "$source_archive" | cut -d' ' -f1)" = 555610c24d53e4316da5b7d3fc25c279d96856d5e0e23ee308c328c5fa881d9f
mkdir -p "$build" "$output/licenses/postgresql"
bash "$notices_script" "$toolchain" "$output"
data_root=$(realpath -m "$(dirname "${BASH_SOURCE[0]}")/../../data")
case "$output/" in "$data_root/"*) ;; *) echo "Output must be under repository data/" >&2; exit 1;; esac
case "$build/" in "$data_root/"*) ;; *) echo "Build root must be under repository data/" >&2; exit 1;; esac

mkdir -p "$build" "$output/licenses/postgresql"
bash "$notices_script" "$toolchain" "$output"
tar -xf "$source_archive" -C "$build"
export PATH="$toolchain/bin:/usr/bin:/bin"
export LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=1789709400
export CC=x86_64-w64-mingw32-clang AR=x86_64-w64-mingw32-ar RANLIB=x86_64-w64-mingw32-ranlib
export CFLAGS='-O2 -g0 -ffile-prefix-map=.=./' LDFLAGS='-static'
export ZIC=/usr/sbin/zic
cd "$build/postgresql-18.6"
patch -p1 < "$patch_file" > "$build/patch.log"
patch -p1 < "$background_patch" >> "$build/patch.log"
flags=(--host=x86_64-w64-mingw32 --build=x86_64-pc-linux-gnu --prefix=/postgresql
  --without-readline --without-zlib --without-icu --disable-nls --without-lz4 --without-zstd)
printf '%s\n' "${flags[@]}" "CFLAGS=$CFLAGS" "LDFLAGS=$LDFLAGS" > "$output/licenses/postgresql/build-config.txt"
./configure "${flags[@]}" > "$build/configure.log" 2>&1
make -j8 > "$build/build.log" 2>&1
make install DESTDIR="$build/install-root" > "$build/install.log" 2>&1
installed="$build/install-root/postgresql"
mkdir -p "$output/bin" "$output/lib"
for name in postgres initdb pg_ctl pg_isready; do cp "$installed/bin/$name.exe" "$output/bin/"; done
cp "$installed/bin/libpq.dll" "$output/bin/"
for file in "$installed/lib/"*.dll; do
  case "$(basename "$file")" in libecpg*|libpgtypes.dll|libpq.dll) continue;; esac
  cp "$file" "$output/lib/"
done
cp -a "$installed/share" "$output/"
cp COPYRIGHT "$output/licenses/postgresql/"
cp "$patch_file" "$output/licenses/postgresql/"
cp "$background_patch" "$output/licenses/postgresql/"
cp "$toolchain/LICENSE.TXT" "$output/licenses/postgresql/llvm-runtime-license.txt"
cp "$source_archive" "$output/licenses/postgresql/postgresql-18.6.tar.bz2"
printf '%s\n' 'https://ftp.postgresql.org/pub/source/v18.6/postgresql-18.6.tar.bz2' > "$output/licenses/postgresql/source-url.txt"
printf '%s\n' '555610c24d53e4316da5b7d3fc25c279d96856d5e0e23ee308c328c5fa881d9f' > "$output/licenses/postgresql/source.sha256"
printf '%s\n' 'Retained LLVM/x64 build patch: builtin setjmp buffer uses five pointer-sized slots with unchanged layout; event DLL exports use undecorated x64 names. Restricted-token child processes run without visible console windows. Private loopback server; optional TLS/ICU/compression integrations disabled.' > "$output/licenses/postgresql/changes.txt"
