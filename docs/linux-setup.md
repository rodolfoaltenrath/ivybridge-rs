# Guia de instalação no Linux

Do sistema recém-instalado até o Mesa do projeto rodando na HD 4000, sem
substituir o Mesa da distribuição. Se algo der errado, basta fechar o terminal:
o sistema continua usando o driver original.

## 1. Dependências

O Mesa exige **meson ≥ 1.7**, Python 3 com `mako`, `yaml` e `packaging`, além
de libdrm, bison, flex e glslang. O jeito mais seguro é pedir ao gerenciador de
pacotes as dependências de build do próprio Mesa da distribuição.

### Fedora

```sh
sudo dnf install dnf-plugins-core
sudo dnf builddep mesa
sudo dnf install git meson ninja-build python3-mako python3-pyyaml python3-packaging \
    glx-utils vulkan-tools igt-gpu-tools glmark2 perf
```

### Debian 13 / Ubuntu 25.04 ou mais novo

Habilite os repositórios de código-fonte (`deb-src`) e então:

```sh
sudo apt build-dep mesa
sudo apt install git meson ninja-build python3-mako python3-yaml python3-packaging \
    mesa-utils vulkan-tools intel-gpu-tools glmark2 linux-perf
```

Se o `meson --version` for menor que 1.7, instale uma versão nova só para o
usuário: `pipx install meson` (ou `pip install --user meson`).

### Ferramentas que podem não estar empacotadas

| Ferramenta | Uso | Fonte |
|---|---|---|
| vkmark | benchmark Vulkan | https://github.com/vkmark/vkmark |
| piglit | testes de correção OpenGL | https://gitlab.freedesktop.org/mesa/piglit |
| VK-GL-CTS (dEQP) | testes de correção Vulkan/GL | https://github.com/KhronosGroup/VK-GL-CTS |
| deqp-runner | executa piglit/dEQP em paralelo | `cargo install deqp-runner` |

Elas só são necessárias a partir do M1 (medições de referência).

## 2. Clonar e compilar

```sh
git clone https://github.com/rodolfoaltenrath/ivybridge-rs.git
cd ivybridge-rs/mesa-main

meson setup build \
    --prefix="$HOME/.local/mesa-ivb" \
    --libdir=lib \
    -Dbuildtype=release \
    -Dplatforms=x11,wayland

ninja -C build
ninja -C build install
```

Os padrões de `meson.options` já restringem o build a `crocus` e `intel_hasvk`
e desativam o LLVM; não é preciso passar `-Dgallium-drivers` nem
`-Dvulkan-drivers`.

O prefixo `~/.local/mesa-ivb` mantém tudo fora de `/usr`: o Mesa do sistema
continua intacto.

### Se o primeiro build falhar

A limpeza do Mesa foi verificada só por análise estática (no Windows). Erros
prováveis e o que fazer:

| Sintoma | Causa provável | Ação |
|---|---|---|
| `subdir ... does not exist` | um `meson.build` ainda aponta para pasta removida | tornar o `subdir` condicional ou removê-lo; anotar em `mesa-main/IVYBRIDGE.md` |
| `include_directories ... does not exist` | idem, para diretórios de include | remover a entrada |
| dependência não encontrada (`libdrm`, `wayland-protocols`, `glslang`...) | pacote de desenvolvimento faltando | instalar o pacote `-devel`/`-dev` correspondente |
| erro de compilação com `#include` de pasta removida | código não protegido por `#ifdef` | verificar se o arquivo deveria estar no build; se sim, restaurar o header do upstream |

Pastas removidas podem ser recuperadas do upstream:
https://gitlab.freedesktop.org/mesa/mesa (versão 27.0.0-devel).

## 3. Usar o driver compilado

```sh
source ~/ivybridge-rs/tools/linux/mesa-env.sh
```

O script ajusta `LD_LIBRARY_PATH`, `LIBGL_DRIVERS_PATH`, `VK_DRIVER_FILES` e
`__EGL_VENDOR_LIBRARY_FILENAMES` **só no terminal atual**. Programas abertos
nesse terminal usam o Mesa do projeto; o resto do sistema, não.

### Confirmar que é o nosso driver

```sh
glxinfo -B | grep -E "renderer|version"
# OpenGL renderer string: Mesa Intel(R) HD Graphics 4000 (IVB GT2)
# OpenGL core profile version string: 4.2 (Core Profile) Mesa 27.0.0-devel

vulkaninfo --summary | grep -E "deviceName|driverName|driverInfo"
# driverName = Intel open-source Mesa driver ... driverInfo = Mesa 27.0.0-devel
```

Se aparecer a versão do Mesa da distribuição (por exemplo, 25.x), o ambiente
não foi carregado: confira o `source` e o caminho do prefixo.

O hasvk imprime `Ivy Bridge Vulkan support is incomplete`: é esperado, e
completar esse suporte é um dos itens do backlog.

## 4. Conferir o hardware antes de medir

```sh
# RAM: precisa estar em dual-channel; anote velocidade e latências
sudo dmidecode -t memory | grep -E "Locator|Size|Speed|Part Number"

# Frequências da GPU (o número do card pode variar: card0, card1...)
cat /sys/class/drm/card*/gt_RPn_freq_mhz /sys/class/drm/card*/gt_RP0_freq_mhz 2>/dev/null
cat /sys/class/drm/card*/gt_min_freq_mhz /sys/class/drm/card*/gt_max_freq_mhz 2>/dev/null

# Uso e frequência da GPU em tempo real
sudo intel_gpu_top
```

Registre os valores em [hardware.md](hardware.md). Os procedimentos de medição
estão em [metodologia.md](metodologia.md).

## 5. Atualizar depois de uma mudança no código

```sh
ninja -C ~/ivybridge-rs/mesa-main/build install
```

Basta o `install`: ele recompila só o que mudou.
