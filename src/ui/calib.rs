use egui::{Pos2, Rect};

// <coordinate helpers>
pub fn screen_to_norm(screen: Pos2, rect: Rect) -> Pos2 {
    Pos2::new(
        ((screen.x - rect.min.x) / rect.width()).clamp(0.0, 1.0),
        ((screen.y - rect.min.y) / rect.height()).clamp(0.0, 1.0),
    )
}

pub fn norm_to_screen(norm: Pos2, rect: Rect) -> Pos2 {
    Pos2::new(
        rect.min.x + norm.x * rect.width(),
        rect.min.y + norm.y * rect.height(),
    )
}

pub fn norm_to_px_dist(p1: Pos2, p2: Pos2, w: u32, h: u32) -> f64 {
    let dx = (p1.x - p2.x) as f64 * w as f64;
    let dy = (p1.y - p2.y) as f64 * h as f64;
    (dx * dx + dy * dy).sqrt()
}
// </coordinate helpers>
