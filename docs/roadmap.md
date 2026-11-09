# Roadmap

Dois objetivos separados:

- **Imediato:** descobrir se a H61 trabalha acima de 60 Hz em resoluções úteis,
  sem instabilidade nem perda de imagem.
- **Engenharia:** um driver aberto e experimental para Intel Gen7, em Rust, com
  código verificável, registradores documentados e comportamento baseado em
  especificações.

Mesmo que o HDMI tenha um limite físico, o segundo objetivo continua válido.

## v0.1 — Diagnóstico e leitura de EDID ✅

- [x] Parser EDID 1.3/1.4 + CTA-861 (DTDs, range limits, HDMI VSDB, FreeSync, VICs)
- [x] Timings VESA CVT, CVT-RB e CVT-RBv2, com testes contra valores de referência
- [x] Windows: caminho ativo, sinal real, conexão e EDID via `QueryDisplayConfig` + registro
- [x] Modos expostos pelo driver (`EnumDisplaySettingsW`)
- [x] Relatório: modos × fórmulas × tetos de clock, taxa máxima por resolução
- [x] Primeiro diagnóstico do hardware real ([hardware.md](hardware.md))

## v0.2 — Laboratório de timings

- [ ] Saída em JSON para comparar execuções
- [ ] Gerar propostas de modos (DTD binário, modeline e formato CRU) sem aplicá-los
- [ ] Gerar um EDID override com os modos propostos (aplicação continua manual)
- [ ] Restrições do HDMI: TMDS, profundidade de cor, data islands e áudio
- [ ] Ler `DISPLAYCONFIG_VIDEO_SIGNAL_INFO` de todos os modos de destino suportados

## v0.3 — Pesquisa do controlador Gen7

- [ ] Ler e interpretar a VBT (Video BIOS Table): `hdmi_max_tmds_clock`, portas, level shifter
- [ ] Mapear os registradores documentados (PRM Ivy Bridge, vol. 3): DPLL, pipes, transcoders, HDMI
- [ ] Comparar com o i915 (`intel_dpll_mgr.c`, `intel_hdmi.c`) e documentar as diferenças
- [ ] Calculadora de DPLL do PCH (Ibex Peak/Cougar Point): mostrar quais clocks são programáveis

## v0.4 — Miniport experimental

- [ ] Estudar o exemplo KMDOD da Microsoft
- [ ] Integração Rust/C com o WDK; testes em máquina separada ou com depurador de kernel
- [ ] Recuperação automática após falha de modesetting

## v1.0 — Driver funcional experimental

Modesetting físico com recuperação de falhas. Manter a aceleração gráfica
existente exigiria integrar os componentes de modo usuário da Intel, não apenas
o miniport.

## Trilha Linux: otimização do Mesa (crocus / hasvk / elk)

Objetivo: levar o OpenGL e o Vulkan da HD 4000 ao limite físico da máquina,
uma otimização medida de cada vez. Base em [`mesa-main/`](../mesa-main/IVYBRIDGE.md),
mapa em [mesa-gen7.md](mesa-gen7.md).

### M0 — Base (Windows) ✅

- [x] Mesa reduzido a crocus + hasvk + elk; build restrito a esses drivers

### M1 — Primeiro build e linha de base (Linux)

Guia: [linux-setup.md](linux-setup.md).

- [ ] Instalar dependências e rodar `meson setup` / `ninja`; corrigir o que a
      limpeza tiver quebrado
- [ ] Confirmar que o driver compilado é o carregado (`glxinfo -B`, `vulkaninfo`)
- [ ] Conferir RAM em dual-channel e a faixa de frequência da GPU; registrar em [hardware.md](hardware.md)
- [ ] Escolher o jogo/benchmark real que será usado em todas as medições
- [ ] Medição de referência (glmark2, vkmark, benchmark real) em `bench/results/<data>-baseline.md`
- [ ] Lista de referência de falhas: piglit (GL) e dEQP-VK, para detectar regressões

### M2 — Otimizações, uma por vez

Seguindo a [metodologia](metodologia.md), na ordem do [backlog](otimizacoes.md):
sistema (O1–O3), parâmetros do driver (O4–O6), compilador elk (O7–O9),
custo de CPU (O10), Vulkan (O11).

### M3 — Consolidação

- [ ] Juntar as otimizações aceitas num build "melhor conhecido" e medir o
      conjunto contra a referência do M1
- [ ] Documentar o ganho total e o que ficou no limite físico

## Trilha Windows (pausada)

As versões v0.2–v1.0 acima (laboratório de timings, miniport WDDM) foram
pausadas quando a máquina passou para Linux. O `ivyprobe` continua útil para
análise de EDID em qualquer sistema.

## Referências

- Intel, [Graphics PRM, Ivy Bridge (2012)](https://www.intel.com/content/www/us/en/docs/graphics-for-linux/developer-reference/1-0/intel-core-processor-2012.html)
- Linux, [drivers/gpu/drm/i915](https://github.com/torvalds/linux/tree/master/drivers/gpu/drm/i915)
- Microsoft, [exemplo KMDOD](https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/kernel-mode-display-only-miniport-driver-kmdod-sample/)
- VESA, Coordinated Video Timings (CVT) 1.2
