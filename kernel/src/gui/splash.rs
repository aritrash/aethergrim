// kernel/src/gui/splash.rs
use crate::gui::utils::Renderer;

static AEG_LOGO: &[u8] = include_bytes!("../../assets/aeg_logo.raw");

pub unsafe fn draw_splash(renderer: &Renderer, progress: u64) {
    let slate_bg = 0x000D1117;
    let accent_blue = 0x0058A6FF;
    let track_color = 0x001C2533;
    
    let logo_w = 347; 
    let logo_h = 180;
    let logo_x = (renderer.width() - logo_w) / 2;
    let logo_y = (renderer.height() / 2) - logo_h - 20;

    let bar_w = 300;
    let bar_h = 4;
    let bar_x = (renderer.width() - bar_w) / 2;
    let bar_y = (renderer.height() / 2) + 40;

    // 1. Initial full-screen clear only at the very start
    if progress == 0 {
        renderer.clear_screen(slate_bg);
    }

    // 2. Clear only the ACTIVE areas (Logo and Bar)
    // This is the "Dirty Rectangle" technique.
    renderer.draw_rect(logo_x, logo_y, logo_w, logo_h, slate_bg);
    renderer.draw_rect(bar_x, bar_y, bar_w, bar_h, slate_bg);

    if progress <= 30 {
        let alpha = ((progress * 255) / 30) as u8;
        renderer.draw_image_faded(logo_x, logo_y, logo_w, logo_h, AEG_LOGO, alpha);
    } else {
        renderer.draw_image(logo_x, logo_y, logo_w, logo_h, AEG_LOGO);
        
        renderer.draw_rect(bar_x, bar_y, bar_w, bar_h, track_color);
        let progress_w = ((progress - 30) * bar_w) / 70;
        if progress_w > 0 {
            renderer.draw_rect(bar_x, bar_y, progress_w, bar_h, accent_blue);
        }
    }

    // 3. Blit to front buffer
    renderer.swap_buffers();
}