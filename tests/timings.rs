//! Valores de referência da planilha VESA CVT 1.2 / tabela DMT.

use ivybridge_rs::display::device_path_to_enum_key;
use ivybridge_rs::timings::{cvt, cvt_rb, cvt_rb2, max_refresh_under, Formula, VideoMode};

fn m(w: u32, h: u32, hz: u32) -> VideoMode {
    VideoMode::new(w, h, hz)
}

#[test]
fn cvt_1080p60() {
    let t = cvt(m(1920, 1080, 60));
    assert_eq!(t.pixel_clock_khz, 173_000);
    assert_eq!((t.h_total(), t.v_total()), (2576, 1120));
    assert_eq!((t.h_front, t.h_sync, t.h_back), (128, 200, 328));
    assert_eq!((t.v_front, t.v_sync, t.v_back), (3, 5, 32));
}

#[test]
fn cvt_1080p75() {
    let t = cvt(m(1920, 1080, 75));
    assert_eq!(t.pixel_clock_khz, 220_750);
    assert_eq!((t.h_total(), t.v_total()), (2608, 1130));
    // H_SYNC = floor(8% × 2608 / 8) × 8, como em drm_cvt_mode().
    assert_eq!(t.h_sync, 208);
}

#[test]
fn cvt_rb_1080p60() {
    let t = cvt_rb(m(1920, 1080, 60));
    assert_eq!(t.pixel_clock_khz, 138_500);
    assert_eq!((t.h_total(), t.v_total()), (2080, 1111));
    assert_eq!((t.h_front, t.h_sync, t.h_back), (48, 32, 80));
    assert_eq!((t.v_front, t.v_sync, t.v_back), (3, 5, 23));
}

#[test]
fn cvt_rb_1080p120() {
    // O DTD do monitor usa 2080x1111 a 277,3 MHz: mesmo blanking horizontal
    // do CVT-RB, mas com intervalo vertical menor que o mínimo de 460 µs.
    let t = cvt_rb(m(1920, 1080, 120));
    assert_eq!((t.h_total(), t.v_total()), (2080, 1144));
    assert_eq!(t.pixel_clock_khz, 285_500);
}

#[test]
fn cvt_rb2_1080p60() {
    let t = cvt_rb2(m(1920, 1080, 60));
    assert_eq!(t.pixel_clock_khz, 133_320);
    assert_eq!((t.h_total(), t.v_total()), (2000, 1111));
    assert_eq!((t.v_front, t.v_sync, t.v_back), (17, 8, 6));
}

#[test]
fn refresh_is_close_to_requested() {
    for f in Formula::ALL {
        for &(w, h, hz) in &[(1920, 1080, 60), (1600, 900, 120), (1280, 720, 144)] {
            let t = f.generate(m(w, h, hz));
            let err = (t.refresh_hz() - hz as f64).abs();
            assert!(
                err < 0.5,
                "{} {w}x{h}@{hz}: {:.3} Hz",
                f.name(),
                t.refresh_hz()
            );
        }
    }
}

#[test]
fn max_refresh_search() {
    assert_eq!(
        max_refresh_under(1920, 1080, Formula::CvtRb, 165_000, 240),
        Some(71)
    );
    assert_eq!(
        max_refresh_under(1920, 1080, Formula::CvtRb, 225_000, 240),
        Some(95)
    );
    assert_eq!(
        max_refresh_under(1280, 720, Formula::CvtRb2, 400_000, 122),
        Some(122)
    );
}

#[test]
fn parses_mode_strings() {
    assert_eq!(VideoMode::parse("1920x1080@144"), Some(m(1920, 1080, 144)));
    assert_eq!(VideoMode::parse("1600*900@75Hz"), Some(m(1600, 900, 75)));
    assert_eq!(VideoMode::parse("1280x720x120"), Some(m(1280, 720, 120)));
    assert_eq!(VideoMode::parse("1920x1080"), None);
    assert_eq!(VideoMode::parse("0x0@60"), None);
}

#[test]
fn maps_device_path_to_registry_key() {
    let p = r"\\?\DISPLAY#MSD2500#4&799cdb5&0&UID50727680#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}";
    assert_eq!(
        device_path_to_enum_key(p).as_deref(),
        Some(r"DISPLAY\MSD2500\4&799cdb5&0&UID50727680")
    );
    assert_eq!(device_path_to_enum_key(""), None);
}
