// kernel/src/gui/utils.rs
use core::ptr;
use crate::arch::x86_64::idt::FONT_8X8;
use crate::gui::assets::cursor;
use alloc::boxed::Box;

pub struct Renderer<'a> {
    pub fb: &'a limine::Framebuffer,
    pub buffer: *mut u32, 
    pub cursor_backup: Box<[u32; cursor::CURSOR_WIDTH * cursor::CURSOR_HEIGHT]>,
    pub last_cursor_x: usize,
    pub last_cursor_y: usize,
}

impl<'a> Renderer<'a> {
    pub fn new(fb: &'a limine::Framebuffer, buffer: *mut u32) -> Self {
        Self { fb, 
            buffer,
            cursor_backup: Box::new([0; cursor::CURSOR_WIDTH * cursor::CURSOR_HEIGHT]),
            last_cursor_x: 0,
            last_cursor_y: 0,
        }
    }

    pub fn width(&self) -> u64 { self.fb.width }
    pub fn height(&self) -> u64 { self.fb.height }

    fn blend_pixels(&self, src: u32, dst: u32, alpha: u8) -> u32 {
        let a = alpha as u16;
        let inv_a = 255 - a;
        let r = (((src >> 16 & 0xFF) as u16 * a + (dst >> 16 & 0xFF) as u16 * inv_a) >> 8) as u32;
        let g = (((src >> 8 & 0xFF) as u16 * a + (dst >> 8 & 0xFF) as u16 * inv_a) >> 8) as u32;
        let b = (((src & 0xFF) as u16 * a + (dst & 0xFF) as u16 * inv_a) >> 8) as u32;
        (r << 16) | (g << 8) | b
    }

    pub unsafe fn swap_buffers(&self) {
        let front_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let pixel_count = (self.fb.width * self.fb.height) as usize;
        core::ptr::copy_nonoverlapping(self.buffer, front_ptr, pixel_count);
    }

    pub unsafe fn clear_screen(&self, color: u32) {
        let pixel_count = (self.fb.width * self.fb.height) as usize;
        let ptr = self.buffer;
        for i in 0..pixel_count {
            ptr.add(i).write_volatile(color);
        }
    }

    pub fn draw_rect(&self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        for row in 0..height {
            for col in 0..width {
                self.put_pixel(x + col, y + row, color);
            }
        }
    }

    pub fn draw_string(&self, x: usize, y: usize, s: &str, color: u32) {
        let mut offset = 0;
        for c in s.chars() {
            self.draw_char(x + offset, y, c, color);
            offset += 8;
        }
    }

    pub unsafe fn draw_image(&self, x: u64, y: u64, width: u64, height: u64, data: &[u8]) {
        let data_ptr = data.as_ptr() as *const u32;
        for dy in 0..height {
            for dx in 0..width {
                let src_pixel = data_ptr.add((dy * width + dx) as usize).read_volatile();
                let alpha = (src_pixel >> 24) as u8;
                if alpha > 0 {
                    self.put_pixel_alpha((x + dx) as usize, (y + dy) as usize, src_pixel, alpha);
                }
            }
        }
    }

    pub unsafe fn draw_image_faded(&self, x: u64, y: u64, width: u64, height: u64, data: &[u8], global_alpha: u8) {
        let data_ptr = data.as_ptr() as *const u32;
        for dy in 0..height {
            for dx in 0..width {
                let src_pixel = data_ptr.add((dy * width + dx) as usize).read_volatile();
                let original_alpha = (src_pixel >> 24) as u8;
                let combined_alpha = ((original_alpha as u16 * global_alpha as u16) / 255) as u8;
                if combined_alpha > 0 {
                    self.put_pixel_alpha((x + dx) as usize, (y + dy) as usize, src_pixel, combined_alpha);
                }
            }
        }
    }

    /// Primary safe pixel function
    pub fn put_pixel(&self, x: usize, y: usize, color: u32) {
        if x >= self.fb.width as usize || y >= self.fb.height as usize {
            return;
        }
        let stride = self.fb.pitch as usize / 4;
        let offset = (y * stride) + x;
        unsafe {
            self.buffer.add(offset).write_volatile(color);
        }
    }

    /// Safe pixel function with alpha blending
    pub fn put_pixel_alpha(&self, x: usize, y: usize, color: u32, alpha: u8) {
        if x >= self.fb.width as usize || y >= self.fb.height as usize {
            return;
        }
        let stride = self.fb.pitch as usize / 4;
        let offset = (y * stride) + x;
        unsafe {
            let dst_ptr = self.buffer.add(offset);
            if alpha == 255 {
                dst_ptr.write_volatile(color);
            } else {
                let bg_pixel = dst_ptr.read_volatile();
                dst_ptr.write_volatile(self.blend_pixels(color, bg_pixel, alpha));
            }
        }
    }

    pub fn draw_char(&self, x: usize, y: usize, c: char, color: u32) {
        let index = match c {
            '0'..='9' => (c as usize - '0' as usize),
            'a'..='f' => (c as usize - 'a' as usize + 10),
            _ => return, 
        };

        if index >= FONT_8X8.len() { return; }

        let glyph = FONT_8X8[index];
        for row in 0..8 {
            let row_data = glyph[row];
            for col in 0..8 {
                if (row_data >> (7 - col)) & 1 == 1 {
                    self.put_pixel(x + col, y + row, color);
                }
            }
        }
    }

    pub fn draw_cursor(&mut self, x: usize, y: usize) {
        self.restore_background();
        self.last_cursor_x = x;
        self.last_cursor_y = y;
        self.save_background(x, y);

        for row in 0..cursor::CURSOR_HEIGHT {
            for col in 0..cursor::CURSOR_WIDTH {
                let sprite_pixel = cursor::SPRITE[row * cursor::CURSOR_WIDTH + col];
                let color = match sprite_pixel {
                    1 => 0x000000,
                    2 => 0xFFFFFF,
                    _ => continue,
                };
                self.put_pixel(x + col, y + row, color);
            }
        }
    }

    fn save_background(&mut self, x: usize, y: usize) {
        let stride = self.fb.pitch as usize / 4;
        for row in 0..cursor::CURSOR_HEIGHT {
            for col in 0..cursor::CURSOR_WIDTH {
                let target_x = x + col;
                let target_y = y + row;
                
                // If the cursor is partially off-screen, we save 0 for those pixels
                if target_x < self.fb.width as usize && target_y < self.fb.height as usize {
                    unsafe {
                        let offset = (target_y * stride) + target_x;
                        self.cursor_backup[row * cursor::CURSOR_WIDTH + col] = self.buffer.add(offset).read_volatile();
                    }
                } else {
                    self.cursor_backup[row * cursor::CURSOR_WIDTH + col] = 0;
                }
            }
        }
    }

    fn restore_background(&self) {
        let stride = self.fb.pitch as usize / 4;
        for row in 0..cursor::CURSOR_HEIGHT {
            for col in 0..cursor::CURSOR_WIDTH {
                let target_x = self.last_cursor_x + col;
                let target_y = self.last_cursor_y + row;
                
                // Only restore if within bounds
                if target_x < self.fb.width as usize && target_y < self.fb.height as usize {
                    unsafe {
                        let offset = (target_y * stride) + target_x;
                        let bg_color = self.cursor_backup[row * cursor::CURSOR_WIDTH + col];
                        self.buffer.add(offset).write_volatile(bg_color);
                    }
                }
            }
        }
    }

    pub unsafe fn swap_rect(&self, x: usize, y: usize, w: usize, h: usize) {
        let fb_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch as usize / 4;

        for row in y..(y + h) {
            // Safety: Don't draw past screen height
            if row >= self.fb.height as usize { break; }
            
            // Safety: Don't draw past screen width
            let row_width = if x + w > self.fb.width as usize {
                (self.fb.width as usize).saturating_sub(x)
            } else {
                w
            };

            if row_width == 0 { continue; }

            let offset = (row * stride) + x;
            core::ptr::copy_nonoverlapping(
                self.buffer.add(offset),
                fb_ptr.add(offset),
                row_width
            );
        }
    }
}