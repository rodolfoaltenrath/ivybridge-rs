# IvyBridge-RS - testa um modo de video com reversao automatica.
# Nao precisa de Administrador.
#
# Aplica o modo sem salvar, pergunta se a imagem esta boa e, sem confirmacao
# em $Timeout segundos (ex.: tela preta), volta ao modo salvo anterior.
# Se confirmado, salva o modo como padrao.

param(
    [int]$Width = 1920,
    [int]$Height = 1080,
    [int]$Hz = 75,
    [int]$Timeout = 15,
    [switch]$List
)
$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class Ivy {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct DEVMODE {
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string dmDeviceName;
        public short dmSpecVersion, dmDriverVersion, dmSize, dmDriverExtra;
        public int dmFields, dmPositionX, dmPositionY, dmDisplayOrientation, dmDisplayFixedOutput;
        public short dmColor, dmDuplex, dmYResolution, dmTTOption, dmCollate;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string dmFormName;
        public short dmLogPixels;
        public int dmBitsPerPel, dmPelsWidth, dmPelsHeight, dmDisplayFlags, dmDisplayFrequency;
        public int dmICMMethod, dmICMIntent, dmMediaType, dmDitherType, dmReserved1, dmReserved2;
        public int dmPanningWidth, dmPanningHeight;
    }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern bool EnumDisplaySettings(string dev, int mode, ref DEVMODE dm);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int ChangeDisplaySettingsEx(string dev, ref DEVMODE dm, IntPtr hwnd, int flags, IntPtr param);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int ChangeDisplaySettingsEx(string dev, IntPtr dm, IntPtr hwnd, int flags, IntPtr param);
    public static DEVMODE Make() { var d = new DEVMODE(); d.dmSize = (short)Marshal.SizeOf(typeof(DEVMODE)); return d; }
}
"@

$CDS_UPDATEREGISTRY = 0x1; $CDS_TEST = 0x2
$DM_BITSPERPEL = 0x40000; $DM_PELSWIDTH = 0x80000; $DM_PELSHEIGHT = 0x100000; $DM_DISPLAYFREQUENCY = 0x400000

$cur = [Ivy]::Make()
[Ivy]::EnumDisplaySettings([NullString]::Value, -1, [ref]$cur) | Out-Null
Write-Host ("Modo atual: {0}x{1} @ {2} Hz" -f $cur.dmPelsWidth, $cur.dmPelsHeight, $cur.dmDisplayFrequency)

$modes = @(); $i = 0
while ($true) {
    $dm = [Ivy]::Make()
    if (-not [Ivy]::EnumDisplaySettings([NullString]::Value, $i, [ref]$dm)) { break }
    $modes += $dm; $i++
}
if ($List) {
    $modes | Where-Object { $_.dmPelsWidth -eq $Width -and $_.dmPelsHeight -eq $Height } |
        ForEach-Object { "{0}x{1} @ {2} Hz, {3} bpp" -f $_.dmPelsWidth, $_.dmPelsHeight, $_.dmDisplayFrequency, $_.dmBitsPerPel } |
        Sort-Object -Unique
    return
}

# 74.97 Hz pode aparecer como 74 ou 75.
$target = $modes | Where-Object {
    $_.dmPelsWidth -eq $Width -and $_.dmPelsHeight -eq $Height -and $_.dmBitsPerPel -eq 32 -and
    ($_.dmDisplayFrequency -eq $Hz -or $_.dmDisplayFrequency -eq $Hz - 1)
} | Select-Object -First 1
if (-not $target) {
    throw "O driver nao oferece ${Width}x${Height} @ $Hz Hz. Rode com -List para ver os modos."
}
$target.dmFields = $DM_BITSPERPEL -bor $DM_PELSWIDTH -bor $DM_PELSHEIGHT -bor $DM_DISPLAYFREQUENCY

$rc = [Ivy]::ChangeDisplaySettingsEx([NullString]::Value, [ref]$target, [IntPtr]::Zero, $CDS_TEST, [IntPtr]::Zero)
if ($rc -ne 0) { throw "O driver recusou o modo no teste (codigo $rc)." }

Write-Host "Aplicando ${Width}x${Height} @ $($target.dmDisplayFrequency) Hz (temporario)..."
$rc = [Ivy]::ChangeDisplaySettingsEx([NullString]::Value, [ref]$target, [IntPtr]::Zero, 0, [IntPtr]::Zero)
if ($rc -ne 0) { throw "Falha ao aplicar (codigo $rc)." }

$msg = "IvyBridge-RS: ${Width}x${Height} @ $($target.dmDisplayFrequency) Hz aplicado.`n`n" +
       "A imagem esta boa (sem tremer, cortes ou pontos piscando)?`n`n" +
       "Sem resposta em $Timeout s, volta para o modo anterior."
$answer = (New-Object -ComObject WScript.Shell).Popup($msg, $Timeout, "IvyBridge-RS", 4 + 32 + 4096)

if ($answer -eq 6) {
    $rc = [Ivy]::ChangeDisplaySettingsEx([NullString]::Value, [ref]$target, [IntPtr]::Zero, $CDS_UPDATEREGISTRY, [IntPtr]::Zero)
    Write-Host "CONFIRMADO: modo salvo como padrao (codigo $rc)."
} else {
    [Ivy]::ChangeDisplaySettingsEx([NullString]::Value, [IntPtr]::Zero, [IntPtr]::Zero, 0, [IntPtr]::Zero) | Out-Null
    $why = if ($answer -eq -1) { "sem resposta" } else { "recusado" }
    Write-Host "REVERTIDO ($why): voltou ao modo salvo."
}
