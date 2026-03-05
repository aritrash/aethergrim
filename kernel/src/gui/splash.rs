use crate::gui::utils::Renderer;

pub unsafe fn draw_splash(renderer: &Renderer, progress: u64) {
    let slate_bg = 0x000D1117;
    let accent_blue = 0x0058A6FF;
    let track_color = 0x001C2533;
    
    // Bar dimensions
    let bar_w = 200;
    let bar_h = 4;
    let x = (renderer.fb.width - bar_w) / 2;
    let y = (renderer.fb.height - bar_h) / 2;

    // 1. Only clear the screen if we are at the very start (0 or 10%)
    // This prevents flickering during the update
    if progress <= 10 {
        renderer.clear_screen(slate_bg);
    }

    // 2. Draw/Refresh the track (background of the bar)
    renderer.draw_rect(x, y, bar_w, bar_h, track_color);
    
    // 3. Calculate and draw the "Progress" portion
    // (progress / 100) * bar_w
    let progress_w = (progress * bar_w) / 100;
    renderer.draw_rect(x, y, progress_w, bar_h, accent_blue);
}