# IvyBridge-RS

Projeto de engenharia para levar o **Intel Core i7-3770 / HD Graphics 4000
(Ivy Bridge, Gen7)** ao limite físico no Linux, otimizando o driver gráfico
ponto a ponto, com cada ganho medido e cada regressão barrada por testes.

O driver é o do Mesa, reduzido ao que esta GPU usa:

- **crocus**: OpenGL até 4.2
- **hasvk**: Vulkan
- **elk**: compilador de shaders

As otimizações entram em C (ajustes no código existente) ou em Rust
(componentes novos), conforme a [metodologia](docs/metodologia.md).

## Situação

| Fase | Estado |
|---|---|
| Diagnóstico de vídeo no Windows (`ivyprobe`, Rust) | ✅ concluído |
| Mesa reduzido a crocus + hasvk + elk | ✅ pronto, **ainda não compilado** |
| Máquina migrando para Linux | 🔜 em andamento |
| M1: primeiro build e medições de referência | ⏳ próximo passo |
| M2: otimizações | ⏳ |

## Por onde começar

| Documento | Para quê |
|---|---|
| [docs/linux-setup.md](docs/linux-setup.md) | Instalar dependências, compilar o Mesa, usar o driver sem tocar no do sistema |
| [docs/metodologia.md](docs/metodologia.md) | Como cada otimização é feita, medida, testada e aceita |
| [docs/otimizacoes.md](docs/otimizacoes.md) | Lista de otimizações candidatas, com hipótese e forma de medir |
| [docs/roadmap.md](docs/roadmap.md) | Fases e marcos |
| [docs/mesa-gen7.md](docs/mesa-gen7.md) | Mapa do código do Mesa para Gen7 e limites físicos da máquina |
| [docs/hardware.md](docs/hardware.md) | Máquina de referência e o que já foi medido |
| [mesa-main/IVYBRIDGE.md](mesa-main/IVYBRIDGE.md) | O que foi removido do Mesa e por quê |
| [bench/README.md](bench/README.md) | Como registrar resultados de medição |

## Início rápido (Linux)

```sh
git clone https://github.com/rodolfoaltenrath/ivybridge-rs.git
cd ivybridge-rs/mesa-main
meson setup build --prefix="$HOME/.local/mesa-ivb" --libdir=lib -Dbuildtype=release
ninja -C build install
source ../tools/linux/mesa-env.sh
glxinfo -B | grep -E "renderer|version"   # deve mostrar Mesa 27.0.0-devel
```

O passo a passo completo, com dependências por distribuição, está em
[docs/linux-setup.md](docs/linux-setup.md).

## Estrutura

```
mesa-main/        Mesa reduzido (crocus, hasvk, elk e dependências)
src/, tests/      ivyprobe: diagnóstico de EDID e modos de vídeo (Rust)
tools/
  linux/          scripts de ambiente para usar o Mesa compilado
  *.ps1, *.hex    override de EDID e teste de modos (Windows, fase anterior)
bench/            resultados de medição, um arquivo por experimento
docs/             guias, metodologia, backlog e roadmap
```

## Limites físicos (o teto que buscamos)

| Recurso | Teto |
|---|---|
| GPU | 16 EUs, 650–1150 MHz, ~294 GFLOPS FP32 |
| Memória | DDR3 compartilhada com a CPU; a H61 limita a DDR3-1333 (~21 GB/s) |
| Saída de vídeo | HDMI ≤ 225 MHz de pixel clock (~95 Hz em 1080p); o monitor aceita ≤ 122 Hz por HDMI |

Detalhes e fontes: [docs/mesa-gen7.md](docs/mesa-gen7.md) e [docs/hardware.md](docs/hardware.md).

## ivyprobe (diagnóstico de vídeo)

Ferramenta da primeira fase, somente leitura: lê o EDID, calcula timings CVT e
compara cada modo com os limites de clock. Funciona no Windows (consulta o
sistema) e em qualquer plataforma com `--edid <arquivo>`.

```sh
cargo run --release -- --no-system --edid tests/edid_samples/msd2500_target_hdmi.hex
cargo test
```

## Licenças

O código do Mesa em `mesa-main/` mantém as licenças originais (`mesa-main/licenses/`).
O restante do projeto: MIT OR Apache-2.0.
