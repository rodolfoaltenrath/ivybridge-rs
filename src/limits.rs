//! Tetos de pixel clock conhecidos para a cadeia GPU → placa → cabo → monitor.
//!
//! Cada teto traz a fonte, para que o relatório deixe claro o que é
//! especificação, o que é escolha de driver e o que é medida.

use crate::edid::Edid;

#[derive(Debug, Clone, PartialEq)]
pub struct Ceiling {
    pub label: String,
    pub khz: u32,
    pub source: &'static str,
}

impl Ceiling {
    pub fn mhz(&self) -> f64 {
        self.khz as f64 / 1000.0
    }
}

/// HDMI 1.0–1.2 / DVI single-link: 165 MHz.
pub const SINGLE_LINK_KHZ: u32 = 165_000;

/// Limite que o i915 aplica ao HDMI de Gen5–Gen7 (Ironlake, Sandy Bridge,
/// Ivy Bridge), antes de aplicar o teto da VBT.
/// Fonte: `intel_hdmi_source_max_tmds_clock()` em
/// drivers/gpu/drm/i915/display/intel_hdmi.c.
pub const I915_GEN7_HDMI_KHZ: u32 = 225_000;

/// Tetos genéricos da fonte (GPU/placa), sempre avaliados.
pub fn source_ceilings() -> Vec<Ceiling> {
    vec![
        Ceiling {
            label: "HDMI 1.x / DVI single-link".into(),
            khz: SINGLE_LINK_KHZ,
            source: "especificação HDMI 1.0–1.2 / DVI 1.0",
        },
        Ceiling {
            label: "i915 HDMI Gen7 (Ivy Bridge)".into(),
            khz: I915_GEN7_HDMI_KHZ,
            source: "Linux i915, intel_hdmi_source_max_tmds_clock(); a VBT da placa pode reduzir",
        },
    ]
}

/// Teto declarado pelo monitor no EDID (o menor entre range limits e VSDB).
pub fn sink_ceiling(edid: &Edid) -> Option<Ceiling> {
    let range = edid
        .range_limits()
        .and_then(|r| r.max_pixel_clock_mhz)
        .map(|m| m as u32 * 1000);
    let tmds = edid.max_tmds_mhz().map(|m| m * 1000);
    let khz = match (range, tmds) {
        (Some(a), Some(b)) => a.min(b),
        (a, b) => a.or(b)?,
    };
    Some(Ceiling {
        label: "monitor (EDID)".into(),
        khz,
        source: "EDID: descritor de range limits / HDMI VSDB",
    })
}
