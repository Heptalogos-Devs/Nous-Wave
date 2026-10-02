param([string]$ToolchainRoot = 'data/runtime-build/llvm-mingw-windows/llvm-mingw-20260922-ucrt-x86_64')
$ErrorActionPreference = 'Stop'
$compiler = Join-Path (Resolve-Path -LiteralPath $ToolchainRoot).Path 'bin'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$cargoRoot = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$rustupRoot = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
$env:PATH = "$compiler;$env:PATH"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER = Join-Path $compiler 'x86_64-w64-mingw32-clang.exe'
$env:CC_x86_64_pc_windows_gnullvm = $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER
$env:CXX_x86_64_pc_windows_gnullvm = Join-Path $compiler 'x86_64-w64-mingw32-clang++.exe'
$env:AR_x86_64_pc_windows_gnullvm = Join-Path $compiler 'llvm-ar.exe'
$env:CXXSTDLIB_x86_64_pc_windows_gnullvm = 'c++'
$env:CXXFLAGS_x86_64_pc_windows_gnullvm = "-std=c++17 -O2 -g0 -ffile-prefix-map=$($repoRoot.Replace('\','/'))=. -ffile-prefix-map=$($cargoRoot.Replace('\','/'))=cargo"
$env:RUSTFLAGS = "--remap-path-prefix=$repoRoot=. --remap-path-prefix=$cargoRoot=cargo --remap-path-prefix=$rustupRoot=rustup -C target-feature=+crt-static -C link-arg=-static"
Push-Location -LiteralPath $repoRoot
try {
    cargo build -p nous-kernel --bin nous-kernel --release --target x86_64-pc-windows-gnullvm
    if ($LASTEXITCODE -ne 0) { throw 'Windows Kernel build failed' }
    & (Join-Path $compiler 'llvm-strip.exe') --strip-debug 'target/x86_64-pc-windows-gnullvm/release/nous-kernel.exe'
    if ($LASTEXITCODE -ne 0) { throw 'Windows Kernel strip failed' }
    foreach ($library in @('libc++.dll', 'libunwind.dll')) {
        Copy-Item -LiteralPath (Join-Path (Join-Path (Split-Path $compiler -Parent) 'x86_64-w64-mingw32/bin') $library) -Destination (Join-Path 'target/x86_64-pc-windows-gnullvm/release' $library)
        & (Join-Path $compiler 'llvm-strip.exe') --strip-debug (Join-Path 'target/x86_64-pc-windows-gnullvm/release' $library)
        if ($LASTEXITCODE -ne 0) { throw "Private runtime strip failed: $library" }
    }
} finally {
    Pop-Location
}
