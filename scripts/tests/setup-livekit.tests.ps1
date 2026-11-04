# Testes do setup-livekit.ps1, sem rede nem firewall, com um zip falso.
# Rode com: powershell -ExecutionPolicy Bypass -File scripts\tests\setup-livekit.tests.ps1

$ErrorActionPreference = "Stop"

$script = Join-Path (Split-Path -Parent $PSScriptRoot) "setup-livekit.ps1"
$root = Join-Path ([System.IO.Path]::GetTempPath()) ("orbit-setup-test-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $root | Out-Null
$failures = 0

function Check([string]$Name, [bool]$Ok) {
  if ($Ok) { Write-Host "ok   $Name" } else { Write-Host "FALHOU $Name"; $script:failures++ }
}

# Roda em outro processo: um `throw` do script precisa virar código de saída.
function Run-Setup([string]$Dir, [string]$Sha, [switch]$Dev) {
  $arguments = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $script,
    "-InstallDir", $Dir, "-SkipFirewall", "-SourceZip", $zip, "-ExpectedSha256", $Sha
  )
  if ($Dev) { $arguments += "-Dev" }
  $process = Start-Process powershell -ArgumentList $arguments -Wait -PassThru -NoNewWindow `
    -RedirectStandardOutput (Join-Path $root "out.txt") -RedirectStandardError (Join-Path $root "err.txt")
  return $process.ExitCode
}

try {
  $payload = Join-Path $root "payload"
  New-Item -ItemType Directory -Path $payload | Out-Null
  Set-Content -Path (Join-Path $payload "livekit-server.exe") -Value "falso"
  $zip = Join-Path $root "livekit.zip"
  Compress-Archive -Path (Join-Path $payload "*") -DestinationPath $zip
  $sha = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash

  # Instalação normal.
  $dir = Join-Path $root "normal"
  Check "instala com o SHA-256 certo" ((Run-Setup $dir $sha) -eq 0)
  Check "copia o executável" (Test-Path (Join-Path $dir "livekit-server.exe"))
  $firstEnv = Get-Content (Join-Path $dir "sun.env") -Raw
  $yaml = Get-Content (Join-Path $dir "livekit.yaml") -Raw
  $key = ($firstEnv -split "`n" | Where-Object { $_ -like "LIVEKIT_API_KEY=*" }) -replace "LIVEKIT_API_KEY=", ""
  $secret = ($firstEnv -split "`n" | Where-Object { $_ -like "LIVEKIT_API_SECRET=*" }) -replace "LIVEKIT_API_SECRET=", ""
  Check "gera chave e segredo" ($key.Length -gt 0 -and $secret.Length -ge 64)
  Check "o yaml usa a mesma chave" ($yaml.Contains("${key}: $secret"))
  Check "restringe à faixa do Tailscale" ($yaml.Contains("100.64.0.0/10"))
  Check "desliga o IP externo" ($yaml.Contains("use_external_ip: false"))

  # Rodar de novo não troca a chave.
  Check "roda de novo" ((Run-Setup $dir $sha) -eq 0)
  Check "mantém as chaves" ((Get-Content (Join-Path $dir "sun.env") -Raw) -eq $firstEnv)
  Check "mantém o yaml" ((Get-Content (Join-Path $dir "livekit.yaml") -Raw) -eq $yaml)

  # Modo -Dev.
  $devDir = Join-Path $root "dev"
  Check "instala com -Dev" ((Run-Setup $devDir $sha -Dev) -eq 0)
  Check "-Dev não restringe a faixa" (-not (Get-Content (Join-Path $devDir "livekit.yaml") -Raw).Contains("100.64.0.0/10"))

  # SHA-256 errado.
  $badDir = Join-Path $root "errado"
  $wrong = "0" * 64
  Check "recusa o SHA-256 errado" ((Run-Setup $badDir $wrong) -ne 0)
  Check "não instala com o SHA-256 errado" (-not (Test-Path (Join-Path $badDir "livekit-server.exe")))
  Check "não gera chaves com o SHA-256 errado" (-not (Test-Path (Join-Path $badDir "sun.env")))
}
finally {
  Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
}

if ($failures -gt 0) {
  Write-Host "$failures teste(s) falharam."
  exit 1
}
Write-Host "Todos os testes passaram."
