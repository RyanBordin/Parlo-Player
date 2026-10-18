# Generates src-tauri/lib/mpv/mpv.lib, the import library the MSVC linker needs
# to link against libmpv-2.dll. See docs/libmpv-windows.md.
#
# Usage (from the repository root):
#   powershell -ExecutionPolicy Bypass -File scripts/make-mpv-import-lib.ps1

$ErrorActionPreference = "Stop"

$mpvDir = Join-Path $PSScriptRoot "..\src-tauri\lib\mpv" | Resolve-Path
$dll = Join-Path $mpvDir "libmpv-2.dll"
$def = Join-Path $mpvDir "mpv.def"
$lib = Join-Path $mpvDir "mpv.lib"

if (-not (Test-Path $dll)) {
    throw "libmpv-2.dll not found in $mpvDir. See docs/libmpv-windows.md."
}

# Find the MSVC tools (lib.exe, dumpbin.exe) of the newest Visual Studio / Build Tools.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) {
    throw "vswhere.exe not found. Install Visual Studio Build Tools with the C++ workload."
}
$libExe = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    -find "VC\Tools\MSVC\**\bin\Hostx64\x64\lib.exe" | Select-Object -First 1
if (-not $libExe) {
    throw "lib.exe not found. Install the 'Desktop development with C++' workload."
}
$dumpbin = Join-Path (Split-Path $libExe) "dumpbin.exe"

# Without mpv.def in the package, build one from the functions the DLL exports.
if (-not (Test-Path $def)) {
    Write-Host "mpv.def not found; generating it from the DLL exports..."
    $exports = & $dumpbin /exports $dll |
        Where-Object { $_ -match '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]{8}\s+(\S+)' } |
        ForEach-Object { $Matches[1] }
    if (-not $exports) {
        throw "No exports found in $dll."
    }
    @("EXPORTS") + $exports | Set-Content -Path $def -Encoding ascii
    Write-Host "  $($exports.Count) functions exported."
}

& $libExe /nologo /def:$def /name:libmpv-2.dll /out:$lib /machine:x64
if ($LASTEXITCODE -ne 0) {
    throw "lib.exe failed with exit code $LASTEXITCODE."
}
Write-Host "Created $lib"
