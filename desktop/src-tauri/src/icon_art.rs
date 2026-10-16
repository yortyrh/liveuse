use std::path::PathBuf;

fn icons_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons")
}

fn count_near(img: &image::RgbaImage, rgb: [u8; 3], tol: u8) -> usize {
    img.pixels()
        .filter(|px| {
            px.0[3] > 200
                && px.0[..3]
                    .iter()
                    .zip(rgb)
                    .all(|(channel, expected)| channel.abs_diff(expected) <= tol)
        })
        .count()
}

#[test]
fn app_icon_matches_the_spark_mark() {
    let svg = std::fs::read_to_string(icons_dir().join("icon.svg")).expect("read icon.svg");
    assert!(svg.contains(r"M26 18 L26 84 L44 66 L58 94 L68 89 L54 61 L78 61 Z"));
    assert!(svg.contains(r##"fill="#3EE0E8""##));
    assert!(svg.contains(r##"fill="#F5B429""##));
    assert!(svg.contains(r##"fill="#FFF4CC""##));
    assert!(svg.contains(r##"fill="#111111""##));

    let img = image::open(icons_dir().join("32x32.png"))
        .expect("open 32x32.png")
        .to_rgba8();
    assert_eq!(img.dimensions(), (32, 32));
    let plate = count_near(&img, [0x11, 0x11, 0x11], 8);
    let cyan = count_near(&img, [0x3E, 0xE0, 0xE8], 40);
    assert!(plate > 200, "plate pixels {plate}");
    assert!(cyan > 10, "cyan pixels {cyan}");

    let large = image::open(icons_dir().join("128x128.png"))
        .expect("open 128x128.png")
        .to_rgba8();
    let amber = count_near(&large, [0xF5, 0xB4, 0x29], 40);
    assert!(amber > 12, "amber pixels {amber}");
}

#[test]
fn tray_template_is_a_black_mask() {
    let svg = std::fs::read_to_string(icons_dir().join("tray.svg")).expect("read tray.svg");
    assert!(svg.contains(r"M3 1.5 L3 12.5 L6 9.5 L8.2 14 L10 13.1 L7.8 8.6 L11.5 8.6 Z"));
    assert!(
        svg.contains(
            "12.2,11.2 12.7,12.6 14.2,13.1 12.7,13.6 12.2,15 11.7,13.6 10.2,13.1 11.7,12.6"
        )
    );
    assert!(!svg.to_ascii_lowercase().contains("#3ee0e8"));
    assert!(!svg.to_ascii_lowercase().contains("#f5b429"));

    let img = image::open(icons_dir().join("trayTemplate.png"))
        .expect("open trayTemplate.png")
        .to_rgba8();
    assert_eq!(img.dimensions(), (32, 32));

    let mut ink = 0usize;
    for (x, y, px) in img.enumerate_pixels() {
        let edge = x < 4 || y < 4 || x >= 28 || y >= 28;
        if edge {
            assert!(px.0[3] <= 80, "padding at {x},{y}: {:?}", px.0);
        }
        if px.0[3] > 200 {
            assert!(
                px.0[0] <= 8 && px.0[1] <= 8 && px.0[2] <= 8,
                "colored pixel at {x},{y}: {:?}",
                px.0
            );
            ink += 1;
        }
    }
    assert!(ink > 10, "ink pixels {ink}");
}
