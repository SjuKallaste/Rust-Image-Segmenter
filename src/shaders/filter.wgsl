// <filter params>
struct FilterParams {
    h_min: u32,
    h_max: u32,
    s_min: u32,
    s_max: u32,
    bri_min: u32,
    bri_max: u32,
    width: u32,
    height: u32,
}
// </filter params>

@group(0) @binding(0) var<storage, read> pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> out_mask: array<u32>;
@group(0) @binding(2) var<storage, read_write> out_count: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params: FilterParams;

// <rgb to hsb conversion>
fn rgb_to_hsb(r_raw: f32, g_raw: f32, b_raw: f32) -> vec3<f32> {
    let r = r_raw / 255.0;
    let g = g_raw / 255.0;
    let b = b_raw / 255.0;
    let cmax = max(max(r, g), b);
    let cmin = min(min(r, g), b);
    let delta = cmax - cmin;
    let brightness = cmax;
    var saturation = 0.0;
    if cmax > 0.0 {
        saturation = delta / cmax;
    }
    var hue_norm = 0.0;
    if delta > 0.0 {
        if cmax == r {
            hue_norm = (((g - b) / delta) % 6.0) / 6.0;
        } else if cmax == g {
            hue_norm = (((b - r) / delta) + 2.0) / 6.0;
        } else {
            hue_norm = (((r - g) / delta) + 4.0) / 6.0;
        }
    }
    if hue_norm < 0.0 {
        hue_norm = hue_norm + 1.0;
    }
    return vec3<f32>(hue_norm * 255.0, saturation * 255.0, brightness * 255.0);
}
// </rgb to hsb conversion>

// <pixel matching>
fn matches(hsb: vec3<f32>) -> bool {
    let h = hsb.x;
    let s = hsb.y;
    let bri = hsb.z;
    if s < f32(params.s_min) || s > f32(params.s_max) {
        return false;
    }
    if bri < f32(params.bri_min) || bri > f32(params.bri_max) {
        return false;
    }
    if params.h_min <= params.h_max {
        return h >= f32(params.h_min) && h <= f32(params.h_max);
    } else {
        return h >= f32(params.h_min) || h <= f32(params.h_max);
    }
}
// </pixel matching>

// <main compute entry>
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total = params.width * params.height;
    if idx >= total {
        return;
    }

    let packed = pixels[idx];
    let r = f32(packed & 0xFFu);
    let g = f32((packed >> 8u) & 0xFFu);
    let b = f32((packed >> 16u) & 0xFFu);

    let hsb = rgb_to_hsb(r, g, b);
    let is_match = matches(hsb);

    if is_match {
        out_mask[idx] = packed | 0xFF000000u;
        atomicAdd(&out_count[0], 1u);
    } else {
        out_mask[idx] = 0xFF000000u;
    }
}
// </main compute entry>
