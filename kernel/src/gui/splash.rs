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

    // 1. Initial Setup: Clear exactly once.
    if progress == 0 {
        renderer.clear_screen(slate_bg);
    }

    // 2. Phase 1: Logo Fade (0 to 15)
    // To kill flicker, we DO NOT clear the area. We rely on the fade math 
    // to blend against the existing slate_bg.
    if progress > 0 && progress <= 15 {
        let alpha = ((progress * 255) / 15) as u8;
        // We only draw every 3rd frame to let the bus breathe
        if progress % 3 == 0 {
            renderer.draw_image_faded(logo_x, logo_y, logo_w, logo_h, AEG_LOGO, alpha);
        }
    } 
    
    // 3. Phase 2: Progress Bar (16 to 100)
    else if progress > 15 {
        // Draw the solid logo EXACTLY ONCE
        if progress == 16 {
            renderer.draw_image(logo_x, logo_y, logo_w, logo_h, AEG_LOGO);
            // Draw the empty track once
            renderer.draw_rect(bar_x, bar_y, bar_w, bar_h, track_color);
        }

        let internal_p = ((progress - 16) * 100) / 84;
        let progress_w = (internal_p * bar_w) / 100;

        // NO CLEARING HERE. Just draw the blue bar. 
        // Since the bar only grows, it will naturally cover the track.
        if progress_w > 0 {
            renderer.draw_rect(bar_x, bar_y, progress_w, bar_h, accent_blue);
        }
    }
}