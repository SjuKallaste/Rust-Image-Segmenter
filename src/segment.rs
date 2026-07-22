use image::RgbImage;
use rayon::prelude::*;
use std::collections::VecDeque;

use crate::types::Region;

// <color distance>
pub fn color_dist(a: [u8; 3], b: [u8; 3]) -> u32 {
    (a[0] as i32 - b[0] as i32).unsigned_abs()
        + (a[1] as i32 - b[1] as i32).unsigned_abs()
        + (a[2] as i32 - b[2] as i32).unsigned_abs()
}
// </color distance>

// <union find, shared by the parallel tiled merge and the gpu edge merge>
// find() is iterative (not recursive) with path compression, and union()
// unions by size. This matters specifically for segment_gpu, which creates
// one Dsu entry per PIXEL (not per region), so a large connected area could
// otherwise produce a recursion chain deep enough to overflow the stack.
struct Dsu { parent: Vec<usize>, size: Vec<usize> }
impl Dsu {
    fn new(n: usize) -> Self { Dsu { parent: (0..n).collect(), size: vec![1; n] } }
    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }
    fn union(&mut self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb { return; }
        if self.size[ra] < self.size[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
    }
}
// </union find, shared by the parallel tiled merge and the gpu edge merge>

// <single threaded flood fill, exact reference implementation, seed based tolerance>
pub fn segment(rgb: &RgbImage, tol: u32, min_px: usize, scale: f64) -> (Vec<i32>, Vec<Region>) {
    let w = rgb.width() as usize;
    let h = rgb.height() as usize;
    let raw = rgb.as_raw();
    let pixels: Vec<[u8; 3]> = raw.par_chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();
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

    finalize_regions(labels, &counts, &color_sum, &cx_sum, &cy_sum, w, h, min_px, scale)
}
// </single threaded flood fill, exact reference implementation, seed based tolerance>

// <parallel flood fill, tiled with union find seam merge, seed based tolerance>
pub fn segment_parallel(rgb: &RgbImage, tol: u32, min_px: usize, scale: f64) -> (Vec<i32>, Vec<Region>) {
    let w = rgb.width() as usize;
    let h = rgb.height() as usize;
    let raw = rgb.as_raw();
    let pixels: Vec<[u8; 3]> = raw.par_chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();

    let n_threads = rayon::current_num_threads().max(1);
    let n_strips = n_threads.min(h.max(1));
    let strip_h = (h + n_strips - 1) / n_strips;

    struct StripResult {
        y0: usize,
        y1: usize,
        local_labels: Vec<i32>,
        seed_colors: Vec<[u8; 3]>,
    }

    let strip_results: Vec<StripResult> = (0..n_strips).into_par_iter().map(|s| {
        let y0 = s * strip_h;
        let y1 = (y0 + strip_h).min(h);
        if y0 >= y1 {
            return StripResult { y0, y1, local_labels: Vec::new(), seed_colors: Vec::new() };
        }
        let strip_rows = y1 - y0;
        let mut local_labels = vec![-1i32; w * strip_rows];
        let mut seed_colors: Vec<[u8; 3]> = Vec::new();
        let mut next_local = 0usize;

        for sy in 0..strip_rows {
            for sx in 0..w {
                let local_idx = sy * w + sx;
                if local_labels[local_idx] != -1 { continue; }
                let global_idx = (y0 + sy) * w + sx;
                let seed = pixels[global_idx];
                let lbl = next_local as i32;
                next_local += 1;
                seed_colors.push(seed);
                local_labels[local_idx] = lbl;
                let mut q = VecDeque::new();
                q.push_back(local_idx);

                while let Some(idx) = q.pop_front() {
                    let px = idx % w;
                    let py = idx / w;
                    for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let nx = px as i32 + dx;
                        let ny = py as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= strip_rows as i32 { continue; }
                        let nidx = ny as usize * w + nx as usize;
                        if local_labels[nidx] != -1 { continue; }
                        let gidx = (y0 + ny as usize) * w + nx as usize;
                        if color_dist(seed, pixels[gidx]) <= tol {
                            local_labels[nidx] = lbl;
                            q.push_back(nidx);
                        }
                    }
                }
            }
        }

        StripResult { y0, y1, local_labels, seed_colors }
    }).collect();

    let mut strip_offsets = vec![0usize; strip_results.len()];
    let mut running = 0usize;
    for (i, sr) in strip_results.iter().enumerate() {
        strip_offsets[i] = running;
        running += sr.seed_colors.len();
    }
    let total_local_labels = running;

    let mut dsu = Dsu::new(total_local_labels);

    let mut global_labels = vec![-1i32; w * h];
    for (i, sr) in strip_results.iter().enumerate() {
        let offset = strip_offsets[i];
        for sy in 0..(sr.y1 - sr.y0) {
            for sx in 0..w {
                let local = sr.local_labels[sy * w + sx];
                if local >= 0 {
                    global_labels[(sr.y0 + sy) * w + sx] = (offset + local as usize) as i32;
                }
            }
        }
    }

    for i in 0..strip_results.len().saturating_sub(1) {
        let sr_top = &strip_results[i];
        let sr_bot = &strip_results[i + 1];
        if sr_top.y1 != sr_bot.y0 { continue; }
        if sr_top.y1 == 0 || sr_top.y1 >= h { continue; }

        let seam_y_top = sr_top.y1 - 1;
        let seam_y_bot = sr_bot.y0;
        let top_offset = strip_offsets[i];
        let bot_offset = strip_offsets[i + 1];

        for x in 0..w {
            let top_local = sr_top.local_labels[(seam_y_top - sr_top.y0) * w + x];
            let bot_local = sr_bot.local_labels[(seam_y_bot - sr_bot.y0) * w + x];
            if top_local < 0 || bot_local < 0 { continue; }
            let top_seed = sr_top.seed_colors[top_local as usize];
            let bot_seed = sr_bot.seed_colors[bot_local as usize];
            if color_dist(top_seed, bot_seed) <= tol {
                dsu.union(top_offset + top_local as usize, bot_offset + bot_local as usize);
            }
        }
    }

    let (final_labels, n_compact) = resolve_dsu(&mut dsu, &global_labels, w, h);
    let (counts, color_sum, cx_sum, cy_sum) = accumulate_stats(rgb, &final_labels, n_compact, w, h);
    finalize_regions(final_labels, &counts, &color_sum, &cx_sum, &cy_sum, w, h, min_px, scale)
}
// </parallel flood fill, tiled with union find seam merge, seed based tolerance>

// <gpu segmentation, neighbor based (chained) tolerance>
pub fn segment_gpu(rgb: &RgbImage, edges: &[u8], min_px: usize, scale: f64) -> (Vec<i32>, Vec<Region>) {
    let w = rgb.width() as usize;
    let h = rgb.height() as usize;

    // Defensive guard: if the GPU readback ever returns a buffer that
    // doesn't match the expected pixel count, treat every pixel as
    // disconnected rather than indexing out of bounds and panicking the
    // background thread silently.
    let edges_valid = edges.len() == w * h;

    let mut dsu = Dsu::new(w * h);

    if edges_valid {
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let e = edges[idx];
                if e & 1 != 0 && x + 1 < w {
                    dsu.union(idx, idx + 1);
                }
                if e & 2 != 0 && y + 1 < h {
                    dsu.union(idx, idx + w);
                }
            }
        }
    }

    let identity_labels: Vec<i32> = (0..(w * h) as i32).collect();
    let (final_labels, n_compact) = resolve_dsu(&mut dsu, &identity_labels, w, h);
    let (counts, color_sum, cx_sum, cy_sum) = accumulate_stats(rgb, &final_labels, n_compact, w, h);
    finalize_regions(final_labels, &counts, &color_sum, &cx_sum, &cy_sum, w, h, min_px, scale)
}
// </gpu segmentation, neighbor based (chained) tolerance>

// <resolve dsu roots to compact contiguous ids>
fn resolve_dsu(dsu: &mut Dsu, pre_labels: &[i32], w: usize, h: usize) -> (Vec<i32>, usize) {
    let mut root_to_compact: std::collections::HashMap<usize, i32> = std::collections::HashMap::new();
    let mut next_compact = 0i32;
    let mut final_labels = vec![-1i32; w * h];
    for idx in 0..(w * h) {
        let l = pre_labels[idx];
        if l < 0 { continue; }
        let root = dsu.find(l as usize);
        let compact = *root_to_compact.entry(root).or_insert_with(|| {
            let id = next_compact;
            next_compact += 1;
            id
        });
        final_labels[idx] = compact;
    }
    (final_labels, next_compact as usize)
}
// </resolve dsu roots to compact contiguous ids>

// <accumulate per region pixel stats from final labels>
fn accumulate_stats(rgb: &RgbImage, labels: &[i32], n_compact: usize, w: usize, h: usize) -> (Vec<usize>, Vec<[u64; 3]>, Vec<u64>, Vec<u64>) {
    let raw = rgb.as_raw();
    let mut counts = vec![0usize; n_compact];
    let mut color_sum = vec![[0u64; 3]; n_compact];
    let mut cx_sum = vec![0u64; n_compact];
    let mut cy_sum = vec![0u64; n_compact];

    for idx in 0..(w * h) {
        let l = labels[idx];
        if l < 0 { continue; }
        let li = l as usize;
        let px = idx % w;
        let py = idx / w;
        let o = idx * 3;
        counts[li] += 1;
        color_sum[li][0] += raw[o] as u64;
        color_sum[li][1] += raw[o + 1] as u64;
        color_sum[li][2] += raw[o + 2] as u64;
        cx_sum[li] += px as u64;
        cy_sum[li] += py as u64;
    }

    (counts, color_sum, cx_sum, cy_sum)
}
// </accumulate per region pixel stats from final labels>

// <shared region finalization, filters by min_px and remaps to contiguous ids>
fn finalize_regions(
    mut labels: Vec<i32>,
    counts: &[usize],
    color_sum: &[[u64; 3]],
    cx_sum: &[u64],
    cy_sum: &[u64],
    w: usize,
    h: usize,
    min_px: usize,
    scale: f64,
) -> (Vec<i32>, Vec<Region>) {
    let px_per_cm2 = scale * scale;
    let next_lbl = counts.len();
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

    (labels, regions)
}
// </shared region finalization, filters by min_px and remaps to contiguous ids>