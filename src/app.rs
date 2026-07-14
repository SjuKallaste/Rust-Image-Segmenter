use egui::{Rect, TextureHandle};
use image::{DynamicImage, RgbImage};
use std::collections::HashSet;
use std::sync::mpsc;

use crate::color::{all_color_filters, ColorFilter};
use crate::types::{Mode, Region, Unit};

// <task state, what background work is currently running>
pub enum TaskKind {
    Loading,
    Segmenting,
    Filtering,
}

pub struct TaskResult {
    pub kind: TaskKind,
    pub payload: TaskPayload,
}

pub enum TaskPayload {
    Loaded {
        image: DynamicImage,
        rgb: RgbImage,
        prominent: Vec<usize>,
    },
    Segmented {
        labels: Vec<i32>,
        regions: Vec<Region>,
    },
    Filtered,
}
// </task state, what background work is currently running>

// <app state>
pub struct App {
    pub image: Option<DynamicImage>,
    pub rgb_cache: Option<RgbImage>,
    pub img_w: u32,
    pub img_h: u32,

    pub orig_tex: Option<TextureHandle>,
    pub seg_tex: Option<TextureHandle>,
    pub edge_tex: Option<TextureHandle>,
    pub color_filter_tex: Option<TextureHandle>,

    pub img_rect: Rect,
    pub show_seg: bool,
    pub show_edges: bool,

    pub mode: Mode,
    pub calib_len_buf: String,
    pub scale_px_per_cm: Option<f64>,

    pub tolerance: u32,
    pub min_pixels: usize,
    pub blur_radius: u32,

    pub label_map: Vec<i32>,
    pub regions: Vec<Region>,
    pub selected: HashSet<usize>,
    pub total_area_cm2: f64,
    pub unit: Unit,

    pub color_filters: Vec<ColorFilter>,
    pub active_color_filters: HashSet<usize>,
    pub prominent_filter_indices: Vec<usize>,
    pub show_all_colors: bool,

    pub imagej_mode: bool,
    pub imagej_hue_min: u8,
    pub imagej_hue_max: u8,
    pub imagej_sat_min: u8,
    pub imagej_sat_max: u8,
    pub imagej_bri_min: u8,
    pub imagej_bri_max: u8,

    pub gpu_ctx: Option<crate::gpu::GpuContext>,
    pub gpu_enabled: bool,
    pub gpu_available: bool,
    pub gpu_is_discrete: bool,

    pub use_parallel_segment: bool,

    // <background task channel>
    // None = idle, Some = work in progress
    pub task_rx: Option<mpsc::Receiver<TaskResult>>,
    pub task_label: Option<String>,
    // </background task channel>

    pub status: String,
}
// </app state>

// <default values>
impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            image: None,
            rgb_cache: None,
            img_w: 0,
            img_h: 0,
            orig_tex: None,
            seg_tex: None,
            edge_tex: None,
            color_filter_tex: None,
            img_rect: Rect::NOTHING,
            show_seg: false,
            show_edges: false,
            mode: Mode::Idle,
            calib_len_buf: String::new(),
            scale_px_per_cm: None,
            tolerance: 30,
            min_pixels: 200,
            blur_radius: 0,
            label_map: Vec::new(),
            regions: Vec::new(),
            selected: HashSet::new(),
            total_area_cm2: 0.0,
            unit: Unit::Cm2,
            color_filters: all_color_filters(),
            active_color_filters: HashSet::new(),
            prominent_filter_indices: Vec::new(),
            show_all_colors: false,
            imagej_mode: false,
            imagej_hue_min: 0,
            imagej_hue_max: 255,
            imagej_sat_min: 0,
            imagej_sat_max: 255,
            imagej_bri_min: 0,
            imagej_bri_max: 255,
            gpu_ctx: None,
            gpu_enabled: false,
            gpu_available: false,
            gpu_is_discrete: false,
            use_parallel_segment: true,
            task_rx: None,
            task_label: None,
            status: "Step 1: Load an image.".into(),
        };

        if let Some(ctx) = crate::gpu::try_init_gpu() {
            app.gpu_available = true;
            app.gpu_is_discrete = ctx.is_discrete;
            app.gpu_enabled = ctx.is_discrete;
            app.gpu_ctx = Some(ctx);
        }

        app
    }
}
// </default values>