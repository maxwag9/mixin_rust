use eframe::egui::*;

pub fn apply(ctx: &Context, compact: bool, dark: bool) {
    ctx.set_visuals(if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    });

    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = if compact {
            vec2(7.0, 5.0)
        } else {
            vec2(10.0, 8.0)
        };

        style.spacing.button_padding = vec2(12.0, 7.0);
        style.spacing.window_margin = Margin::same(16);

        style.visuals.window_corner_radius = CornerRadius::same(10);
        style.visuals.menu_corner_radius = CornerRadius::same(8);
        style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        style.visuals.widgets.active.corner_radius = CornerRadius::same(6);
    });
}
pub fn section_frame(style: &Style) -> Frame {
    Frame::group(style)
        .inner_margin(14.0)
        .outer_margin(0.0)
}
