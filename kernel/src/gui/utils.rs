// kernel/src/gui/utils.rs

pub struct Renderer<'a> {
    pub fb: &'a limine::Framebuffer,
}

impl<'a> Renderer<'a> {
    pub fn new(fb: &'a limine::Framebuffer) -> Self {
        Self { fb }
    }

    pub fn width(&self) -> u64 { self.fb.width }
    pub fn height(&self) -> u64 { self.fb.height }

    fn blend_pixels(&self, src: u32, dst: u32, alpha: u8) -> u32 {
        let a = alpha as u16;
        let inv_a = 255 - a;

        let r = (((src >> 16 & 0xFF) as u16 * a + (dst >> 16 & 0xFF) as u16 * inv_a) / 255) as u32;
        let g = (((src >> 8 & 0xFF) as u16 * a + (dst >> 8 & 0xFF) as u16 * inv_a) / 255) as u32;
        let b = (((src & 0xFF) as u16 * a + (dst & 0xFF) as u16 * inv_a) / 255) as u32;

        (r << 16) | (g << 8) | b
    }

    pub unsafe fn draw_rect(&self, x: u64, y: u64, width: u64, height: u64, color: u32) {
        let pixel_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch / 4;

        for dy in 0..height {
            let row_offset = (y + dy) * stride;
            for dx in 0..width {
                pixel_ptr.add((row_offset + x + dx) as usize).write_volatile(color);
            }
        }
    }

    pub unsafe fn clear_screen(&self, color: u32) {
        let pixel_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch / 4;
        
        for y in 0..self.fb.height {
            let row_offset = y * stride;
            for x in 0..self.fb.width {
                pixel_ptr.add((row_offset + x) as usize).write_volatile(color);
            }
        }
    }

    pub unsafe fn draw_image(&self, x: u64, y: u64, width: u64, height: u64, data: &[u8]) {
        let pixel_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch / 4;
        let data_ptr = data.as_ptr() as *const u32;

        for dy in 0..height {
            let row_offset = (y + dy) * stride;
            for dx in 0..width {
                let src_pixel = data_ptr.add((dy * width + dx) as usize).read_volatile();
                let alpha = (src_pixel >> 24) as u8;

                if alpha == 255 {
                    pixel_ptr.add((row_offset + x + dx) as usize).write_volatile(src_pixel);
                } else if alpha > 0 {
                    let dst_ptr = pixel_ptr.add((row_offset + x + dx) as usize);
                    let bg_pixel = dst_ptr.read_volatile();
                    dst_ptr.write_volatile(self.blend_pixels(src_pixel, bg_pixel, alpha));
                }
            }
        }
    }

    pub unsafe fn draw_image_faded(&self, x: u64, y: u64, width: u64, height: u64, data: &[u8], global_alpha: u8) {
        let pixel_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch / 4;
        let data_ptr = data.as_ptr() as *const u32;

        for dy in 0..height {
            let row_offset = (y + dy) * stride;
            for dx in 0..width {
                let src_pixel = data_ptr.add((dy * width + dx) as usize).read_volatile();
                let original_alpha = (src_pixel >> 24) as u8;
                let combined_alpha = ((original_alpha as u16 * global_alpha as u16) / 255) as u8;

                if combined_alpha > 0 {
                    let dst_ptr = pixel_ptr.add((row_offset + x + dx) as usize);
                    let bg_pixel = dst_ptr.read_volatile();
                    dst_ptr.write_volatile(self.blend_pixels(src_pixel, bg_pixel, combined_alpha));
                }
            }
        }
    }
}