// kernel/src/gui/window.rs
use crate::gui::utils::Renderer;
use alloc::string::String;

pub struct Window {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub title: &'static str,
    pub is_visible: bool,
    pub buffer: String, // Internal terminal text buffer
}

impl Window {
    pub fn new(x: usize, y: usize, w: usize, h: usize, title: &'static str) -> Self {
        Self {
            x, y, width: w, height: h,
            title,
            is_visible: false,
            buffer: String::new(),
        }
    }

    pub fn draw(&self, renderer: &Renderer) {
        if !self.is_visible { return; }

        // Window Frame & Title Bar
        renderer.draw_rect(self.x, self.y - 25, self.width, 25, 0x333333); // Header
        renderer.draw_string(self.x + 10, self.y - 18, self.title, 0xFFFFFF);
        
        // Terminal Content Area
        renderer.draw_rect(self.x, self.y, self.width, self.height, 0x000000);

        // Draw the text from the buffer
        // (Simple implementation: just draw one line for now)
        renderer.draw_string(self.x + 5, self.y + 5, &self.buffer, 0x00FF00);
    }

    pub fn on_key(&mut self, c: char) {
        if c == '\x08' { // Backspace
            self.buffer.pop();
        } else if self.buffer.len() < 100 { // Simple limit
            self.buffer.push(c);
        }
    }

    pub fn is_mouse_over(&self, mx: i32, my: i32) -> bool {
        // Includes title bar in the hit detection
        mx >= self.x as i32 && mx <= (self.x + self.width) as i32 &&
        my >= (self.y - 25) as i32 && my <= (self.y + self.height) as i32
    }
}