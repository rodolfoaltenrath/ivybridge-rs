# IvyBridge-RS: usa o Mesa compilado do projeto no terminal atual.
#
#   source tools/linux/mesa-env.sh            # prefixo padrão
#   MESA_IVB_PREFIX=/outro/lugar source tools/linux/mesa-env.sh
#
# Só afeta este terminal. Para voltar ao Mesa do sistema, abra outro terminal.

MESA_IVB_PREFIX="${MESA_IVB_PREFIX:-$HOME/.local/mesa-ivb}"

if [ ! -d "$MESA_IVB_PREFIX/lib" ]; then
    echo "mesa-env: $MESA_IVB_PREFIX/lib não existe; rode 'ninja -C build install' antes." >&2
    return 1 2>/dev/null || exit 1
fi

export LD_LIBRARY_PATH="$MESA_IVB_PREFIX/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export LIBGL_DRIVERS_PATH="$MESA_IVB_PREFIX/lib/dri"
export GBM_BACKENDS_PATH="$MESA_IVB_PREFIX/lib/gbm"

_icd="$(ls "$MESA_IVB_PREFIX"/share/vulkan/icd.d/intel_hasvk_icd.*.json 2>/dev/null | head -n1)"
[ -n "$_icd" ] && export VK_DRIVER_FILES="$_icd"

_egl="$MESA_IVB_PREFIX/share/glvnd/egl_vendor.d/50_mesa.json"
[ -f "$_egl" ] && export __EGL_VENDOR_LIBRARY_FILENAMES="$_egl"
unset _icd _egl

echo "mesa-env: usando o Mesa de $MESA_IVB_PREFIX"
