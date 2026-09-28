use eframe::egui::*;
use eframe::wgpu::naga::compact::compact;

pub fn apply(ctx: &Context, compact: bool, dark: bool) {
    ctx.set_visuals(if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    });

    ctx.global_style_mut(|style| {
        style.compact_menu_style = compact;
        style.spacing.item_spacing = if compact {
            vec2(5.0, 6.0)
        } else {
            vec2(10.0, 8.0)
        };

        style.spacing.button_padding = if compact { vec2(5.0, 5.0) } else { vec2(10.0, 6.0) };
        let base_radius = if compact { 3 } else { 5 };
        style.spacing.window_margin = Margin::same(if compact {5} else {14});

        style.visuals.window_corner_radius = CornerRadius::same(base_radius+4);
        style.visuals.menu_corner_radius = CornerRadius::same(base_radius+2);
        style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(base_radius);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(base_radius);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(base_radius);
        style.visuals.widgets.active.corner_radius = CornerRadius::same(base_radius);
    });
}
pub fn section_frame(style: &Style) -> Frame {
    Frame::group(style)
        .inner_margin(if style.compact_menu_style { 4.0 } else { 14.0 })
        .outer_margin(0.0)
}
