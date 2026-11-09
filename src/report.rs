//! Relatório de diagnóstico em texto (Markdown leve, legível no terminal).

use crate::display::{ActiveOutput, SystemInfo};
use crate::edid::{CtaBlock, Edid};
use crate::limits::{self, Ceiling};
use crate::timings::{max_refresh_under, Formula, Timing, VideoMode};
use std::collections::BTreeMap;
use std::fmt::Write;

pub const DEFAULT_MODES: [VideoMode; 12] = [
    VideoMode::new(1920, 1080, 60),
    VideoMode::new(1920, 1080, 75),
    VideoMode::new(1920, 1080, 85),
    VideoMode::new(1920, 1080, 100),
    VideoMode::new(1920, 1080, 120),
    VideoMode::new(1920, 1080, 144),
    VideoMode::new(1600, 900, 75),
    VideoMode::new(1600, 900, 120),
    VideoMode::new(1600, 900, 144),
    VideoMode::new(1280, 720, 120),
    VideoMode::new(1280, 720, 144),
    VideoMode::new(1366, 768, 144),
];

pub struct Input {
    pub system: Result<SystemInfo, String>,
    pub edid: Option<Edid>,
    pub edid_source: String,
    pub edid_error: Option<String>,
    pub modes: Vec<VideoMode>,
    pub extra_ceilings: Vec<Ceiling>,
}

/// Restrições do monitor que um timing precisa respeitar.
#[derive(Debug, Clone, Copy, Default)]
pub struct SinkLimits {
    pub v_hz: Option<(f64, f64)>,
    pub h_khz: Option<(f64, f64)>,
    pub max_clock_khz: Option<u32>,
}

impl SinkLimits {
    pub fn from_edid(edid: &Edid) -> Self {
        let r = edid.range_limits();
        SinkLimits {
            v_hz: r.map(|r| (r.v_min_hz as f64, r.v_max_hz as f64)),
            h_khz: r.map(|r| (r.h_min_khz as f64, r.h_max_khz as f64)),
            max_clock_khz: limits::sink_ceiling(edid).map(|c| c.khz),
        }
    }

    /// Lista de violações (vazia = o monitor declara aceitar).
    pub fn violations(&self, t: &Timing) -> Vec<String> {
        let mut v = Vec::new();
        // Tolerância de 0,5 Hz / 0,5 kHz para arredondamentos do EDID.
        if let Some((lo, hi)) = self.v_hz {
            let r = t.refresh_hz();
            if r < lo - 0.5 || r > hi + 0.5 {
                v.push(format!("V {r:.1} Hz fora de {lo}-{hi} Hz"));
            }
        }
        if let Some((lo, hi)) = self.h_khz {
            let h = t.h_freq_khz();
            if h < lo - 0.5 || h > hi + 0.5 {
                v.push(format!("H {h:.1} kHz fora de {lo}-{hi} kHz"));
            }
        }
        if let Some(max) = self.max_clock_khz {
            if t.pixel_clock_khz > max {
                v.push(format!("clock acima de {} MHz", max / 1000));
            }
        }
        v
    }
}

fn table(out: &mut String, header: &[&str], rows: &[Vec<String>]) {
    let cols = header.len();
    let mut w: Vec<usize> = header.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, c) in r.iter().enumerate().take(cols) {
            w[i] = w[i].max(c.chars().count());
        }
    }
    let line = |out: &mut String, cells: &mut dyn Iterator<Item = &str>| {
        out.push('|');
        for (i, c) in cells.enumerate() {
            let pad = w[i] - c.chars().count();
            let _ = write!(out, " {c}{} |", " ".repeat(pad));
        }
        out.push('\n');
    };
    line(out, &mut header.iter().copied());
    out.push('|');
    for wi in &w {
        let _ = write!(out, "{}|", "-".repeat(wi + 2));
    }
    out.push('\n');
    for r in rows {
        line(out, &mut r.iter().map(String::as_str));
    }
    out.push('\n');
}

pub fn render(input: &Input) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# IvyBridge-RS Probe v{}\n", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(
        o,
        "Somente leitura: nenhum registrador foi escrito e nenhum modo foi aplicado.\n"
    );

    match &input.system {
        Ok(sys) => render_system(&mut o, sys),
        Err(e) => {
            let _ = writeln!(o, "## Sistema\n\n(consulta indisponível: {e})\n");
        }
    }

    let _ = writeln!(o, "## Monitor (EDID)\n");
    let _ = writeln!(o, "Fonte: {}\n", input.edid_source);
    if let Some(e) = &input.edid_error {
        let _ = writeln!(o, "Erro ao interpretar o EDID: {e}\n");
    }
    let sink = input
        .edid
        .as_ref()
        .map(SinkLimits::from_edid)
        .unwrap_or_default();
    if let Some(edid) = &input.edid {
        render_edid(&mut o, edid, &sink);
    } else {
        let _ = writeln!(
            o,
            "Nenhum EDID disponível; a análise ignora os limites do monitor.\n"
        );
    }

    let mut ceilings = limits::source_ceilings();
    ceilings.extend(input.extra_ceilings.iter().cloned());
    let _ = writeln!(o, "## Tetos de pixel clock considerados\n");
    let mut rows: Vec<Vec<String>> = ceilings
        .iter()
        .map(|c| {
            vec![
                c.label.clone(),
                format!("{:.0} MHz", c.mhz()),
                c.source.to_string(),
            ]
        })
        .collect();
    if let Some(c) = input.edid.as_ref().and_then(limits::sink_ceiling) {
        rows.push(vec![
            c.label.clone(),
            format!("{:.0} MHz", c.mhz()),
            c.source.into(),
        ]);
    }
    table(&mut o, &["Teto", "Clock", "Fonte"], &rows);

    render_modes(&mut o, &input.modes, &ceilings, &sink);
    render_max_refresh(&mut o, &input.modes, &ceilings, &sink);
    render_conclusions(&mut o, input, &ceilings, &sink);
    o
}

fn render_system(o: &mut String, sys: &SystemInfo) {
    let _ = writeln!(o, "## Saídas ativas (Windows)\n");
    if sys.outputs.is_empty() {
        let _ = writeln!(o, "Nenhuma saída ativa encontrada.\n");
    }
    for (i, out) in sys.outputs.iter().enumerate() {
        render_output(o, i, out);
    }
}

fn render_output(o: &mut String, i: usize, out: &ActiveOutput) {
    let _ = writeln!(o, "### Saída {} — {}\n", i + 1, out.gdi_name);
    let _ = writeln!(o, "- Adaptador: {}", or_dash(&out.adapter_name));
    let _ = writeln!(o, "- Monitor: {}", or_dash(&out.monitor_name));
    if let Some(t) = out.output_technology {
        let _ = writeln!(o, "- Conexão: {t}");
    }
    if let Some(s) = out.signal {
        let _ = writeln!(
            o,
            "- Sinal atual: {}x{} @ {:.3} Hz, pixel clock {:.2} MHz, total {}x{}, H {:.2} kHz",
            s.active.0,
            s.active.1,
            s.v_freq_hz,
            s.pixel_rate_hz as f64 / 1e6,
            s.total.0,
            s.total.1,
            s.h_freq_hz / 1000.0
        );
    } else if let Some(r) = out.refresh_hz {
        let _ = writeln!(o, "- Frequência atual: {r:.3} Hz");
    }
    let _ = writeln!(
        o,
        "- EDID no registro: {}",
        out.edid
            .as_ref()
            .map_or("não encontrado".to_string(), |e| format!(
                "{} bytes",
                e.len()
            ))
    );

    // Frequências oferecidas pelo driver, agrupadas por resolução.
    let mut by_res: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for m in &out.driver_modes {
        let v = by_res.entry((m.width, m.height)).or_default();
        if !v.contains(&m.refresh_hz) {
            v.push(m.refresh_hz);
        }
    }
    if !by_res.is_empty() {
        let _ = writeln!(o, "\nModos que o driver atual oferece:\n");
        let rows: Vec<Vec<String>> = by_res
            .iter()
            .rev()
            .map(|(&(w, h), hz)| {
                let mut hz = hz.clone();
                hz.sort_unstable();
                vec![
                    format!("{w}x{h}"),
                    hz.iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                ]
            })
            .collect();
        table(o, &["Resolução", "Hz"], &rows);
    } else {
        o.push('\n');
    }
}

fn or_dash(s: &str) -> &str {
    if s.is_empty() {
        "—"
    } else {
        s
    }
}

fn render_edid(o: &mut String, e: &Edid, sink: &SinkLimits) {
    let _ = writeln!(
        o,
        "- Identificação: {} {:04X}{} (fabricado em {}, semana {})",
        e.manufacturer,
        e.product_code,
        e.name().map(|n| format!(" \"{n}\"")).unwrap_or_default(),
        e.year,
        e.week
    );
    let _ = writeln!(
        o,
        "- EDID {}.{}, entrada {}{}, {}x{} cm, {} extensão(ões)",
        e.version.0,
        e.version.1,
        e.interface,
        e.bits_per_color
            .map(|b| format!(" {b} bits/cor"))
            .unwrap_or_default(),
        e.size_cm.0,
        e.size_cm.1,
        e.extension_count
    );
    if let Some(r) = e.range_limits() {
        let _ = writeln!(
            o,
            "- Range limits: V {}–{} Hz, H {}–{} kHz, pixel clock máx {}",
            r.v_min_hz,
            r.v_max_hz,
            r.h_min_khz,
            r.h_max_khz,
            r.max_pixel_clock_mhz
                .map_or("não declarado".into(), |m| format!("{m} MHz"))
        );
    }
    let _ = writeln!(
        o,
        "- HDMI VSDB: {}; TMDS máx declarado: {}",
        if e.has_hdmi_vsdb() {
            "presente"
        } else {
            "ausente (sink tratado como DVI)"
        },
        e.max_tmds_mhz()
            .map_or("não declarado".into(), |m| format!("{m} MHz"))
    );
    if let Some((lo, hi)) = e.freesync_range() {
        let _ = writeln!(o, "- AMD FreeSync (VRR) via HDMI: {lo}–{hi} Hz");
    }
    for c in &e.cta {
        for b in &c.blocks {
            if let CtaBlock::Video { vics } = b {
                let list: Vec<String> = vics
                    .iter()
                    .map(|s| {
                        let base = match crate::edid::vic_info(s.vic) {
                            Some((w, h, hz, i)) => {
                                format!("{}x{}{}{}", w, h, if i { "i" } else { "p" }, hz)
                            }
                            None => format!("VIC {}", s.vic),
                        };
                        if s.native {
                            format!("{base}*")
                        } else {
                            base
                        }
                    })
                    .collect();
                let _ = writeln!(o, "- CTA-861 VICs: {}", list.join(", "));
            }
        }
    }
    if !e.standard.is_empty() {
        let list: Vec<String> = e
            .standard
            .iter()
            .map(|s| format!("{}x{}@{}", s.width, s.height, s.refresh_hz))
            .collect();
        let _ = writeln!(o, "- Standard timings: {}", list.join(", "));
    }
    o.push('\n');

    let dtds = e.detailed_timings();
    if !dtds.is_empty() {
        let _ = writeln!(
            o,
            "Timings detalhados declarados pelo monitor (o 1º é o preferido):\n"
        );
        let rows: Vec<Vec<String>> = dtds
            .iter()
            .map(|t| {
                vec![
                    format!(
                        "{}x{}{}",
                        t.h_active,
                        t.v_active,
                        if t.interlaced { "i" } else { "" }
                    ),
                    format!("{:.2}", t.refresh_hz()),
                    format!("{:.2}", t.pixel_clock_mhz()),
                    format!("{}x{}", t.h_total(), t.v_total()),
                    format!("{:.2}", t.h_freq_khz()),
                    fit_marks(t.pixel_clock_khz, &limits::source_ceilings(), sink, t),
                ]
            })
            .collect();
        table(
            o,
            &["Modo", "Hz", "MHz", "Total", "H kHz", "Cabe em"],
            &rows,
        );
    }
}

/// Ex.: "165 ✗ · 225 ✓ · monitor ✓".
fn fit_marks(clock_khz: u32, ceilings: &[Ceiling], sink: &SinkLimits, t: &Timing) -> String {
    let mut parts: Vec<String> = ceilings
        .iter()
        .map(|c| {
            format!(
                "{:.0} {}",
                c.mhz(),
                if clock_khz <= c.khz { "✓" } else { "✗" }
            )
        })
        .collect();
    let v = sink.violations(t);
    parts.push(format!("monitor {}", if v.is_empty() { "✓" } else { "✗" }));
    parts.join(" · ")
}

fn render_modes(o: &mut String, modes: &[VideoMode], ceilings: &[Ceiling], sink: &SinkLimits) {
    let _ = writeln!(o, "## Análise de modos candidatos\n");
    let _ = writeln!(
        o,
        "Pixel clock exigido por fórmula (MHz). Mpix/s = só pixels ativos, o piso teórico.\n"
    );
    let rows: Vec<Vec<String>> = modes
        .iter()
        .map(|&m| {
            let mut r = vec![m.to_string(), format!("{:.1}", m.active_pixel_rate_mhz())];
            for f in Formula::ALL {
                r.push(format!("{:.2}", f.generate(m).pixel_clock_mhz()));
            }
            let best = Formula::CvtRb.generate(m);
            r.push(fit_marks(best.pixel_clock_khz, ceilings, sink, &best));
            r
        })
        .collect();
    table(
        o,
        &[
            "Modo",
            "Mpix/s",
            "CVT",
            "CVT-RB",
            "CVT-RBv2",
            "CVT-RB cabe em",
        ],
        &rows,
    );
}

fn render_max_refresh(
    o: &mut String,
    modes: &[VideoMode],
    ceilings: &[Ceiling],
    sink: &SinkLimits,
) {
    let _ = writeln!(o, "## Taxa máxima por resolução\n");
    let _ = writeln!(
        o,
        "Maior taxa inteira cujo pixel clock cabe em cada teto (limitada também pela faixa \
         vertical do monitor, quando conhecida).\n"
    );
    let v_cap = sink.v_hz.map(|(_, hi)| hi as u32).unwrap_or(240);
    let mut resolutions: Vec<(u32, u32)> = modes.iter().map(|m| (m.width, m.height)).collect();
    resolutions.sort_unstable_by(|a, b| b.cmp(a));
    resolutions.dedup();

    let mut header = vec!["Resolução".to_string()];
    for c in ceilings {
        for f in [Formula::CvtRb, Formula::CvtRb2] {
            header.push(format!("{:.0} MHz {}", c.mhz(), f.name()));
        }
    }
    let rows: Vec<Vec<String>> = resolutions
        .iter()
        .map(|&(w, h)| {
            let mut r = vec![format!("{w}x{h}")];
            for c in ceilings {
                for f in [Formula::CvtRb, Formula::CvtRb2] {
                    r.push(
                        max_refresh_under(w, h, f, c.khz, v_cap)
                            .map_or("—".into(), |hz| format!("{hz} Hz")),
                    );
                }
            }
            r
        })
        .collect();
    let header_ref: Vec<&str> = header.iter().map(String::as_str).collect();
    table(o, &header_ref, &rows);
}

fn render_conclusions(o: &mut String, input: &Input, ceilings: &[Ceiling], sink: &SinkLimits) {
    let _ = writeln!(o, "## Leitura dos resultados\n");
    let native = input
        .edid
        .as_ref()
        .and_then(Edid::preferred_timing)
        .map(|t| (t.h_active, t.v_active))
        .unwrap_or((1920, 1080));
    let v_cap = sink.v_hz.map(|(_, hi)| hi as u32).unwrap_or(240);

    if let (Some((_, vmax)), Some(clk)) = (sink.v_hz, sink.max_clock_khz) {
        let _ = writeln!(
            o,
            "- O monitor declara, nesta entrada, até {vmax:.0} Hz e {} MHz de pixel clock.",
            clk / 1000
        );
    }
    if let Some(edid) = &input.edid {
        if let Some(best) = edid
            .detailed_timings()
            .into_iter()
            .filter(|t| (t.h_active, t.v_active) == native && !t.interlaced)
            .max_by(|a, b| a.refresh_hz().total_cmp(&b.refresh_hz()))
        {
            let _ = writeln!(o, "- Modo nativo mais rápido declarado: {best}.");
        }
    }
    for c in ceilings {
        let rb = max_refresh_under(native.0, native.1, Formula::CvtRb, c.khz, v_cap);
        let rb2 = max_refresh_under(native.0, native.1, Formula::CvtRb2, c.khz, v_cap);
        let _ = writeln!(
            o,
            "- Com teto de {:.0} MHz ({}), {}x{} chega a {} com CVT-RB e {} com CVT-RBv2.",
            c.mhz(),
            c.label,
            native.0,
            native.1,
            rb.map_or("—".into(), |h| format!("{h} Hz")),
            rb2.map_or("—".into(), |h| format!("{h} Hz")),
        );
    }
    let _ = writeln!(
        o,
        "- 144 Hz em {}x{} exige pelo menos {:.0} MHz (CVT-RBv2): acima do teto do HDMI da \
         HD 4000 em qualquer cenário documentado.",
        native.0,
        native.1,
        Formula::CvtRb2
            .generate(VideoMode::new(native.0, native.1, 144))
            .pixel_clock_mhz()
    );
    if let Ok(sys) = &input.system {
        if let Some(max) = sys
            .outputs
            .iter()
            .flat_map(|out| out.driver_modes.iter())
            .filter(|m| (m.width, m.height) == native)
            .map(|m| m.refresh_hz)
            .max()
        {
            let _ = writeln!(
                o,
                "- O driver Intel atual oferece no máximo {max} Hz em {}x{}.",
                native.0, native.1
            );
        }
    }
    let _ = writeln!(
        o,
        "\nO teto efetivo da placa pode ser menor que 225 MHz: a VBT (tabela de vídeo da BIOS) \
         pode impor um limite próprio, e o circuito HDMI da placa-mãe pode não sustentar o clock. \
         Confirmar isso é o objetivo da v0.3."
    );
}
