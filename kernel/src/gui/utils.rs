// kernel/src/gui/utils.rs
use core::ptr;

pub struct Renderer<'a> {
    pub fb: &'a limine::Framebuffer,
    pub buffer: *mut u32,
}

impl<'a> Renderer<'a> {
    pub fn new(fb: &'a limine::Framebuffer, buffer: *mut u32) -> Self {
        Self { fb, buffer }
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
        
        // This is significantly faster than a manual loop.
        // It bypasses the overhead of incrementing loop counters in Rust.
        core::ptr::copy_nonoverlapping(self.buffer, front_ptr, pixel_count);
    }

    pub unsafe fn clear_screen(&self, color: u32) {
        let pixel_count = (self.fb.width * self.fb.height) as usize;
        // Optimization: If clearing to black, use write_bytes. 
        // For other colors, we use a tighter loop.
        let ptr = self.buffer;
        for i in 0..pixel_count {
            ptr.add(i).write_volatile(color);
        }
    }

    pub unsafe fn draw_rect(&self, x: u64, y: u64, width: u64, height: u64, color: u32) {
        let stride = self.fb.width;
        for dy in 0..height {
            let row_offset = (y + dy) * stride;
            for dx in 0..width {
                self.buffer.add((row_offset + x + dx) as usize).write_volatile(color);
            }
        }
    }

    pub unsafe fn draw_image(&self, x: u64, y: u64, width: u64, height: u64, data: &[u8]) {
        let stride = self.fb.width;
        let data_ptr = data.as_ptr() as *const u32;
        for dy in 0..height {
            let row_offset = (y + dy) * stride;
            for dx in 0..width {
                let src_pixel = data_ptr.add((dy * width + dx) as usize).read_volatile();
                let alpha = (src_pixel >> 24) as u8;
                let offset = (row_offset + x + dx) as usize;
                if alpha == 255 {
                    self.buffer.add(offset).write_volatile(src_pixel);
                } else if alpha > 0 {
                    let dst_ptr = self.buffer.add(offset);
                    let bg_pixel = dst_ptr.read_volatile();
                    dst_ptr.write_volatile(self.blend_pixels(src_pixel, bg_pixel, alpha));
                }
            }
        }
    }

    // RESTORED: draw_image_faded
    pub unsafe fn draw_image_faded(&self, x: u64, y: u64, width: u64, height: u64, data: &[u8], global_alpha: u8) {
        let stride = self.fb.width;
        let data_ptr = data.as_ptr() as *const u32;
        for dy in 0..height {
            let row_offset = (y + dy) * stride;
            for dx in 0..width {
                let src_pixel = data_ptr.add((dy * width + dx) as usize).read_volatile();
                let original_alpha = (src_pixel >> 24) as u8;
                let combined_alpha = ((original_alpha as u16 * global_alpha as u16) / 255) as u8;
                if combined_alpha > 0 {
                    let offset = (row_offset + x + dx) as usize;
                    let dst_ptr = self.buffer.add(offset);
                    let bg_pixel = dst_ptr.read_volatile();
                    dst_ptr.write_volatile(self.blend_pixels(src_pixel, bg_pixel, combined_alpha));
                }
            }
        }
    }
}