//! Cálculo de timings de vídeo: VESA CVT 1.2 (blanking padrão,
//! Reduced Blanking v1 e v2).
//!
//! Todos os geradores produzem modos progressivos, sem margens.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub h_active: u32,
    pub h_front: u32,
    pub h_sync: u32,
    pub h_back: u32,
    pub v_active: u32,
    pub v_front: u32,
    pub v_sync: u32,
    pub v_back: u32,
    pub pixel_clock_khz: u32,
    pub interlaced: bool,
}

impl Timing {
    pub fn h_total(&self) -> u32 {
        self.h_active + self.h_front + self.h_sync + self.h_back
    }

    pub fn v_total(&self) -> u32 {
        self.v_active + self.v_front + self.v_sync + self.v_back
    }

    pub fn pixel_clock_mhz(&self) -> f64 {
        self.pixel_clock_khz as f64 / 1000.0
    }

    pub fn h_freq_khz(&self) -> f64 {
        self.pixel_clock_khz as f64 / self.h_total() as f64
    }

    /// Frequência de quadro (para entrelaçado, a de campo dividida por 2).
    pub fn refresh_hz(&self) -> f64 {
        let frame =
            self.pixel_clock_khz as f64 * 1000.0 / (self.h_total() as f64 * self.v_total() as f64);
        if self.interlaced {
            frame * 2.0
        } else {
            frame
        }
    }
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}x{}{} @ {:.2} Hz, {:.2} MHz, total {}x{}, H {:.2} kHz",
            self.h_active,
            self.v_active,
            if self.interlaced { "i" } else { "" },
            self.refresh_hz(),
            self.pixel_clock_mhz(),
            self.h_total(),
            self.v_total(),
            self.h_freq_khz()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Formula {
    Cvt,
    CvtRb,
    CvtRb2,
}

impl Formula {
    pub const ALL: [Formula; 3] = [Formula::Cvt, Formula::CvtRb, Formula::CvtRb2];

    pub fn name(self) -> &'static str {
        match self {
            Formula::Cvt => "CVT",
            Formula::CvtRb => "CVT-RB",
            Formula::CvtRb2 => "CVT-RBv2",
        }
    }

    pub fn generate(self, mode: VideoMode) -> Timing {
        match self {
            Formula::Cvt => cvt(mode),
            Formula::CvtRb => cvt_rb(mode),
            Formula::CvtRb2 => cvt_rb2(mode),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VideoMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

impl VideoMode {
    pub const fn new(width: u32, height: u32, refresh_hz: u32) -> Self {
        VideoMode {
            width,
            height,
            refresh_hz,
        }
    }

    /// Limite inferior absoluto: só pixels ativos, sem blanking.
    pub fn active_pixel_rate_mhz(self) -> f64 {
        self.width as f64 * self.height as f64 * self.refresh_hz as f64 / 1e6
    }

    /// Aceita `1920x1080@144` (também `1920x1080x144` ou `1920*1080@144`).
    pub fn parse(s: &str) -> Option<VideoMode> {
        let s = s.trim().to_ascii_lowercase();
        let (res, hz) = s.split_once('@').or_else(|| s.rsplit_once('x'))?;
        let (w, h) = res.split_once(['x', '*'])?;
        let mode = VideoMode::new(
            w.trim().parse().ok()?,
            h.trim().parse().ok()?,
            hz.trim().trim_end_matches("hz").parse().ok()?,
        );
        (mode.width > 0 && mode.height > 0 && mode.refresh_hz > 0).then_some(mode)
    }
}

impl fmt::Display for VideoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{} @ {} Hz", self.width, self.height, self.refresh_hz)
    }
}

const CELL_GRAN: u32 = 8;

/// Largura do VSYNC no CVT, codificando a proporção da tela.
fn cvt_vsync_lines(w: u32, h: u32) -> u32 {
    let is = |a: u32, b: u32| h * a == w * b;
    if is(4, 3) {
        4
    } else if is(16, 9) {
        5
    } else if is(16, 10) {
        6
    } else if is(5, 4) || is(15, 9) {
        7
    } else {
        10
    }
}

fn round_down(v: f64, step: f64) -> f64 {
    (v / step).floor() * step
}

/// VESA CVT 1.2, blanking padrão (CRT).
pub fn cvt(mode: VideoMode) -> Timing {
    const MIN_VSYNC_BP_US: f64 = 550.0;
    const MIN_V_PORCH: u32 = 3;
    const MIN_V_BPORCH: u32 = 6;
    const C_PRIME: f64 = 30.0;
    const M_PRIME: f64 = 300.0;
    const H_SYNC_PER: f64 = 0.08;
    const CLOCK_STEP_MHZ: f64 = 0.25;

    let h_active = mode.width / CELL_GRAN * CELL_GRAN;
    let v_lines = mode.height;
    let v_sync = cvt_vsync_lines(mode.width, mode.height);
    let rate = mode.refresh_hz as f64;

    let h_period_est = (1e6 / rate - MIN_VSYNC_BP_US) / (v_lines + MIN_V_PORCH) as f64; // µs
    let mut vsync_bp = (MIN_VSYNC_BP_US / h_period_est).floor() as u32 + 1;
    if vsync_bp < v_sync + MIN_V_BPORCH {
        vsync_bp = v_sync + MIN_V_BPORCH;
    }

    let duty = (C_PRIME - M_PRIME * h_period_est / 1000.0).max(20.0);
    let h_blank = (h_active as f64 * duty / (100.0 - duty) / (2 * CELL_GRAN) as f64).floor() as u32
        * 2
        * CELL_GRAN;
    let h_total = h_active + h_blank;
    let clock_mhz = round_down(h_total as f64 / h_period_est, CLOCK_STEP_MHZ);
    let h_sync = (H_SYNC_PER * h_total as f64 / CELL_GRAN as f64).floor() as u32 * CELL_GRAN;
    let h_back = h_blank / 2;

    Timing {
        h_active,
        h_front: h_blank - h_sync - h_back,
        h_sync,
        h_back,
        v_active: v_lines,
        v_front: MIN_V_PORCH,
        v_sync,
        v_back: vsync_bp - v_sync,
        pixel_clock_khz: (clock_mhz * 1000.0).round() as u32,
        interlaced: false,
    }
}

/// VESA CVT 1.2, Reduced Blanking v1 (160 px de blanking horizontal).
pub fn cvt_rb(mode: VideoMode) -> Timing {
    const RB_MIN_V_BLANK_US: f64 = 460.0;
    const RB_V_FPORCH: u32 = 3;
    const MIN_V_BPORCH: u32 = 6;
    const RB_H_BLANK: u32 = 160;
    const RB_H_SYNC: u32 = 32;
    const CLOCK_STEP_MHZ: f64 = 0.25;

    let h_active = mode.width / CELL_GRAN * CELL_GRAN;
    let v_lines = mode.height;
    let v_sync = cvt_vsync_lines(mode.width, mode.height);
    let rate = mode.refresh_hz as f64;

    let h_period_est = (1e6 / rate - RB_MIN_V_BLANK_US) / v_lines as f64;
    let vbi = ((RB_MIN_V_BLANK_US / h_period_est).floor() as u32 + 1)
        .max(RB_V_FPORCH + v_sync + MIN_V_BPORCH);
    let v_total = v_lines + vbi;
    let h_total = h_active + RB_H_BLANK;
    let clock_mhz = round_down(rate * v_total as f64 * h_total as f64 / 1e6, CLOCK_STEP_MHZ);

    Timing {
        h_active,
        h_front: RB_H_BLANK / 2 - RB_H_SYNC,
        h_sync: RB_H_SYNC,
        h_back: RB_H_BLANK / 2,
        v_active: v_lines,
        v_front: RB_V_FPORCH,
        v_sync,
        v_back: vbi - RB_V_FPORCH - v_sync,
        pixel_clock_khz: (clock_mhz * 1000.0).round() as u32,
        interlaced: false,
    }
}

/// VESA CVT 1.2, Reduced Blanking v2 (80 px de blanking horizontal,
/// VSYNC fixo de 8 linhas, clock em passos de 1 kHz).
pub fn cvt_rb2(mode: VideoMode) -> Timing {
    const RB_MIN_V_BLANK_US: f64 = 460.0;
    const RB_V_SYNC: u32 = 8;
    const RB_V_BPORCH: u32 = 6;
    const RB_MIN_V_FPORCH: u32 = 1;
    const RB_H_BLANK: u32 = 80;
    const RB_H_SYNC: u32 = 32;
    const RB_H_FPORCH: u32 = 8;

    let h_active = mode.width; // RBv2 usa granularidade de 1 pixel
    let v_lines = mode.height;
    let rate = mode.refresh_hz as f64;

    let h_period_est = (1e6 / rate - RB_MIN_V_BLANK_US) / v_lines as f64;
    let vbi = ((RB_MIN_V_BLANK_US / h_period_est).floor() as u32 + 1)
        .max(RB_V_SYNC + RB_V_BPORCH + RB_MIN_V_FPORCH);
    let v_total = v_lines + vbi;
    let h_total = h_active + RB_H_BLANK;
    let clock_khz = (rate * v_total as f64 * h_total as f64 / 1e3).floor() as u32;

    Timing {
        h_active,
        h_front: RB_H_FPORCH,
        h_sync: RB_H_SYNC,
        h_back: RB_H_BLANK - RB_H_FPORCH - RB_H_SYNC,
        v_active: v_lines,
        v_front: vbi - RB_V_SYNC - RB_V_BPORCH,
        v_sync: RB_V_SYNC,
        v_back: RB_V_BPORCH,
        pixel_clock_khz: clock_khz,
        interlaced: false,
    }
}

/// Maior taxa inteira (Hz) em que `formula` cabe em `ceiling_khz`,
/// procurando em `1..=max_hz`. `None` se nem 1 Hz couber.
pub fn max_refresh_under(
    width: u32,
    height: u32,
    formula: Formula,
    ceiling_khz: u32,
    max_hz: u32,
) -> Option<u32> {
    // O clock cresce monotonicamente com a taxa: busca binária.
    let fits = |hz: u32| {
        formula
            .generate(VideoMode::new(width, height, hz))
            .pixel_clock_khz
            <= ceiling_khz
    };
    if !fits(1) {
        return None;
    }
    let (mut lo, mut hi) = (1, max_hz);
    if fits(hi) {
        return Some(hi);
    }
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo)
}
