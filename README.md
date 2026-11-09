# IvyBridge-RS

Projeto de engenharia para entender e, por fim, controlar a saída de vídeo da
**Intel HD Graphics 4000 (Ivy Bridge, Gen7)** em Rust. A pergunta que move o
projeto: até onde dá para subir a taxa de atualização numa placa H61 + HDMI,
e o que impede de ir além?

Fase atual: **v0.1, Probe**, um diagnóstico somente leitura. Ele não instala
driver, não escreve em registradores e não muda o modo de vídeo.

## O que o probe faz

- Lê o EDID do monitor ativo (registro do Windows) ou de um arquivo
- Interpreta EDID 1.3/1.4 e CTA-861: timings detalhados, range limits, HDMI VSDB, FreeSync, VICs
- Mostra o sinal real na saída (pixel clock, totais) e os modos que o driver atual oferece
- Calcula timings CVT, CVT-RB e CVT-RBv2 para cada modo candidato
- Compara cada modo com os tetos de clock: 165 MHz (HDMI 1.x/DVI), 225 MHz
  (limite do i915 para Gen7) e o limite declarado pelo monitor
- Calcula a taxa máxima atingível por resolução

## Uso

```powershell
cargo run --release                                  # diagnóstico completo do sistema
cargo run --release -- --mode 1920x1080@90           # acrescenta um modo à análise
cargo run --release -- --ceiling 200                 # acrescenta um teto (ex.: medido)
cargo run --release -- --dump-edid meu_monitor.hex   # salva o EDID ativo
cargo run --release -- --no-system --edid tests/edid_samples/msd2500_target_hdmi.hex
```

O binário se chama `ivyprobe`. `--help` lista todas as opções.

### Toolchain

Funciona com o toolchain `x86_64-pc-windows-gnu` sem nada extra instalado. O
`windows-sys` está fixado na 0.59 porque a 0.60+ usa `raw-dylib`, que nesse
toolchain exige o `dlltool.exe`. Em Linux/macOS o crate compila; a consulta ao
sistema fica desativada e o EDID entra por `--edid`.

## Estrutura

```
mesa-main/     Mesa reduzido só para Ivy Bridge: crocus (OpenGL), hasvk (Vulkan),
               compilador elk. Ver mesa-main/IVYBRIDGE.md
src/
  edid.rs      parser EDID + CTA-861 (sem dependência de plataforma)
  timings.rs   VESA CVT / CVT-RB / CVT-RBv2 e busca de taxa máxima
  limits.rs    tetos de pixel clock e suas fontes
  display.rs   consulta ao Windows (único módulo com unsafe)
  report.rs    relatório
  main.rs      CLI
tests/
  edid_samples/  EDIDs reais para teste
docs/
  hardware.md  máquina de referência e conclusões
  mesa-gen7.md mapa do Mesa para Gen7 e tetos físicos do hardware
  roadmap.md   v0.1 → v1.0
tools/         override de EDID e teste de modos (Windows)
```

## Primeiros resultados

Na máquina de referência (i7-3770, H61, monitor Target 144 Hz via HDMI), veja
[docs/hardware.md](docs/hardware.md):

- A entrada HDMI do monitor anuncia **no máximo 122 Hz**. 144 Hz não está
  disponível por HDMI nesse monitor, com qualquer GPU.
- Com o limite que o i915 aplica ao Ivy Bridge (225 MHz), 1920x1080 chega a
  **~95 Hz** (CVT-RB) ou **~99 Hz** (CVT-RBv2). Com 165 MHz, ~71–73 Hz.
- 1280x720 e 1366x768 cabem a 122 Hz.
- O driver Intel atual não oferece mais que 60 Hz em 1080p.

## Segurança

Até a v0.3 tudo é somente leitura. Quando o projeto passar a aplicar modos, cada
mudança deverá ter reversão automática por tempo.
