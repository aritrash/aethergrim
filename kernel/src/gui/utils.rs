pub struct Renderer<'a> {
    pub fb: &'a limine::Framebuffer,
}

impl<'a> Renderer<'a> {
    pub fn new(fb: &'a limine::Framebuffer) -> Self {
        Self { fb }
    }

    pub unsafe fn draw_rect(&self, x: u64, y: u64, width: u64, height: u64, color: u32) {
        let pixel_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch / 4;

        for dy in 0..height {
            for dx in 0..width {
                let offset = ((y + dy) * stride) + (x + dx);
                pixel_ptr.add(offset as usize).write_volatile(color);
            }
        }
    }

    pub unsafe fn clear_screen(&self, color: u32) {
        let pixel_ptr = self.fb.address.as_ptr().unwrap() as *mut u32;
        let stride = self.fb.pitch / 4;
        
        for y in 0..self.fb.height {
            for x in 0..self.fb.width {
                let offset = (y * stride) + x;
                pixel_ptr.add(offset as usize).write_volatile(color);
            }
        }
    }
}