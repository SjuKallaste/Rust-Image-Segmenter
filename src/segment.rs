use image::DynamicImage;
use std::collections::VecDeque;

use crate::types::Region;

// <color distance>
pub fn color_dist(a: [u8; 3], b: [u8; 3]) -> u32 {
    (a[0] as i32 - b[0] as i32).unsigned_abs()
        + (a[1] as i32 - b[1] as i32).unsigned_abs()
        + (a[2] as i32 - b[2] as i32).unsigned_abs()
}
// </color distance>

// <flood fill segmentation>
pub fn segment(img: &DynamicImage, tol: u32, min_px: usize, scale: f64) -> (Vec<i32>, Vec<Region>) {
    let rgb = img.to_rgb8();
    let w = rgb.width() as usize;
    let h = rgb.height() as usize;
    let pixels: Vec<[u8; 3]> = rgb.pixels().map(|p| [p[0], p[1], p[2]]).collect();
    let mut labels = vec![-1i32; w * h];
    let mut next_lbl = 0usize;
    let mut counts: Vec<usize> = Vec::new();
    let mut color_sum: Vec<[u64; 3]> = Vec::new();
    let mut cx_sum: Vec<u64> = Vec::new();
    let mut cy_sum: Vec<u64> = Vec::new();

    for start in 0..(w * h) {
        if labels[start] != -1 { continue; }
        let lbl = next_lbl as i32;
        next_lbl += 1;
        counts.push(0); color_sum.push([0; 3]); cx_sum.push(0); cy_sum.push(0);
        let seed = pixels[start];
        labels[start] = lbl;
        let mut q = VecDeque::new();
        q.push_back(start);

        while let Some(idx) = q.pop_front() {
            let px = idx % w;
            let py = idx / w;
            let li = lbl as usize;
            counts[li] += 1;
            let c = pixels[idx];
            color_sum[li][0] += c[0] as u64;
            color_sum[li][1] += c[1] as u64;
            color_sum[li][2] += c[2] as u64;
            cx_sum[li] += px as u64;
            cy_sum[li] += py as u64;

            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = px as i32 + dx;
                let ny = py as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 { continue; }
                let nidx = ny as usize * w + nx as usize;
                if labels[nidx] != -1 { continue; }
                if color_dist(seed, pixels[nidx]) <= tol {
                    labels[nidx] = lbl;
                    q.push_back(nidx);
                }
            }
        }
    }

    // <filter small regions and build output>
    let px_per_cm2 = scale * scale;
    let mut id_map = vec![-1i32; next_lbl];
    let mut regions: Vec<Region> = Vec::new();
    let mut new_id = 0usize;

    for l in 0..next_lbl {
        if counts[l] < min_px { continue; }
        id_map[l] = new_id as i32;
        let cnt = counts[l];
        let cs = color_sum[l];
        let avg = [(cs[0] / cnt as u64) as u8, (cs[1] / cnt as u64) as u8, (cs[2] / cnt as u64) as u8];
        let centroid = (cx_sum[l] as f32 / (cnt as f32 * w as f32), cy_sum[l] as f32 / (cnt as f32 * h as f32));
        regions.push(Region { index: new_id + 1, pixel_count: cnt, area_cm2: cnt as f64 / px_per_cm2, avg_color: avg, centroid });
        new_id += 1;
    }

    for lbl in labels.iter_mut() {
        if *lbl >= 0 { *lbl = id_map[*lbl as usize]; }
    }
    // </filter small regions and build output>

    (labels, regions)
}
// </flood fill segmentation>
