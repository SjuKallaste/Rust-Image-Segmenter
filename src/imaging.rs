use egui::ColorImage;
use image::DynamicImage;
use std::collections::HashSet;

use crate::color::hsv_to_rgb;

// <box blur>
pub fn box_blur(img: &DynamicImage, radius: u32) -> DynamicImage {
    if radius == 0 { return img.clone(); }
    let src = img.to_rgb8();
    let w = src.width();
    let h = src.height();
    let mut out = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let x0 = x.saturating_sub(radius);
            let x1 = (x + radius).min(w - 1);
            let y0 = y.saturating_sub(radius);
            let y1 = (y + radius).min(h - 1);
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for ny in y0..=y1 {
                for nx in x0..=x1 {
                    let p = src.get_pixel(nx, ny);
                    r += p[0] as u32; g += p[1] as u32; b += p[2] as u32; n += 1;
                }
            }
            out.put_pixel(x, y, image::Rgb([(r/n) as u8, (g/n) as u8, (b/n) as u8]));
        }
    }
    DynamicImage::ImageRgb8(out)
}
// </box blur>

// <sobel edge detection>
pub fn sobel_texture(img: &DynamicImage) -> ColorImage {
    let gray = img.to_luma8();
    let w = gray.width() as usize;
    let h = gray.height() as usize;
    let get = |x: usize, y: usize| gray.get_pixel(x as u32, y as u32)[0] as f32;
    let mut pixels = vec![egui::Color32::TRANSPARENT; w * h];
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let gx = -get(x-1,y-1) - 2.0*get(x-1,y) - get(x-1,y+1)
                +get(x+1,y-1) + 2.0*get(x+1,y) + get(x+1,y+1);
            let gy = -get(x-1,y-1) - 2.0*get(x,y-1) - get(x+1,y-1)
                +get(x-1,y+1) + 2.0*get(x,y+1) + get(x+1,y+1);
            let mag = (gx * gx + gy * gy).sqrt().min(255.0) as u8;
            pixels[y * w + x] = egui::Color32::from_rgba_unmultiplied(255, 100, 0, mag);
        }
    }
    ColorImage { size: [w, h], pixels }
}
// </sobel edge detection>

// <segmentation texture>
pub fn build_seg_texture(labels: &[i32], w: u32, h: u32, n: usize, selected: &HashSet<usize>) -> ColorImage {
    let palette: Vec<egui::Color32> = (0..n).map(|i| {
        let rgb = hsv_to_rgb(i as f32 * 360.0 / n.max(1) as f32, 0.75, 0.90);
        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }).collect();
    let has_sel = !selected.is_empty();
    let pixels = labels.iter().map(|&l| {
        if l < 0 || l as usize >= n { return egui::Color32::from_gray(20); }
        let idx = l as usize;
        let c = palette[idx];
        if has_sel && !selected.contains(&idx) {
            let [r, g, b, _] = c.to_array();
            egui::Color32::from_rgb(r / 6, g / 6, b / 6)
        } else {
            c
        }
    }).collect();
    ColorImage { size: [w as usize, h as usize], pixels }
}
// </segmentation texture>

// <image format conversion>
pub fn dyn_to_color_image(img: &DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let pixels = rgba.pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    ColorImage { size: [w as usize, h as usize], pixels }
}
// </image format conversion>
