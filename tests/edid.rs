use ivybridge_rs::edid::{load_bytes, CtaBlock, Edid, EdidError};
use ivybridge_rs::limits;

const TARGET_HDMI: &str = include_str!("edid_samples/msd2500_target_hdmi.hex");

fn target() -> Edid {
    Edid::parse(&load_bytes(TARGET_HDMI.as_bytes()).unwrap()).unwrap()
}

#[test]
fn identifies_target_monitor() {
    let e = target();
    assert_eq!(e.manufacturer, "MSD");
    assert_eq!(e.product_code, 0x2500);
    assert_eq!(e.name(), Some("TGTATTP2BL01"));
    assert_eq!(e.year, 2022);
    assert_eq!(e.version, (1, 3));
    assert_eq!(e.extension_count, 1);
}

#[test]
fn reads_range_limits() {
    let r = target().range_limits().unwrap();
    assert_eq!((r.v_min_hz, r.v_max_hz), (48, 122));
    assert_eq!((r.h_min_khz, r.h_max_khz), (31, 159));
    assert_eq!(r.max_pixel_clock_mhz, Some(300));
}

#[test]
fn reads_detailed_timings() {
    let t = target().detailed_timings();
    assert_eq!(t.len(), 3);

    // Preferido: CEA 1080p60.
    assert_eq!((t[0].h_active, t[0].v_active), (1920, 1080));
    assert_eq!(t[0].pixel_clock_khz, 148_500);
    assert_eq!((t[0].h_total(), t[0].v_total()), (2200, 1125));
    assert!((t[0].refresh_hz() - 60.0).abs() < 0.01);

    // Na extensão CTA: 1080p120 com reduced blanking.
    assert_eq!(t[2].pixel_clock_khz, 277_300);
    assert_eq!((t[2].h_total(), t[2].v_total()), (2080, 1111));
    assert!((t[2].refresh_hz() - 120.0).abs() < 0.01);
}

#[test]
fn reads_cta_blocks() {
    let e = target();
    assert!(e.has_hdmi_vsdb());
    assert_eq!(e.max_tmds_mhz(), None);
    assert_eq!(e.freesync_range(), Some((48, 122)));
    let vics: Vec<u8> = e.cta[0]
        .blocks
        .iter()
        .find_map(|b| match b {
            CtaBlock::Video { vics } => Some(vics.iter().map(|s| s.vic).collect()),
            _ => None,
        })
        .unwrap();
    assert_eq!(vics[0], 16);
    assert!(vics.contains(&78));
}

#[test]
fn sink_ceiling_comes_from_range_limits() {
    assert_eq!(limits::sink_ceiling(&target()).unwrap().khz, 300_000);
}

#[test]
fn accepts_binary_and_rejects_corruption() {
    let bytes = load_bytes(TARGET_HDMI.as_bytes()).unwrap();
    assert_eq!(load_bytes(&bytes).unwrap(), bytes); // binário passa direto

    let mut bad = bytes.clone();
    bad[60] ^= 0xFF;
    assert!(matches!(
        Edid::parse(&bad),
        Err(EdidError::BadChecksum { block: 0, .. })
    ));

    let mut bad_ext = bytes.clone();
    bad_ext[200] ^= 0x01;
    assert!(matches!(
        Edid::parse(&bad_ext),
        Err(EdidError::BadChecksum { block: 1, .. })
    ));

    assert!(matches!(
        Edid::parse(&bytes[..100]),
        Err(EdidError::TooShort(100))
    ));
    assert!(load_bytes(b"00 ff zz").is_err());
}

#[test]
fn dtd_roundtrip_and_insertion() {
    use ivybridge_rs::edid::{add_dtd_to_cta, encode_dtd, parse_dtd};
    use ivybridge_rs::timings::{cvt_rb, VideoMode};

    // Codificar os DTDs do próprio monitor reproduz os mesmos timings.
    for t in target().detailed_timings() {
        let dtd = encode_dtd(&t, (540, 300)).unwrap();
        assert_eq!(parse_dtd(&dtd), Some(t));
    }

    let t75 = cvt_rb(VideoMode::new(1920, 1080, 75));
    assert_eq!(t75.pixel_clock_khz, 174_500);
    let dtd = encode_dtd(&t75, (540, 300)).unwrap();
    assert_eq!(parse_dtd(&dtd), Some(t75));

    let base = load_bytes(TARGET_HDMI.as_bytes()).unwrap();
    let out = add_dtd_to_cta(&base, &dtd).unwrap();
    assert_eq!(out.len(), base.len());
    assert_eq!(out[..128], base[..128]); // bloco base intacto
    let e = Edid::parse(&out).unwrap(); // checksum recalculado
    let dtds = e.detailed_timings();
    assert_eq!(dtds.len(), 4);
    assert_eq!(dtds[3], t75);
}
