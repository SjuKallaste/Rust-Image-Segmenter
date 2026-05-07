use egui::ColorImage;
use image::DynamicImage;

// <color filter struct>
#[derive(Clone, PartialEq)]
pub struct ColorFilter {
    pub label: &'static str,
    pub swatch: egui::Color32,
    pub hue: Option<(u8, u8)>,
    pub sat: (u8, u8),
    pub bri: (u8, u8),
}
// </color filter struct>

// <named filter list>
pub fn all_color_filters() -> Vec<ColorFilter> {
    vec![
        ColorFilter { label: "Red", swatch: egui::Color32::from_rgb(220, 50, 50), hue: Some((0, 15)), sat: (100, 255), bri: (50, 255) },
        ColorFilter { label: "Orange", swatch: egui::Color32::from_rgb(230, 130, 30), hue: Some((11, 30)), sat: (100, 255), bri: (50, 255) },
        ColorFilter { label: "Yellow", swatch: egui::Color32::from_rgb(230, 210, 30), hue: Some((28, 52)), sat: (100, 255), bri: (50, 255) },
        ColorFilter { label: "Green", swatch: egui::Color32::from_rgb(40, 190, 60), hue: Some((49, 115)), sat: (50, 255), bri: (30, 255) },
        ColorFilter { label: "Cyan", swatch: egui::Color32::from_rgb(30, 200, 210), hue: Some((113, 142)), sat: (50, 255), bri: (30, 255) },
        ColorFilter { label: "Blue", swatch: egui::Color32::from_rgb(50, 90, 220), hue: Some((138, 185)), sat: (50, 255), bri: (30, 255) },
        ColorFilter { label: "Purple", swatch: egui::Color32::from_rgb(140, 50, 210), hue: Some((183, 213)), sat: (50, 255), bri: (30, 255) },
        ColorFilter { label: "Pink", swatch: egui::Color32::from_rgb(220, 80, 160), hue: Some((213, 245)), sat: (50, 255), bri: (50, 255) },
        ColorFilter { label: "White", swatch: egui::Color32::from_rgb(240, 240, 240), hue: None, sat: (0, 50), bri: (200, 255) },
        ColorFilter { label: "Gray", swatch: egui::Color32::from_rgb(140, 140, 140), hue: None, sat: (0, 50), bri: (80, 200) },
        ColorFilter { label: "Black", swatch: egui::Color32::from_rgb(30, 30, 30), hue: None, sat: (0, 255), bri: (0, 50) },
    ]
}
// </named filter list>

// <rgb to hsb conversion>
pub fn rgb_to_hsb(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let cmax = rf.max(gf).max(bf);
    let cmin = rf.min(gf).min(bf);
    let delta = cmax - cmin;
    let brightness = cmax;
    let saturation = if cmax == 0.0 { 0.0 } else { delta / cmax };
    let hue_norm = if delta == 0.0 {
        0.0
    } else if cmax == rf {
        (((gf - bf) / delta) % 6.0) / 6.0
    } else if cmax == gf {
        (((bf - rf) / delta) + 2.0) / 6.0
    } else {
        (((rf - gf) / delta) + 4.0) / 6.0
    };
    let hue_norm = if hue_norm < 0.0 { hue_norm + 1.0 } else { hue_norm };
    (
        (hue_norm * 255.0).round() as u8,
        (saturation * 255.0).round() as u8,
        (brightness * 255.0).round() as u8,
    )
}
// </rgb to hsb conversion>

// <hsv to rgb conversion>
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0f32),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}
// </hsv to rgb conversion>

// <pixel matching>
pub fn pixel_matches_filter(r: u8, g: u8, b: u8, filter: &ColorFilter) -> bool {
    let (h, s, bri) = rgb_to_hsb(r, g, b);
    if s < filter.sat.0 || s > filter.sat.1 { return false; }
    if bri < filter.bri.0 || bri > filter.bri.1 { return false; }
    match filter.hue {
        None => true,
        Some((lo, hi)) => {
            if lo <= hi { h >= lo && h <= hi } else { h >= lo || h <= hi }
        }
    }
}

pub fn pixel_matches_imagej(r: u8, g: u8, b: u8, h_min: u8, h_max: u8, s_min: u8, s_max: u8, bri_min: u8, bri_max: u8) -> bool {
    let (h, s, bri) = rgb_to_hsb(r, g, b);
    if s < s_min || s > s_max { return false; }
    if bri < bri_min || bri > bri_max { return false; }
    if h_min <= h_max { h >= h_min && h <= h_max } else { h >= h_min || h <= h_max }
}
// </pixel matching>

// <texture builders>
pub fn build_color_filter_texture(img: &DynamicImage, filters: &[&ColorFilter]) -> ColorImage {
    let rgb = img.to_rgb8();
    let w = rgb.width() as usize;
    let h = rgb.height() as usize;
    let pixels = rgb.pixels().map(|p| {
        let (r, g, b) = (p[0], p[1], p[2]);
        if filters.iter().any(|f| pixel_matches_filter(r, g, b, f)) {
            egui::Color32::from_rgb(r, g, b)
        } else {
            egui::Color32::BLACK
        }
    }).collect();
    ColorImage { size: [w, h], pixels }
}

pub fn build_imagej_filter_texture(img: &DynamicImage, h_min: u8, h_max: u8, s_min: u8, s_max: u8, bri_min: u8, bri_max: u8) -> ColorImage {
    let rgb = img.to_rgb8();
    let w = rgb.width() as usize;
    let h = rgb.height() as usize;
    let pixels = rgb.pixels().map(|p| {
        let (r, g, b) = (p[0], p[1], p[2]);
        if pixel_matches_imagej(r, g, b, h_min, h_max, s_min, s_max, bri_min, bri_max) {
            egui::Color32::from_rgb(r, g, b)
        } else {
            egui::Color32::BLACK
        }
    }).collect();
    ColorImage { size: [w, h], pixels }
}
// </texture builders>

// <prominence analysis>
pub fn compute_prominent_filters(img: &DynamicImage, filters: &[ColorFilter], threshold: f64) -> Vec<usize> {
    let rgb = img.to_rgb8();
    let total = (rgb.width() * rgb.height()) as f64;
    let mut counts = vec![0usize; filters.len()];
    for p in rgb.pixels() {
        for (i, f) in filters.iter().enumerate() {
            if pixel_matches_filter(p[0], p[1], p[2], f) {
                counts[i] += 1;
                break;
            }
        }
    }
    let mut prominent: Vec<(usize, usize)> = counts.iter().enumerate()
        .filter(|&(_, &c)| c as f64 / total >= threshold)
        .map(|(i, &c)| (i, c))
        .collect();
    prominent.sort_by(|a, b| b.1.cmp(&a.1));
    prominent.into_iter().map(|(i, _)| i).collect()
}
// </prominence analysis>

// <area measurement>
pub fn pixel_area_for_filter(img: &DynamicImage, filter: &ColorFilter, scale_px_per_cm: f64) -> (usize, f64) {
    let rgb = img.to_rgb8();
    let count = rgb.pixels().filter(|p| pixel_matches_filter(p[0], p[1], p[2], filter)).count();
    (count, count as f64 / (scale_px_per_cm * scale_px_per_cm))
}

pub fn pixel_area_imagej(img: &DynamicImage, h_min: u8, h_max: u8, s_min: u8, s_max: u8, bri_min: u8, bri_max: u8, scale_px_per_cm: f64) -> (usize, f64) {
    let rgb = img.to_rgb8();
    let count = rgb.pixels()
        .filter(|p| pixel_matches_imagej(p[0], p[1], p[2], h_min, h_max, s_min, s_max, bri_min, bri_max))
        .count();
    (count, count as f64 / (scale_px_per_cm * scale_px_per_cm))
}

#[allow(dead_code)]
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (h, s, v) = rgb_to_hsb(r, g, b);
    (h as f32 / 255.0 * 360.0, s as f32 / 255.0, v as f32 / 255.0)
}
// </area measurement>
