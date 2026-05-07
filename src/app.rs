use egui::{Rect, TextureHandle};
use image::DynamicImage;
use std::collections::HashSet;

use crate::color::{all_color_filters, compute_prominent_filters, ColorFilter};
use crate::types::{Mode, Region, Unit};

// <app state>
pub struct App {
    pub image: Option<DynamicImage>,
    pub img_w: u32,
    pub img_h: u32,

    // <textures>
    pub orig_tex: Option<TextureHandle>,
    pub seg_tex: Option<TextureHandle>,
    pub edge_tex: Option<TextureHandle>,
    pub color_filter_tex: Option<TextureHandle>,
    // </textures>

    pub img_rect: Rect,
    pub show_seg: bool,
    pub show_edges: bool,

    // <calibration>
    pub mode: Mode,
    pub calib_len_buf: String,
    pub scale_px_per_cm: Option<f64>,
    // </calibration>

    // <segmentation params>
    pub tolerance: u32,
    pub min_pixels: usize,
    pub blur_radius: u32,
    // </segmentation params>

    // <segmentation results>
    pub label_map: Vec<i32>,
    pub regions: Vec<Region>,
    pub selected: HashSet<usize>,
    pub total_area_cm2: f64,
    pub unit: Unit,
    // </segmentation results>

    // <color filter panel>
    pub color_filters: Vec<ColorFilter>,
    pub active_color_filters: HashSet<usize>,
    pub prominent_filter_indices: Vec<usize>,
    pub show_all_colors: bool,
    // </color filter panel>

    // <imagej hsb mode>
    pub imagej_mode: bool,
    pub imagej_hue_min: u8,
    pub imagej_hue_max: u8,
    pub imagej_sat_min: u8,
    pub imagej_sat_max: u8,
    pub imagej_bri_min: u8,
    pub imagej_bri_max: u8,
    // </imagej hsb mode>

    pub status: String,
}
// </app state>

// <default values>
impl Default for App {
    fn default() -> Self {
        Self {
            image: None,
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
            status: "Step 1: Load an image.".into(),
        }
    }
}
// </default values>
