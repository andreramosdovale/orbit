<#
.SYNOPSIS
  Instala o LiveKit no PC do host e gera a configuração (ADR 0014).

.DESCRIPTION
  Baixa uma versão fixa do livekit-server, confere o SHA-256, gera a chave e o segredo
  e cria as regras de firewall. Pode rodar mais de uma vez: a chave existente é mantida.
  Só a parte do firewall precisa de administrador.

.PARAMETER Dev
  Não restringe os endereços à faixa do Tailscale, para testar sem ele.

.PARAMETER InstallDir
  Onde instalar. Padrão: a pasta livekit/ na raiz do repositório.

.PARAMETER SkipFirewall
  Não mexe no firewall.

.PARAMETER SourceZip
  Usa um zip local em vez de baixar. Junto com ExpectedSha256, serve aos testes do script.

.PARAMETER ExpectedSha256
  SHA-256 esperado do zip. Padrão: o da versão fixa.
#>
[CmdletBinding()]
param(
  [switch]$Dev,
  [string]$InstallDir,
  [switch]$SkipFirewall,
  [string]$SourceZip,
  [string]$ExpectedSha256 = "5d071ea43339f0140d24a66f0b2ff8a7bf6c4d0dceb19b30abca835670213667"
)

$ErrorActionPreference = "Stop"

# Aqui, e não no param(): no PowerShell 5.1, $PSScriptRoot chega vazio nos valores padrão.
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $InstallDir) { $InstallDir = Join-Path $repoRoot "livekit" }

$Version = "1.13.11"
$ZipName = "livekit_${Version}_windows_amd64.zip"
$DownloadUrl = "https://github.com/livekit/livekit/releases/download/v$Version/$ZipName"

$SignalPort = 7880
$RtcTcpPort = 7881
$RtcUdpPort = 7882
$SunPort = 7700
$TailscaleRange = "100.64.0.0/10"

function New-RandomHex([int]$Bytes) {
  $buffer = New-Object byte[] $Bytes
  $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
  try { $rng.GetBytes($buffer) } finally { $rng.Dispose() }
  return -join ($buffer | ForEach-Object { $_.ToString("x2") })
}

# UTF-8 sem BOM e com LF: o LiveKit lê o YAML, e o start-host.ps1 lê o .env.
function Write-TextFile([string]$Path, [string]$Content) {
  $encoding = New-Object System.Text.UTF8Encoding $false
  [System.IO.File]::WriteAllText($Path, $Content.Replace("`r`n", "`n"), $encoding)
}

function Install-Server {
  $exe = Join-Path $InstallDir "livekit-server.exe"
  $versionFile = Join-Path $InstallDir "VERSION"
  if ((Test-Path $exe) -and (Test-Path $versionFile) -and
      ((Get-Content $versionFile -Raw).Trim() -eq $Version)) {
    Write-Host "livekit-server $Version já instalado."
    return
  }

  $temp = Join-Path ([System.IO.Path]::GetTempPath()) ("orbit-livekit-" + [guid]::NewGuid())
  New-Item -ItemType Directory -Path $temp | Out-Null
  try {
    $zip = $SourceZip
    if (-not $zip) {
      $zip = Join-Path $temp $ZipName
      Write-Host "Baixando livekit-server $Version..."
      [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
      $ProgressPreference = "SilentlyContinue"
      Invoke-WebRequest -Uri $DownloadUrl -OutFile $zip -UseBasicParsing
    }

    $actual = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedSha256.ToLowerInvariant()) {
      throw "SHA-256 inesperado: o arquivo do LiveKit foi recusado e nada foi instalado. Esperado $ExpectedSha256, veio $actual."
    }

    $extracted = Join-Path $temp "extraido"
    Expand-Archive -Path $zip -DestinationPath $extracted
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Force (Join-Path $extracted "livekit-server.exe") $exe
    Write-TextFile $versionFile "$Version`n"
    Write-Host "livekit-server $Version instalado em $InstallDir."
  }
  finally {
    Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
  }
}

# Devolve a chave e o segredo, gerando só na primeira vez.
function Get-Keys {
  $envFile = Join-Path $InstallDir "sun.env"
  if (Test-Path $envFile) {
    $values = @{}
    foreach ($line in Get-Content $envFile) {
      $name, $value = $line -split "=", 2
      if ($value) { $values[$name] = $value }
    }
    if ($values["LIVEKIT_API_KEY"] -and $values["LIVEKIT_API_SECRET"]) {
      Write-Host "Chaves existentes mantidas."
      return $values["LIVEKIT_API_KEY"], $values["LIVEKIT_API_SECRET"]
    }
    throw "$envFile existe, mas está incompleto. Corrija o arquivo ou apague-o para gerar chaves novas."
  }

  $key = "API" + (New-RandomHex 8)
  $secret = New-RandomHex 32
  Write-TextFile $envFile "LIVEKIT_API_KEY=$key`nLIVEKIT_API_SECRET=$secret`nLIVEKIT_PORT=$SignalPort`n"
  Write-Host "Chaves novas geradas."
  return $key, $secret
}

function Write-Config([string]$Key, [string]$Secret) {
  $ips = if ($Dev) { "" } else { "  ips:`n    includes:`n      - $TailscaleRange`n" }
  $config = "port: $SignalPort`nrtc:`n  tcp_port: $RtcTcpPort`n  udp_port: $RtcUdpPort`n  use_external_ip: false`n$ips" +
    "keys:`n  ${Key}: $Secret`n"
  Write-TextFile (Join-Path $InstallDir "livekit.yaml") $config
}

function Set-FirewallRules {
  if ($SkipFirewall) { return }
  $identity = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
  if (-not $identity.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Warning "Sem administrador: o firewall não foi configurado. Rode este script de novo como administrador para os amigos alcançarem o servidor."
    return
  }
  $rules = @(
    @{ Name = "Orbit sun"; Protocol = "TCP"; Port = @($SunPort) },
    @{ Name = "Orbit LiveKit TCP"; Protocol = "TCP"; Port = @($SignalPort, $RtcTcpPort) },
    @{ Name = "Orbit LiveKit UDP"; Protocol = "UDP"; Port = @($RtcUdpPort) }
  )
  foreach ($rule in $rules) {
    Get-NetFirewallRule -DisplayName $rule.Name -ErrorAction SilentlyContinue | Remove-NetFirewallRule
    New-NetFirewallRule -DisplayName $rule.Name -Direction Inbound -Action Allow `
      -Protocol $rule.Protocol -LocalPort $rule.Port -RemoteAddress $TailscaleRange | Out-Null
  }
  Write-Host "Firewall configurado: só a faixa do Tailscale ($TailscaleRange) alcança as portas do Orbit."
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Install-Server
$key, $secret = Get-Keys
Write-Config $key $secret
Set-FirewallRules
Write-Host "Pronto. Para subir o servidor: powershell -ExecutionPolicy Bypass -File scripts\start-host.ps1"
