# Hardware de referência

Máquina onde o projeto é desenvolvido e testado. Dados coletados em 2026-11-09
com `ivyprobe` e `Get-CimInstance`.

## Sistema

| Item | Valor |
|---|---|
| CPU | Intel Core i7-3770 (Ivy Bridge, 2012) |
| GPU | Intel HD Graphics 4000 (Gen7), PCI `VEN_8086&DEV_0162&SUBSYS_21118086&REV_09` |
| Placa-mãe | TGT `-H61-T` V1.0 (chipset Intel H61) |
| Sistema | Windows 11 Pro 10.0.26200 |
| Driver | Intel 10.18.10.4252 (WDDM 1.3, último lançado em 2015) |
| Conexão | HDMI |

## Monitor

| Item | Valor |
|---|---|
| Nome EDID | `TGTATTP2BL01` (Target 24", 144 Hz) |
| Fabricante / produto | `MSD` / `0x2500`, fabricado em 2022 (semana 29) |
| EDID | 1.3 + extensão CTA-861 rev. 3 (256 bytes) |
| Amostra | [`tests/edid_samples/msd2500_target_hdmi.hex`](../tests/edid_samples/msd2500_target_hdmi.hex) |
| Range limits (entrada HDMI) | V 48–122 Hz, H 31–159 kHz, pixel clock máx 300 MHz |
| HDMI VSDB | presente, sem clock TMDS máximo declarado |
| AMD FreeSync | 48–122 Hz |
| Modo preferido | 1920x1080 @ 60 Hz, 148,50 MHz (CEA VIC 16) |
| Modo mais rápido declarado | 1920x1080 @ 120 Hz, 277,30 MHz, total 2080x1111 |

## O que o driver atual faz

- Sinal ativo: 1920x1080 @ 60,000 Hz, 148,50 MHz, total 2200x1125.
- Oferece 59/60 Hz em todas as resoluções; 75 Hz só em 1280x1024, 1024x768,
  800x600 e 640x480 (modos VESA herdados).

## Conclusões da fase 1

1. **A entrada HDMI do monitor não aceita 144 Hz.** O EDID limita a faixa
   vertical a 122 Hz. 144 Hz provavelmente só existe pela DisplayPort do
   monitor, e a placa H61 não tem saída DisplayPort.
2. **O gargalo é a fonte, não o monitor.** O monitor aceita até 300 MHz de pixel
   clock; o i915 do Linux limita o HDMI de Gen5–Gen7 a 225 MHz
   (`intel_hdmi_source_max_tmds_clock()`), e a VBT da placa pode reduzir esse
   valor.
3. **Teto teórico em 1920x1080:**

   | Teto de clock | CVT-RB | CVT-RBv2 |
   |---|---|---|
   | 165 MHz (HDMI 1.x / DVI single-link) | 71 Hz | 73 Hz |
   | 225 MHz (limite do i915 para Ivy Bridge) | 95 Hz | 99 Hz |

4. **Resoluções menores chegam ao limite do monitor:** 1280x720 e 1366x768
   cabem a 122 Hz até no teto de 165 MHz; 1600x900 vai a ~99 Hz (165 MHz) ou
   122 Hz (225 MHz).
5. O driver Intel atual não oferece nada acima de 60 Hz em 1080p, embora o
   cálculo indique que 75 Hz (174,5 MHz em CVT-RB) caberia no limite do i915.
   Isso é o que justifica a v0.2/v0.3: descobrir se o limite é do driver
   Windows, da VBT ou do circuito da placa.

## Próximas medições

- [ ] Ler a VBT (Video BIOS Table) para descobrir o `hdmi_max_tmds_clock` desta placa.
- [ ] Testar, com recuperação automática, um modo personalizado de 1080p @ 75 Hz
      (CVT-RB, 174,5 MHz), acima do teto de 165 MHz.
- [ ] Verificar se a placa usa level shifter passivo ou ativo na saída HDMI.
