# IvyBridge-RS - remove o EDID override e volta ao EDID original do monitor.
# Requer PowerShell como Administrador.

param([string]$HardwareId = "MSD2500")
$ErrorActionPreference = "Stop"

$base = "HKLM:\SYSTEM\CurrentControlSet\Enum\DISPLAY\$HardwareId"
foreach ($inst in Get-ChildItem $base) {
    $ov = Join-Path $inst.PSPath "Device Parameters\EDID_OVERRIDE"
    if (Test-Path $ov) {
        Remove-Item $ov -Recurse -Force
        Write-Host "Override removido de $($inst.PSChildName)"
    }
}

$gpu = Get-PnpDevice -Class Display | Where-Object { $_.FriendlyName -like "*HD Graphics 4000*" } | Select-Object -First 1
if ($gpu) {
    Write-Host "Reiniciando o driver de video..."
    pnputil /restart-device "$($gpu.InstanceId)" | Out-Null
} else {
    Write-Host "Reinicie o PC para concluir."
}
