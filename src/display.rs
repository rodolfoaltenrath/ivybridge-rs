//! Consulta (somente leitura) das saídas de vídeo do sistema.
//!
//! Windows: `QueryDisplayConfig` para o caminho ativo e o sinal real,
//! `EnumDisplaySettingsW` para os modos que o driver aceita e o registro
//! (`HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY\...\Device Parameters\EDID`)
//! para o EDID bruto. Todo o `unsafe` do projeto fica neste módulo.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTechnology {
    Vga,
    Dvi,
    Hdmi,
    DisplayPort,
    Internal,
    Other(i32),
}

impl OutputTechnology {
    pub fn from_raw(v: i32) -> Self {
        match v {
            0 => OutputTechnology::Vga,
            4 => OutputTechnology::Dvi,
            5 => OutputTechnology::Hdmi,
            10 | 11 => OutputTechnology::DisplayPort,
            i32::MIN => OutputTechnology::Internal,
            n => OutputTechnology::Other(n),
        }
    }
}

impl fmt::Display for OutputTechnology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputTechnology::Vga => f.write_str("VGA (HD15)"),
            OutputTechnology::Dvi => f.write_str("DVI"),
            OutputTechnology::Hdmi => f.write_str("HDMI"),
            OutputTechnology::DisplayPort => f.write_str("DisplayPort"),
            OutputTechnology::Internal => f.write_str("interna"),
            OutputTechnology::Other(n) => write!(f, "outra ({n})"),
        }
    }
}

/// Sinal efetivamente programado na saída, segundo o Windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalInfo {
    pub pixel_rate_hz: u64,
    pub h_freq_hz: f64,
    pub v_freq_hz: f64,
    pub active: (u32, u32),
    pub total: (u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DriverMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub bpp: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveOutput {
    pub gdi_name: String,
    pub adapter_name: String,
    pub monitor_name: String,
    pub monitor_device_path: String,
    pub output_technology: Option<OutputTechnology>,
    pub refresh_hz: Option<f64>,
    pub signal: Option<SignalInfo>,
    pub edid: Option<Vec<u8>>,
    pub driver_modes: Vec<DriverMode>,
}

#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    pub outputs: Vec<ActiveOutput>,
}

#[cfg(not(windows))]
pub fn query() -> Result<SystemInfo, String> {
    Err("consulta de sistema implementada apenas para Windows nesta versão; use --edid".into())
}

#[cfg(windows)]
pub fn query() -> Result<SystemInfo, String> {
    win::query()
}

/// Converte `\\?\DISPLAY#MSD2500#4&799cdb5&0&UID50727680#{...}` na chave
/// `DISPLAY\MSD2500\4&799cdb5&0&UID50727680` do registro.
pub fn device_path_to_enum_key(path: &str) -> Option<String> {
    let mut parts = path.trim_start_matches(r"\\?\").split('#');
    let class = parts.next()?;
    let hw = parts.next()?;
    let inst = parts.next()?;
    if class.is_empty() || hw.is_empty() || inst.is_empty() {
        return None;
    }
    Some(format!(r"{class}\{hw}\{inst}"))
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::collections::BTreeSet;
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_TARGET, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    };
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplaySettingsW, DEVMODEW, DISPLAY_DEVICEW,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    fn rational(num: u32, den: u32) -> f64 {
        if den == 0 {
            0.0
        } else {
            num as f64 / den as f64
        }
    }

    pub fn query() -> Result<SystemInfo, String> {
        let mut outputs = active_paths()?;
        for out in &mut outputs {
            out.adapter_name = adapter_name(&out.gdi_name).unwrap_or_default();
            out.driver_modes = driver_modes(&out.gdi_name);
            out.edid =
                device_path_to_enum_key(&out.monitor_device_path).and_then(|key| read_edid(&key));
        }
        Ok(SystemInfo { outputs })
    }

    fn active_paths() -> Result<Vec<ActiveOutput>, String> {
        let mut n_paths = 0u32;
        let mut n_modes = 0u32;
        // SAFETY: ponteiros válidos para dois u32 locais.
        let rc = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut n_paths, &mut n_modes)
        };
        if rc != ERROR_SUCCESS {
            return Err(format!("GetDisplayConfigBufferSizes falhou ({rc})"));
        }
        // SAFETY: estruturas POD; zero é uma representação válida.
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            (0..n_paths).map(|_| unsafe { zeroed() }).collect();
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
            (0..n_modes).map(|_| unsafe { zeroed() }).collect();
        // SAFETY: os buffers têm exatamente os tamanhos informados.
        let rc = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut n_paths,
                paths.as_mut_ptr(),
                &mut n_modes,
                modes.as_mut_ptr(),
                null_mut(),
            )
        };
        if rc != ERROR_SUCCESS {
            return Err(format!("QueryDisplayConfig falhou ({rc})"));
        }
        paths.truncate(n_paths as usize);
        modes.truncate(n_modes as usize);

        let mut out = Vec::new();
        for p in &paths {
            let t = &p.targetInfo;
            let mut o = ActiveOutput {
                output_technology: Some(OutputTechnology::from_raw(t.outputTechnology)),
                refresh_hz: Some(rational(t.refreshRate.Numerator, t.refreshRate.Denominator)),
                ..Default::default()
            };

            // SAFETY: header preenchido conforme a documentação; a API só
            // escreve dentro de `size` bytes.
            unsafe {
                let mut name: DISPLAYCONFIG_TARGET_DEVICE_NAME = zeroed();
                name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                name.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                name.header.adapterId = t.adapterId;
                name.header.id = t.id;
                if DisplayConfigGetDeviceInfo(&mut name.header) == 0 {
                    o.monitor_name = from_wide(&name.monitorFriendlyDeviceName);
                    o.monitor_device_path = from_wide(&name.monitorDevicePath);
                }

                let mut src: DISPLAYCONFIG_SOURCE_DEVICE_NAME = zeroed();
                src.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                src.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                src.header.adapterId = p.sourceInfo.adapterId;
                src.header.id = p.sourceInfo.id;
                if DisplayConfigGetDeviceInfo(&mut src.header) == 0 {
                    o.gdi_name = from_wide(&src.viewGdiDeviceName);
                }

                let idx = t.Anonymous.modeInfoIdx as usize;
                if let Some(m) = modes.get(idx) {
                    if m.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_TARGET {
                        let s = m.Anonymous.targetMode.targetVideoSignalInfo;
                        o.signal = Some(SignalInfo {
                            pixel_rate_hz: s.pixelRate,
                            h_freq_hz: rational(s.hSyncFreq.Numerator, s.hSyncFreq.Denominator),
                            v_freq_hz: rational(s.vSyncFreq.Numerator, s.vSyncFreq.Denominator),
                            active: (s.activeSize.cx, s.activeSize.cy),
                            total: (s.totalSize.cx, s.totalSize.cy),
                        });
                    }
                }
            }
            out.push(o);
        }
        Ok(out)
    }

    fn adapter_name(gdi_name: &str) -> Option<String> {
        let mut i = 0;
        loop {
            // SAFETY: DISPLAY_DEVICEW é POD; cb definido antes da chamada.
            let mut dd: DISPLAY_DEVICEW = unsafe { zeroed() };
            dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
            if unsafe { EnumDisplayDevicesW(null(), i, &mut dd, 0) } == 0 {
                return None;
            }
            if from_wide(&dd.DeviceName) == gdi_name {
                return Some(from_wide(&dd.DeviceString));
            }
            i += 1;
        }
    }

    /// Modos que o driver atual expõe para a saída (sem testar nenhum).
    fn driver_modes(gdi_name: &str) -> Vec<DriverMode> {
        let name = wide(gdi_name);
        let mut set = BTreeSet::new();
        let mut i = 0u32;
        loop {
            // SAFETY: DEVMODEW é POD; dmSize definido antes da chamada.
            let mut dm: DEVMODEW = unsafe { zeroed() };
            dm.dmSize = size_of::<DEVMODEW>() as u16;
            if unsafe { EnumDisplaySettingsW(name.as_ptr(), i, &mut dm) } == 0 {
                break;
            }
            set.insert(DriverMode {
                width: dm.dmPelsWidth,
                height: dm.dmPelsHeight,
                refresh_hz: dm.dmDisplayFrequency,
                bpp: dm.dmBitsPerPel,
            });
            i += 1;
        }
        set.into_iter().collect()
    }

    fn read_edid(enum_key: &str) -> Option<Vec<u8>> {
        let path = wide(&format!(
            r"SYSTEM\CurrentControlSet\Enum\{enum_key}\Device Parameters"
        ));
        let value = wide("EDID");
        let mut key: HKEY = null_mut();
        // SAFETY: strings terminadas em zero; `key` só é usada se a abertura
        // teve sucesso e é fechada logo em seguida.
        unsafe {
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, KEY_READ, &mut key)
                != ERROR_SUCCESS
            {
                return None;
            }
            let mut len = 0u32;
            let mut ok = RegQueryValueExW(
                key,
                value.as_ptr(),
                null(),
                null_mut(),
                null_mut(),
                &mut len,
            ) == ERROR_SUCCESS;
            let mut buf = vec![0u8; len as usize];
            if ok {
                ok = RegQueryValueExW(
                    key,
                    value.as_ptr(),
                    null(),
                    null_mut(),
                    buf.as_mut_ptr(),
                    &mut len,
                ) == ERROR_SUCCESS;
                buf.truncate(len as usize);
            }
            RegCloseKey(key);
            ok.then_some(buf)
        }
    }
}
