# IvyBridge-RS - instala um EDID override para o monitor (mesmo mecanismo do CRU).
# Requer PowerShell como Administrador. Desfazer: remove-edid-override.ps1
#
# O override acrescenta modos 1920x1080 @ 75 Hz e @ 70 Hz (CVT-RB) ao EDID real.
# Os modos originais continuam la, e o preferido segue sendo 1080p60, entao o
# Windows inicia em 60 Hz mesmo com o override instalado.

param(
    [string]$HardwareId = "MSD2500",
    [string]$EdidFile = (Join-Path $PSScriptRoot "edid_override_75hz.hex")
)
$ErrorActionPreference = "Stop"

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw "Rode este script num PowerShell aberto como Administrador." }

$hex = (Get-Content $EdidFile -Raw) -replace '\s', ''
$bytes = [byte[]] -split ($hex -replace '..', '0x$& ')
if ($bytes.Length % 128 -ne 0) { throw "EDID com tamanho invalido: $($bytes.Length)" }
for ($b = 0; $b -lt $bytes.Length / 128; $b++) {
    $sum = 0; for ($i = 0; $i -lt 128; $i++) { $sum += $bytes[$b * 128 + $i] }
    if ($sum % 256 -ne 0) { throw "Checksum invalido no bloco $b" }
}

$base = "HKLM:\SYSTEM\CurrentControlSet\Enum\DISPLAY\$HardwareId"
$instances = Get-ChildItem $base
if (-not $instances) { throw "Monitor $HardwareId nao encontrado no registro." }

$backupDir = Join-Path $PSScriptRoot "backup"
New-Item -ItemType Directory -Force $backupDir | Out-Null

foreach ($inst in $instances) {
    $params = Join-Path $inst.PSPath "Device Parameters"
    if (-not (Test-Path $params)) { continue }
    $regPath = "HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY\$HardwareId\$($inst.PSChildName)\Device Parameters"
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $safe = $inst.PSChildName -replace '[^\w]', '_'
    reg export $regPath (Join-Path $backupDir "$safe-$stamp.reg") /y | Out-Null

    $ov = Join-Path $params "EDID_OVERRIDE"
    if (Test-Path $ov) { Remove-Item $ov -Recurse -Force }
    New-Item -Path $ov -Force | Out-Null
    for ($b = 0; $b -lt $bytes.Length / 128; $b++) {
        $block = [byte[]]$bytes[($b * 128)..($b * 128 + 127)]
        New-ItemProperty -Path $ov -Name "$b" -PropertyType Binary -Value $block -Force | Out-Null
    }
    Write-Host "Override gravado em $($inst.PSChildName)"
}

$gpu = Get-PnpDevice -Class Display | Where-Object { $_.FriendlyName -like "*HD Graphics 4000*" } | Select-Object -First 1
if (-not $gpu) { throw "GPU Intel HD Graphics 4000 nao encontrada. Reinicie o PC para aplicar." }
Write-Host "Reiniciando o driver de video (a tela vai piscar)..."
pnputil /restart-device "$($gpu.InstanceId)" | Out-Null
Start-Sleep -Seconds 5
Write-Host "Pronto. Se 1920x1080 @ 75 Hz nao aparecer na lista de modos, reinicie o PC."
