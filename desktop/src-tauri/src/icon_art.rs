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
    assert!(svg.contains(r#"M26 18 L26 84 L44 66 L58 94 L68 89 L54 61 L78 61 Z"#));
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
    let amber = count_near(&img, [0xF5, 0xB4, 0x29], 40);
    assert!(plate > 200, "plate pixels {plate}");
    assert!(cyan > 10, "cyan pixels {cyan}");
    assert!(amber > 5, "amber pixels {amber}");
}
