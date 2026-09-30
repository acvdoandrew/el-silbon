# Package a Windows test build: the executable, the assets the game loads at
# run time (not the Blender sources, review renders or the whistle's source
# recording) and the player notes, zipped. The Windows twin of package.sh,
# which needs `zip` (Git Bash on Windows has none).
#
#   powershell -File tools\package.ps1 0.1.0-test.4            build, then package
#   powershell -File tools\package.ps1 0.1.0-test.4 -NoBuild   package target\release as it is
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [switch]$NoBuild
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $NoBuild) {
    Push-Location $root
    try {
        cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    } finally { Pop-Location }
}
$exe = Join-Path $root 'target\release\el_silbon.exe'
if (-not (Test-Path $exe)) { throw "no ${exe}: build first or drop -NoBuild" }
$name = "el_silbon-$Version-windows"
$dist = Join-Path $root 'target\dist'
$out = Join-Path $dist $name
$zip = "$out.zip"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
if (Test-Path $zip) { Remove-Item -Force $zip }
New-Item -ItemType Directory -Force (Join-Path $out 'assets\models'), (Join-Path $out 'assets\branding') | Out-Null
Copy-Item $exe $out
foreach ($d in 'audio', 'fonts', 'shaders', 'ui') {
    Copy-Item -Recurse (Join-Path $root "assets\$d") (Join-Path $out "assets\$d")
}
Remove-Item -Recurse -Force (Join-Path $out 'assets\audio\source') -ErrorAction SilentlyContinue
Copy-Item (Join-Path $root 'assets\models\*.glb') (Join-Path $out 'assets\models')
# The launch splash's cover; the icons are built into the executable.
Copy-Item (Join-Path $root 'assets\branding\whistle-cover.png') (Join-Path $out 'assets\branding')
Copy-Item (Join-Path $root 'docs\PLAYING.txt') $out
# Git LFS pointers instead of models mean the checkout lacks the real files.
$head = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes((Join-Path $out 'assets\models\silbon.glb'))[0..39])
if ($head -match 'git-lfs') { throw "assets/models are Git LFS pointers: run git lfs pull first" }
# Windows 10 and later ship bsdtar, which writes zips.
Push-Location $dist
try {
    tar -a -c -f "$name.zip" $name
    if ($LASTEXITCODE -ne 0) { throw "tar failed" }
} finally { Pop-Location }
"{0} ({1:N1} MB)" -f $zip, ((Get-Item $zip).Length / 1MB)
