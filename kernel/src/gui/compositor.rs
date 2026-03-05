// kernel/src/gui/compositor.rs
use crate::gui::utils::Renderer;
use crate::gui::window::Window;
use crate::drivers::legacy::keyboard;
use crate::drivers::legacy::mouse; // Import mouse directly
use alloc::vec::Vec;

static mut WAS_PRESSED: bool = false;

pub struct Compositor {
    pub windows: Vec<Window>,
    pub background_color: u32,
    pub icon_x: usize,
    pub icon_y: usize,
}

impl Compositor {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            background_color: 0x000D1117, // Slate
            icon_x: 50,
            icon_y: 50,
        }
    }

    pub fn render(&mut self, renderer: &Renderer) {
        // Draw windows from back to front
        for window in self.windows.iter() {
            window.draw(renderer);
        }
    }

    pub fn draw_icons(&self, renderer: &Renderer) {
        renderer.draw_rect(self.icon_x, self.icon_y, 64, 64, 0x222222);
        renderer.draw_string(self.icon_x + 5, self.icon_y + 25, "GrimBox", 0xFFFFFF);
    }

    pub fn handle_click(&mut self, mx: i32, my: i32) {
        let is_pressed = mouse::is_left_pressed();
        
        unsafe {
            if is_pressed && !WAS_PRESSED {
                WAS_PRESSED = true;
                
                let mut clicked_window_index = None;

                // Check windows from front to back
                for (i, window) in self.windows.iter().enumerate().rev() {
                    if window.is_mouse_over(mx, my) {
                        clicked_window_index = Some(i);
                        break;
                    }
                }

                if let Some(index) = clicked_window_index {
                    let win = self.windows.remove(index);
                    self.windows.push(win);
                } else {
                    // Check desktop icon
                    if mx >= self.icon_x as i32 && mx <= (self.icon_x + 64) as i32 &&
                       my >= self.icon_y as i32 && my <= (self.icon_y + 64) as i32 {
                        self.open_grimbox();
                    }
                }
            } else if !is_pressed {
                WAS_PRESSED = false;
            }
        }
    }

    pub fn handle_keyboard(&mut self, scancode: u8) {
        if let Some(focused_window) = self.windows.last_mut() {
            if let Some(c) = keyboard::scancode_to_ascii(scancode) {
                focused_window.on_key(c);
            }
        }
    }

    fn open_grimbox(&mut self) {
        if !self.windows.iter().any(|w| w.title == "GrimBox Terminal") {
            let mut win = Window::new(400, 300, 600, 400, "GrimBox Terminal");
            win.is_visible = true;
            self.windows.push(win);
        }
    }
}