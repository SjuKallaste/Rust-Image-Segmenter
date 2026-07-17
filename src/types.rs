use egui::Pos2;

// <app mode>
#[derive(Clone, PartialEq, Debug)]
pub enum Mode {
    Idle,
    Ready,
    CalibP1,
    CalibP2 { p1: Pos2 },
    CalibLen { p1: Pos2, p2: Pos2 },
    Segmented,
}
// </app mode>

// <unit of measurement>
#[derive(Clone, PartialEq)]
pub enum Unit {
    Cm2,
    Mm2,
}

impl Unit {
    pub fn label(&self) -> &'static str {
        match self { Unit::Cm2 => "cm²", Unit::Mm2 => "mm²" }
    }
    pub fn factor(&self) -> f64 {
        match self { Unit::Cm2 => 1.0, Unit::Mm2 => 100.0 }
    }
}
// </unit of measurement>

// <segmentation engine choice>
// Exact: single threaded, seed based tolerance, identical to the original algorithm.
// Parallel: multi core CPU, tiled, seed based tolerance, seams merged via union find.
// Gpu: neighbor based (chained) tolerance, computed on the GPU. Different semantics
// from the seed based CPU engines, see comments in segment.rs / gpu.rs.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SegmentEngine {
    Exact,
    Parallel,
    Gpu,
}

impl SegmentEngine {
    pub fn label(&self) -> &'static str {
        match self {
            SegmentEngine::Exact => "Exact",
            SegmentEngine::Parallel => "Parallel",
            SegmentEngine::Gpu => "GPU",
        }
    }
}
// </segmentation engine choice>

// <region data>
#[derive(Clone)]
pub struct Region {
    pub index: usize,
    pub pixel_count: usize,
    pub area_cm2: f64,
    pub avg_color: [u8; 3],
    pub centroid: (f32, f32),
}
// </region data>