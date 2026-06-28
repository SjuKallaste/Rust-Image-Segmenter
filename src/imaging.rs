use egui::ColorImage;
use image::{DynamicImage, RgbImage};
use rayon::prelude::*;
use std::collections::HashSet;

use crate::color::hsv_to_rgb;

// <box blur, takes a pre-converted rgb8 buffer, two-pass separable, rayon parallel>
pub fn box_blur(src: &RgbImage, radius: u32) -> RgbImage {
    if radius == 0 { return src.clone(); }
    let w = src.width() as usize;
    let h = src.height() as usize;
    let r = radius as usize;
    let raw = src.as_raw();

    let mut horiz = vec![[0f32; 3]; w * h];
    horiz.par_chunks_mut(w).enumerate().for_each(|(y, row_out)| {
        let row = &raw[y * w * 3..(y + 1) * w * 3];
        let get = |x: usize| {
            let i = x * 3;
            [row[i] as f32, row[i + 1] as f32, row[i + 2] as f32]
        };
        let mut sum = [0f32; 3];
        let mut count = 0f32;
        for nx in 0..=r.min(w - 1) {
            let c = get(nx);
            sum[0] += c[0]; sum[1] += c[1]; sum[2] += c[2];
            count += 1.0;
        }
        row_out[0] = [sum[0] / count, sum[1] / count, sum[2] / count];
        for x in 1..w {
            let add_idx = x + r;
            let rem_idx = x as isize - r as isize - 1;
            if add_idx < w {
                let c = get(add_idx);
                sum[0] += c[0]; sum[1] += c[1]; sum[2] += c[2];
                count += 1.0;
            }
            if rem_idx >= 0 {
                let c = get(rem_idx as usize);
                sum[0] -= c[0]; sum[1] -= c[1]; sum[2] -= c[2];
                count -= 1.0;
            }
            row_out[x] = [sum[0] / count, sum[1] / count, sum[2] / count];
        }
    });

    let mut out_raw = vec![0u8; w * h * 3];

    let columns: Vec<Vec<[f32; 3]>> = (0..w).into_par_iter().map(|x| {
        let mut col_out = vec![[0f32; 3]; h];
        let mut sum = [0f32; 3];
        let mut count = 0f32;
        for ny in 0..=r.min(h - 1) {
            let c = horiz[ny * w + x];
            sum[0] += c[0]; sum[1] += c[1]; sum[2] += c[2];
            count += 1.0;
        }
        col_out[0] = [sum[0] / count, sum[1] / count, sum[2] / count];
        for y in 1..h {
            let add_idx = y + r;
            let rem_idx = y as isize - r as isize - 1;
            if add_idx < h {
                let c = horiz[add_idx * w + x];
                sum[0] += c[0]; sum[1] += c[1]; sum[2] += c[2];
                count += 1.0;
            }
            if rem_idx >= 0 {
                let c = horiz[rem_idx as usize * w + x];
                sum[0] -= c[0]; sum[1] -= c[1]; sum[2] -= c[2];
                count -= 1.0;
            }
            col_out[y] = [sum[0] / count, sum[1] / count, sum[2] / count];
        }
        col_out
    }).collect();

    out_raw.par_chunks_mut(3).enumerate().for_each(|(idx, px)| {
        let x = idx % w;
        let y = idx / w;
        let c = columns[x][y];
        px[0] = c[0] as u8;
        px[1] = c[1] as u8;
        px[2] = c[2] as u8;
    });

    RgbImage::from_raw(w as u32, h as u32, out_raw).expect("buffer size matches dimensions")
}
// </box blur, takes a pre-converted rgb8 buffer, two-pass separable, rayon parallel>

// <sobel edge detection, takes a pre-converted rgb8 buffer>
pub fn sobel_texture(rgb: &RgbImage) -> ColorImage {
    let gray = image::DynamicImage::ImageRgb8(rgb.clone()).to_luma8();
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
// </sobel edge detection, takes a pre-converted rgb8 buffer>

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

// <image format conversion, rayon parallel>
pub fn dyn_to_color_image(img: &DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let raw = rgba.as_raw();
    let pixels = raw.par_chunks_exact(4)
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    ColorImage { size: [w as usize, h as usize], pixels }
}
// </image format conversion, rayon parallel>