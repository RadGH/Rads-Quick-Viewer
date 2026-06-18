$ErrorActionPreference = "Stop"

$vsDevCmd = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
$nodeDir = "C:\tmp\rads-toolchain\node-v22.16.0-win-x64"
$cargoDir = "$env:USERPROFILE\.cargo\bin"
$blockedPath = "C:\laragon\bin\git\usr\bin"
$env:CARGO_TARGET_DIR = "C:\tmp\rads-quick-viewer-target"

if (!(Test-Path $vsDevCmd)) {
  throw "Visual Studio Build Tools developer command was not found at $vsDevCmd"
}

cmd /c "call `"$vsDevCmd`" -arch=x64 > nul && set" | ForEach-Object {
  if ($_ -match "^(.*?)=(.*)$") {
    Set-Item -Path "Env:\$($matches[1])" -Value $matches[2]
  }
}

$cleanPath = ($env:Path -split ";" | Where-Object { $_ -and $_ -ne $blockedPath }) -join ";"
$env:Path = "$nodeDir;$cargoDir;$cleanPath"

Write-Host "Using linker:"
Get-Command link

npm run build
