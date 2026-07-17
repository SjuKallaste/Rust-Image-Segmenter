// <segmentation edge params>
struct SegParams {
    tol: u32,
    width: u32,
    height: u32,
    _pad: u32,
}
// </segmentation edge params>

@group(0) @binding(0) var<storage, read> pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> out_edges: array<u32>;
@group(0) @binding(2) var<uniform> params: SegParams;

// <color distance, mirrors segment::color_dist>
fn color_dist(a: vec3<u32>, b: vec3<u32>) -> u32 {
    let dr = u32(abs(i32(a.x) - i32(b.x)));
    let dg = u32(abs(i32(a.y) - i32(b.y)));
    let db = u32(abs(i32(a.z) - i32(b.z)));
    return dr + dg + db;
}
// </color distance, mirrors segment::color_dist>

fn unpack(p: u32) -> vec3<u32> {
    return vec3<u32>(p & 0xFFu, (p >> 8u) & 0xFFu, (p >> 16u) & 0xFFu);
}

// <main compute entry, one invocation per pixel, outputs right/down edge bits>
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total = params.width * params.height;
    if idx >= total {
        return;
    }

    let x = idx % params.width;
    let y = idx / params.width;
    let self_color = unpack(pixels[idx]);

    var edges: u32 = 0u;

    if x + 1u < params.width {
        let right_idx = idx + 1u;
        let right_color = unpack(pixels[right_idx]);
        if color_dist(self_color, right_color) <= params.tol {
            edges = edges | 1u;
        }
    }

    if y + 1u < params.height {
        let down_idx = idx + params.width;
        let down_color = unpack(pixels[down_idx]);
        if color_dist(self_color, down_color) <= params.tol {
            edges = edges | 2u;
        }
    }

    out_edges[idx] = edges;
}
// </main compute entry, one invocation per pixel, outputs right/down edge bits>
