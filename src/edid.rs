//! Parser de EDID 1.3/1.4 com suporte à extensão CTA-861.
//!
//! Só leitura de bytes: nenhuma dependência de plataforma.

use crate::timings::Timing;
use std::fmt;

pub const BLOCK_LEN: usize = 128;
const HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdidError {
    TooShort(usize),
    BadHeader,
    BadChecksum { block: usize, sum: u8 },
    BadHex(String),
}

impl fmt::Display for EdidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdidError::TooShort(n) => write!(f, "EDID curto demais ({n} bytes, mínimo 128)"),
            EdidError::BadHeader => write!(f, "cabeçalho EDID inválido"),
            EdidError::BadChecksum { block, sum } => {
                write!(f, "checksum inválido no bloco {block} (soma = 0x{sum:02x})")
            }
            EdidError::BadHex(s) => write!(f, "texto hexadecimal inválido: {s}"),
        }
    }
}

impl std::error::Error for EdidError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoInterface {
    Analog,
    Undefined,
    Dvi,
    HdmiA,
    HdmiB,
    Mddi,
    DisplayPort,
    Other(u8),
}

impl fmt::Display for VideoInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            VideoInterface::Analog => "analógica (VGA)",
            VideoInterface::Undefined => "digital (não definida)",
            VideoInterface::Dvi => "DVI",
            VideoInterface::HdmiA => "HDMI-a",
            VideoInterface::HdmiB => "HDMI-b",
            VideoInterface::Mddi => "MDDI",
            VideoInterface::DisplayPort => "DisplayPort",
            VideoInterface::Other(v) => return write!(f, "digital (código {v})"),
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeLimits {
    pub v_min_hz: u16,
    pub v_max_hz: u16,
    pub h_min_khz: u16,
    pub h_max_khz: u16,
    /// Pixel clock máximo declarado, em MHz (múltiplo de 10).
    pub max_pixel_clock_mhz: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardTiming {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Descriptor {
    Detailed(Timing),
    RangeLimits(RangeLimits),
    Name(String),
    Serial(String),
    Text(String),
    Other(u8),
    Dummy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtaBlock {
    Audio {
        len: usize,
    },
    Video {
        vics: Vec<Svd>,
    },
    HdmiVsdb {
        physical_address: [u8; 2],
        max_tmds_mhz: Option<u32>,
    },
    HdmiForumVsdb {
        max_tmds_mhz: Option<u32>,
    },
    AmdFreeSync {
        version: u8,
        min_hz: u8,
        max_hz: u8,
    },
    Vendor {
        oui: u32,
        len: usize,
    },
    Speaker,
    Extended {
        tag: u8,
        len: usize,
    },
    Unknown {
        tag: u8,
        len: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Svd {
    pub vic: u8,
    pub native: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtaExtension {
    pub revision: u8,
    pub underscan: bool,
    pub basic_audio: bool,
    pub ycbcr444: bool,
    pub ycbcr422: bool,
    pub native_dtds: u8,
    pub blocks: Vec<CtaBlock>,
    pub detailed: Vec<Timing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edid {
    pub manufacturer: String,
    pub product_code: u16,
    pub serial: u32,
    pub week: u8,
    pub year: u16,
    pub version: (u8, u8),
    pub digital: bool,
    pub interface: VideoInterface,
    pub bits_per_color: Option<u8>,
    pub size_cm: (u8, u8),
    pub established: Vec<StandardTiming>,
    pub standard: Vec<StandardTiming>,
    pub descriptors: Vec<Descriptor>,
    pub extension_count: u8,
    pub cta: Vec<CtaExtension>,
    /// Blocos de extensão que não são CTA-861 (tag, índice).
    pub other_extensions: Vec<(u8, usize)>,
}

/// Aceita binário puro ou texto hexadecimal (espaços, quebras de linha e
/// prefixos `0x` são ignorados).
pub fn load_bytes(raw: &[u8]) -> Result<Vec<u8>, EdidError> {
    if raw.len() >= 8 && raw[..8] == HEADER {
        return Ok(raw.to_vec());
    }
    let text = String::from_utf8_lossy(raw);
    let mut digits = String::new();
    for token in text.split_whitespace() {
        let token = token.trim_start_matches("0x").trim_start_matches("0X");
        for c in token.chars() {
            if c.is_ascii_hexdigit() {
                digits.push(c);
            } else if c != ',' && c != ':' {
                return Err(EdidError::BadHex(format!("caractere '{c}'")));
            }
        }
    }
    if digits.len() % 2 != 0 {
        return Err(EdidError::BadHex("número ímpar de dígitos".into()));
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&digits[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| EdidError::BadHex(e.to_string()))
}

fn checksum(block: &[u8]) -> u8 {
    block.iter().fold(0u8, |a, b| a.wrapping_add(*b))
}

fn ascii_field(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&b| b == 0x0A || b == 0)
        .unwrap_or(bytes.len());
    bytes[..end]
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '?'
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Detailed Timing Descriptor (18 bytes). Retorna `None` se for um
/// descritor de display (pixel clock zero).
pub fn parse_dtd(d: &[u8]) -> Option<Timing> {
    let clock_10khz = u16::from_le_bytes([d[0], d[1]]) as u32;
    if clock_10khz == 0 {
        return None;
    }
    let h_active = d[2] as u32 | ((d[4] as u32 >> 4) << 8);
    let h_blank = d[3] as u32 | ((d[4] as u32 & 0x0F) << 8);
    let v_active = d[5] as u32 | ((d[7] as u32 >> 4) << 8);
    let v_blank = d[6] as u32 | ((d[7] as u32 & 0x0F) << 8);
    let h_front = d[8] as u32 | (((d[11] as u32 >> 6) & 3) << 8);
    let h_sync = d[9] as u32 | (((d[11] as u32 >> 4) & 3) << 8);
    let v_front = (d[10] as u32 >> 4) | (((d[11] as u32 >> 2) & 3) << 4);
    let v_sync = (d[10] as u32 & 0x0F) | ((d[11] as u32 & 3) << 4);
    let flags = d[17];
    Some(Timing {
        h_active,
        h_front,
        h_sync,
        h_back: h_blank.saturating_sub(h_front + h_sync),
        v_active,
        v_front,
        v_sync,
        v_back: v_blank.saturating_sub(v_front + v_sync),
        pixel_clock_khz: clock_10khz * 10,
        interlaced: flags & 0x80 != 0,
    })
}

/// Codifica um timing como Detailed Timing Descriptor (18 bytes).
/// `size_mm` é o tamanho da imagem a declarar; a sincronização é digital
/// separada, com HSYNC+ e VSYNC− (convenção do CVT-RB).
pub fn encode_dtd(t: &Timing, size_mm: (u16, u16)) -> Option<[u8; 18]> {
    let clock = t.pixel_clock_khz / 10;
    let h_blank = t.h_total() - t.h_active;
    let v_blank = t.v_total() - t.v_active;
    let fits = clock > 0
        && clock <= 0xFFFF
        && t.h_active <= 0xFFF
        && h_blank <= 0xFFF
        && t.v_active <= 0xFFF
        && v_blank <= 0xFFF
        && t.h_front <= 0x3FF
        && t.h_sync <= 0x3FF
        && t.v_front <= 0x3F
        && t.v_sync <= 0x3F
        && t.pixel_clock_khz % 10 == 0
        && !t.interlaced;
    if !fits {
        return None;
    }
    let (w, h) = (size_mm.0.min(0xFFF), size_mm.1.min(0xFFF));
    let lo = |v: u32| (v & 0xFF) as u8;
    let hi = |v: u32| ((v >> 8) & 0x0F) as u8;
    Some([
        lo(clock),
        (clock >> 8) as u8,
        lo(t.h_active),
        lo(h_blank),
        hi(t.h_active) << 4 | hi(h_blank),
        lo(t.v_active),
        lo(v_blank),
        hi(t.v_active) << 4 | hi(v_blank),
        lo(t.h_front),
        lo(t.h_sync),
        ((t.v_front & 0x0F) as u8) << 4 | (t.v_sync & 0x0F) as u8,
        (((t.h_front >> 8) & 3) as u8) << 6
            | (((t.h_sync >> 8) & 3) as u8) << 4
            | (((t.v_front >> 4) & 3) as u8) << 2
            | ((t.v_sync >> 4) & 3) as u8,
        (w & 0xFF) as u8,
        (h & 0xFF) as u8,
        ((w >> 8) as u8) << 4 | (h >> 8) as u8,
        0,
        0,
        0x1A,
    ])
}

/// Insere um DTD no primeiro espaço livre de uma extensão CTA-861 e
/// recalcula o checksum. Devolve o EDID modificado.
pub fn add_dtd_to_cta(raw: &[u8], dtd: &[u8; 18]) -> Result<Vec<u8>, String> {
    let mut out = raw.to_vec();
    let ext_count = *raw.get(126).ok_or("EDID curto demais")? as usize;
    for idx in 1..=ext_count {
        let start = idx * BLOCK_LEN;
        let Some(block) = out.get_mut(start..start + BLOCK_LEN) else {
            break;
        };
        if block[0] != 0x02 || block[2] < 4 {
            continue;
        }
        let mut j = block[2] as usize;
        while j + 18 <= 127 && block[j] | block[j + 1] != 0 {
            j += 18;
        }
        if j + 18 > 127 || block[j..j + 18].iter().any(|&b| b != 0) {
            continue;
        }
        block[j..j + 18].copy_from_slice(dtd);
        let sum = block[..127].iter().fold(0u8, |a, b| a.wrapping_add(*b));
        block[127] = 0u8.wrapping_sub(sum);
        return Ok(out);
    }
    Err("nenhuma extensão CTA-861 com espaço livre para um DTD".into())
}

fn parse_descriptor(d: &[u8], version_minor: u8) -> Descriptor {
    if let Some(t) = parse_dtd(d) {
        return Descriptor::Detailed(t);
    }
    match d[3] {
        0xFF => Descriptor::Serial(ascii_field(&d[5..18])),
        0xFE => Descriptor::Text(ascii_field(&d[5..18])),
        0xFC => Descriptor::Name(ascii_field(&d[5..18])),
        0xFD => {
            // Offsets de +255 Hz/kHz só existem no EDID 1.4.
            let off = if version_minor >= 4 { d[4] } else { 0 };
            let add = |bit: u8| if off & bit != 0 { 255u16 } else { 0 };
            let max_clk = d[9] as u16 * 10;
            Descriptor::RangeLimits(RangeLimits {
                v_min_hz: d[5] as u16 + add(0x01),
                v_max_hz: d[6] as u16 + add(0x02),
                h_min_khz: d[7] as u16 + add(0x04),
                h_max_khz: d[8] as u16 + add(0x08),
                max_pixel_clock_mhz: if max_clk == 0 { None } else { Some(max_clk) },
            })
        }
        0x10 => Descriptor::Dummy,
        tag => Descriptor::Other(tag),
    }
}

fn parse_standard(b0: u8, b1: u8, version_minor: u8) -> Option<StandardTiming> {
    if (b0 == 0x01 && b1 == 0x01) || b0 == 0x00 {
        return None;
    }
    let width = (b0 as u32 + 31) * 8;
    let height = match b1 >> 6 {
        0 if version_minor < 3 => width,
        0 => width * 10 / 16,
        1 => width * 3 / 4,
        2 => width * 4 / 5,
        _ => width * 9 / 16,
    };
    Some(StandardTiming {
        width,
        height,
        refresh_hz: (b1 & 0x3F) as u32 + 60,
    })
}

const ESTABLISHED: [(u32, u32, u32); 17] = [
    (720, 400, 70),
    (720, 400, 88),
    (640, 480, 60),
    (640, 480, 67),
    (640, 480, 72),
    (640, 480, 75),
    (800, 600, 56),
    (800, 600, 60),
    (800, 600, 72),
    (800, 600, 75),
    (832, 624, 75),
    (1024, 768, 87), // entrelaçado
    (1024, 768, 60),
    (1024, 768, 70),
    (1024, 768, 75),
    (1280, 1024, 75),
    (1152, 870, 75),
];

fn parse_cta(b: &[u8]) -> CtaExtension {
    let revision = b[1];
    let dtd_offset = (b[2] as usize).min(127);
    let flags = b[3];
    let mut blocks = Vec::new();
    let mut i = 4;
    while revision >= 3 && i < dtd_offset {
        let tag = b[i] >> 5;
        let len = (b[i] & 0x1F) as usize;
        let end = (i + 1 + len).min(dtd_offset);
        let p = &b[i + 1..end];
        blocks.push(match tag {
            1 => CtaBlock::Audio { len },
            2 => CtaBlock::Video {
                vics: p.iter().map(|&v| decode_svd(v)).collect(),
            },
            3 if p.len() >= 3 => {
                let oui = p[0] as u32 | (p[1] as u32) << 8 | (p[2] as u32) << 16;
                match oui {
                    0x000C03 => CtaBlock::HdmiVsdb {
                        physical_address: [
                            p.get(3).copied().unwrap_or(0),
                            p.get(4).copied().unwrap_or(0),
                        ],
                        max_tmds_mhz: p.get(6).filter(|&&v| v != 0).map(|&v| v as u32 * 5),
                    },
                    0xC45DD8 => CtaBlock::HdmiForumVsdb {
                        max_tmds_mhz: p.get(4).filter(|&&v| v != 0).map(|&v| v as u32 * 5),
                    },
                    0x00001A if p.len() >= 7 => CtaBlock::AmdFreeSync {
                        version: p[3],
                        min_hz: p[5],
                        max_hz: p[6],
                    },
                    _ => CtaBlock::Vendor { oui, len },
                }
            }
            4 => CtaBlock::Speaker,
            7 => CtaBlock::Extended {
                tag: p.first().copied().unwrap_or(0),
                len,
            },
            _ => CtaBlock::Unknown { tag, len },
        });
        i += 1 + len;
    }
    let mut detailed = Vec::new();
    if dtd_offset >= 4 {
        let mut j = dtd_offset;
        while j + 18 <= 127 {
            match parse_dtd(&b[j..j + 18]) {
                Some(t) => detailed.push(t),
                None => break,
            }
            j += 18;
        }
    }
    CtaExtension {
        revision,
        underscan: flags & 0x80 != 0,
        basic_audio: flags & 0x40 != 0,
        ycbcr444: flags & 0x20 != 0,
        ycbcr422: flags & 0x10 != 0,
        native_dtds: flags & 0x0F,
        blocks,
        detailed,
    }
}

fn decode_svd(v: u8) -> Svd {
    // Para VIC 1–64 o bit 7 indica modo nativo; 129–192 são VIC 1–64 nativos.
    if (129..=192).contains(&v) {
        Svd {
            vic: v & 0x7F,
            native: true,
        }
    } else {
        Svd {
            vic: v,
            native: false,
        }
    }
}

impl Edid {
    pub fn parse(raw: &[u8]) -> Result<Edid, EdidError> {
        if raw.len() < BLOCK_LEN {
            return Err(EdidError::TooShort(raw.len()));
        }
        if raw[..8] != HEADER {
            return Err(EdidError::BadHeader);
        }
        let b = &raw[..BLOCK_LEN];
        let sum = checksum(b);
        if sum != 0 {
            return Err(EdidError::BadChecksum { block: 0, sum });
        }

        let mfg = u16::from_be_bytes([b[8], b[9]]);
        let letter = |shift: u16| (((mfg >> shift) & 0x1F) as u8 + b'@') as char;
        let manufacturer: String = [letter(10), letter(5), letter(0)].iter().collect();

        let version = (b[18], b[19]);
        let input = b[20];
        let digital = input & 0x80 != 0;
        let (interface, bits_per_color) = if digital {
            let depth = match (input >> 4) & 0x07 {
                0 | 7 => None,
                n => Some(4 + 2 * n),
            };
            let iface = match input & 0x0F {
                0 => VideoInterface::Undefined,
                1 => VideoInterface::Dvi,
                2 => VideoInterface::HdmiA,
                3 => VideoInterface::HdmiB,
                4 => VideoInterface::Mddi,
                5 => VideoInterface::DisplayPort,
                n => VideoInterface::Other(n),
            };
            (iface, if version.1 >= 4 { depth } else { None })
        } else {
            (VideoInterface::Analog, None)
        };

        let est_bits = (b[35] as u32) << 16 | (b[36] as u32) << 8 | b[37] as u32;
        let established = ESTABLISHED
            .iter()
            .enumerate()
            .filter(|(i, _)| est_bits & (1 << (23 - i)) != 0)
            .map(|(_, &(w, h, r))| StandardTiming {
                width: w,
                height: h,
                refresh_hz: r,
            })
            .collect();

        let standard = (0..8)
            .filter_map(|i| parse_standard(b[38 + 2 * i], b[39 + 2 * i], version.1))
            .collect();

        let descriptors = (0..4)
            .map(|i| parse_descriptor(&b[54 + 18 * i..72 + 18 * i], version.1))
            .collect();

        let extension_count = b[126];
        let mut cta = Vec::new();
        let mut other_extensions = Vec::new();
        for idx in 1..=extension_count as usize {
            let start = idx * BLOCK_LEN;
            let Some(block) = raw.get(start..start + BLOCK_LEN) else {
                break;
            };
            let sum = checksum(block);
            if sum != 0 {
                return Err(EdidError::BadChecksum { block: idx, sum });
            }
            if block[0] == 0x02 {
                cta.push(parse_cta(block));
            } else {
                other_extensions.push((block[0], idx));
            }
        }

        Ok(Edid {
            manufacturer,
            product_code: u16::from_le_bytes([b[10], b[11]]),
            serial: u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
            week: b[16],
            year: 1990 + b[17] as u16,
            version,
            digital,
            interface,
            bits_per_color,
            size_cm: (b[21], b[22]),
            established,
            standard,
            descriptors,
            extension_count,
            cta,
            other_extensions,
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.descriptors.iter().find_map(|d| match d {
            Descriptor::Name(n) => Some(n.as_str()),
            _ => None,
        })
    }

    pub fn range_limits(&self) -> Option<RangeLimits> {
        self.descriptors.iter().find_map(|d| match d {
            Descriptor::RangeLimits(r) => Some(*r),
            _ => None,
        })
    }

    /// Todos os timings detalhados: bloco base primeiro (o primeiro é o
    /// modo preferido), depois os das extensões CTA.
    pub fn detailed_timings(&self) -> Vec<Timing> {
        let mut v: Vec<Timing> = self
            .descriptors
            .iter()
            .filter_map(|d| match d {
                Descriptor::Detailed(t) => Some(*t),
                _ => None,
            })
            .collect();
        for c in &self.cta {
            v.extend(c.detailed.iter().copied());
        }
        v
    }

    pub fn preferred_timing(&self) -> Option<Timing> {
        self.detailed_timings().into_iter().next()
    }

    /// Clock TMDS máximo declarado pelo sink (HDMI VSDB / HF-VSDB).
    pub fn max_tmds_mhz(&self) -> Option<u32> {
        self.cta
            .iter()
            .flat_map(|c| c.blocks.iter())
            .filter_map(|b| match b {
                CtaBlock::HdmiVsdb { max_tmds_mhz, .. } => *max_tmds_mhz,
                CtaBlock::HdmiForumVsdb { max_tmds_mhz } => *max_tmds_mhz,
                _ => None,
            })
            .max()
    }

    pub fn has_hdmi_vsdb(&self) -> bool {
        self.cta
            .iter()
            .flat_map(|c| c.blocks.iter())
            .any(|b| matches!(b, CtaBlock::HdmiVsdb { .. }))
    }

    pub fn freesync_range(&self) -> Option<(u8, u8)> {
        self.cta
            .iter()
            .flat_map(|c| c.blocks.iter())
            .find_map(|b| match b {
                CtaBlock::AmdFreeSync { min_hz, max_hz, .. } => Some((*min_hz, *max_hz)),
                _ => None,
            })
    }
}

/// Subconjunto da tabela de VICs do CTA-861 (modos progressivos e os
/// entrelaçados mais comuns). VICs fora da tabela são exibidos pelo número.
pub fn vic_info(vic: u8) -> Option<(u32, u32, f64, bool)> {
    // (largura, altura, Hz, entrelaçado)
    Some(match vic {
        1 => (640, 480, 59.94, false),
        2 | 3 => (720, 480, 59.94, false),
        4 => (1280, 720, 60.0, false),
        5 => (1920, 1080, 60.0, true),
        6 | 7 => (720, 480, 59.94, true),
        16 => (1920, 1080, 60.0, false),
        17 | 18 => (720, 576, 50.0, false),
        19 => (1280, 720, 50.0, false),
        20 => (1920, 1080, 50.0, true),
        21 | 22 => (720, 576, 50.0, true),
        31 => (1920, 1080, 50.0, false),
        32 => (1920, 1080, 24.0, false),
        33 => (1920, 1080, 25.0, false),
        34 => (1920, 1080, 30.0, false),
        41 => (1280, 720, 100.0, false),
        47 => (1280, 720, 120.0, false),
        60 => (1280, 720, 24.0, false),
        61 => (1280, 720, 25.0, false),
        62 => (1280, 720, 30.0, false),
        63 => (1920, 1080, 120.0, false),
        64 => (1920, 1080, 100.0, false),
        // 65–78: os mesmos formatos com proporção 64:27.
        65 => (1280, 720, 24.0, false),
        66 => (1280, 720, 25.0, false),
        67 => (1280, 720, 30.0, false),
        68 => (1280, 720, 50.0, false),
        69 => (1280, 720, 60.0, false),
        70 => (1280, 720, 100.0, false),
        71 => (1280, 720, 120.0, false),
        72 => (1920, 1080, 24.0, false),
        73 => (1920, 1080, 25.0, false),
        74 => (1920, 1080, 30.0, false),
        75 => (1920, 1080, 50.0, false),
        76 => (1920, 1080, 60.0, false),
        77 => (1920, 1080, 100.0, false),
        78 => (1920, 1080, 120.0, false),
        93 => (3840, 2160, 24.0, false),
        94 => (3840, 2160, 25.0, false),
        95 => (3840, 2160, 30.0, false),
        96 => (3840, 2160, 50.0, false),
        97 => (3840, 2160, 60.0, false),
        _ => return None,
    })
}
