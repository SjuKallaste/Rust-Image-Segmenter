use egui::{Color32, Rounding, Stroke};

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    let bg_window = Color32::from_rgb(0x1F, 0x1F, 0x1F);
    let bg_panel = Color32::from_rgb(0x2B, 0x2B, 0x2B);
    let bg_widget = Color32::from_rgb(0x33, 0x33, 0x36);
    let bg_widget_hover = Color32::from_rgb(0x3E, 0x3E, 0x42);
    let bg_widget_active = Color32::from_rgb(0x00, 0x78, 0xD7); // classic win10 blue
    let border = Color32::from_rgb(0x3F, 0x3F, 0x46);
    let text = Color32::from_rgb(0xE8, 0xE8, 0xE8);

    let rounding = Rounding::same(2.0);

    visuals.window_fill = bg_window;
    visuals.window_stroke = Stroke::new(1.0, border);
    visuals.window_rounding = rounding;
    visuals.panel_fill = bg_panel;
    visuals.extreme_bg_color = Color32::from_rgb(0x18, 0x18, 0x18);
    visuals.faint_bg_color = bg_widget;

    visuals.widgets.noninteractive.bg_fill = bg_panel;
    visuals.widgets.noninteractive.weak_bg_fill = bg_panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.noninteractive.rounding = rounding;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);

    visuals.widgets.inactive.bg_fill = bg_widget;
    visuals.widgets.inactive.weak_bg_fill = bg_widget;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.inactive.rounding = rounding;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);

    visuals.widgets.hovered.bg_fill = bg_widget_hover;
    visuals.widgets.hovered.weak_bg_fill = bg_widget_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, bg_widget_active);
    visuals.widgets.hovered.rounding = rounding;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text);

    visuals.widgets.active.bg_fill = bg_widget_active;
    visuals.widgets.active.weak_bg_fill = bg_widget_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, bg_widget_active);
    visuals.widgets.active.rounding = rounding;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.open.bg_fill = bg_widget_hover;
    visuals.widgets.open.weak_bg_fill = bg_widget_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.open.rounding = rounding;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, text);

    visuals.selection.bg_fill = bg_widget_active;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.hyperlink_color = Color32::from_rgb(0x4C, 0xC2, 0xFF);
    visuals.override_text_color = None;

    ctx.set_visuals(visuals);
}