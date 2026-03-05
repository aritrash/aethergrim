use alloc::string::String;
use alloc::vec::Vec;
use crate::gui::utils::Renderer;

pub struct GrimBox {
    history: Vec<String>,
    current_line: String,
    cursor_x: usize,
    cursor_y: usize,
    width: usize,
    height: usize,
}

impl GrimBox {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            history: Vec::new(),
            current_line: String::new(),
            cursor_x: 20, // Padding from left
            cursor_y: 20, // Padding from top
            width,
            height,
        }
    }

    pub fn push_char(&mut self, c: char, renderer: &Renderer) {
        if c == '\n' {
            self.history.push(self.current_line.clone());
            self.current_line.clear();
            self.cursor_x = 20;
            self.cursor_y += 12; // Line height
        } else {
            renderer.draw_char(self.cursor_x, self.cursor_y, c, 0xFFFFFF);
            self.current_line.push(c);
            self.cursor_x += 8;
        }
        
        // Performance: Only swap the area where the character was drawn
        // (If you've implemented partial swapping)
        unsafe {
            renderer.swap_buffers();
        }
        
    }
}