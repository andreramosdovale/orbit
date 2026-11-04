<#
.SYNOPSIS
  Sobe o LiveKit e o sun juntos, com as chaves geradas pelo setup-livekit.ps1 (ADR 0014).

.DESCRIPTION
  O sun fica em primeiro plano: Ctrl+C encerra os dois.

.PARAMETER SunExe
  Executável do sun. Padrão: target\release\sun.exe na raiz do repositório.

.PARAMETER InstallDir
  Onde o LiveKit foi instalado. Padrão: a pasta livekit/ na raiz do repositório.
#>
[CmdletBinding()]
param(
  [string]$SunExe,
  [string]$InstallDir
)

$ErrorActionPreference = "Stop"

# Aqui, e não no param(): no PowerShell 5.1, $PSScriptRoot chega vazio nos valores padrão.
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $SunExe) { $SunExe = Join-Path $repoRoot "target\release\sun.exe" }
if (-not $InstallDir) { $InstallDir = Join-Path $repoRoot "livekit" }

$livekit = Join-Path $InstallDir "livekit-server.exe"
$config = Join-Path $InstallDir "livekit.yaml"
$envFile = Join-Path $InstallDir "sun.env"

foreach ($file in @($livekit, $config, $envFile)) {
  if (-not (Test-Path $file)) {
    throw "Não encontrei $file. Rode antes: powershell -ExecutionPolicy Bypass -File scripts\setup-livekit.ps1"
  }
}
if (-not (Test-Path $SunExe)) {
  throw "Não encontrei $SunExe. Compile antes: cargo build -p sun --release"
}

foreach ($line in Get-Content $envFile) {
  $name, $value = $line -split "=", 2
  if ($value) { [Environment]::SetEnvironmentVariable($name, $value, "Process") }
}

$server = Start-Process -FilePath $livekit -ArgumentList "--config", "`"$config`"" -NoNewWindow -PassThru
try {
  Start-Sleep -Seconds 1
  if ($server.HasExited) { throw "O LiveKit encerrou logo ao iniciar. Veja as mensagens acima." }
  & $SunExe
}
finally {
  if (-not $server.HasExited) { Stop-Process -Id $server.Id -Force }
}
