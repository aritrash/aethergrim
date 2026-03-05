#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod arch;
mod gui;
mod drivers;
mod allocator; 

use limine::FramebufferRequest;
use core::panic::PanicInfo;
use gui::utils::Renderer;
use gui::splash::draw_splash;
use core::arch::naked_asm;
use x86_64::VirtAddr;
use crate::arch::x86_64::process::{self, SPLASH_COMPLETE};
use core::sync::atomic::Ordering;
use crate::gui::compositor::Compositor;

pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new(0);

#[repr(C, align(16))]
struct Stack {
    data: [u8; 16384],
}

#[no_mangle]
#[link_section = ".bss"]
static mut BOOT_STACK: Stack = Stack { data: [0; 16384] };

#[no_mangle]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    unsafe {
        naked_asm!(
            "lea rsp, [rip + {stack} + 16384]",
            "and rsp, -16",
            "call {kernel_main}",
            "1: hlt",
            "jmp 1b",
            stack = sym BOOT_STACK,
            kernel_main = sym kernel_main,
        );
    }
}

#[link_section = ".bss"]
static mut BACKBUFFER: [u32; 1920 * 1080] = [0; 1920 * 1080];

extern "C" fn kernel_main() -> ! {
    crate::serial_println!("Aether Grim: Initiating Memory and Paging...");

    // 1. Core Hardware & SSE setup
    unsafe { init_fpu_sse(); }
    arch::x86_64::gdt::init(); 
    arch::x86_64::idt::init();
    arch::x86_64::timer::init(1000);

    // 2. Initialize Memory Management (PMM & VMM)
    arch::x86_64::memory::init(); 

    // 3. Initialize Global Heap Allocator
    let hhdm_offset = arch::x86_64::memory::get_hhdm_offset(); 
    let mut mapper = unsafe { arch::x86_64::memory::paging::init(VirtAddr::new(hhdm_offset)) };
    let mut frame_allocator = arch::x86_64::memory::paging::BootFrameAllocator;

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed");
    
    crate::serial_println!("Aether Grim: Dynamic Allocator Ready.");

    // 4. Initialize Input Drivers
    unsafe {
        drivers::legacy::keyboard::init();
        drivers::legacy::mouse::init();
    }
    
    crate::serial_println!("Input Systems Online.");

    // 5. Setup Graphics & Compositor
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if let Some(fb_ptr) = response.framebuffers().iter().next() {
            let fb = unsafe { &*fb_ptr.as_ptr() };
            let mut renderer = Renderer::new(fb, unsafe { BACKBUFFER.as_mut_ptr() });

            // Run Splash Screen
            unsafe {
                for p in 0..=100 {
                    draw_splash(&renderer, p as u64);
                }
                renderer.clear_screen(0x000D1117); 
                renderer.swap_buffers();
                SPLASH_COMPLETE.store(true, Ordering::SeqCst);
            }

            // Initialize the Compositor (Our UI Brain)
            let mut compositor = Compositor::new();

            crate::serial_println!("Aether Desktop Ready. Enabling Interrupts...");
            x86_64::instructions::interrupts::enable();

            unsafe { 
                renderer.clear_screen(0x000D1117);
                compositor.draw_icons(&renderer);
                renderer.swap_buffers(); } // PUSH THE ENTIRE DESKTOP ONCE

            loop {
                let (mx, my) = drivers::legacy::mouse::get_mouse_pos();
                let draw_x = mx as usize;
                let draw_y = my as usize;

                // 1. Capture the EXACT state of the last draw
                let last_x = renderer.last_cursor_x;
                let last_y = renderer.last_cursor_y;

                // 2. Render to BACKBUFFER
                compositor.render(&renderer);
                renderer.draw_cursor(draw_x, draw_y);

                // 3. SWAP THE BOUNDING BOX
                unsafe {
                    // Calculate a rectangle that encloses both old and new positions
                    let min_x = last_x.min(draw_x).saturating_sub(10);
                    let min_y = last_y.min(draw_y).saturating_sub(10);
                    let max_x = last_x.max(draw_x) + 40;
                    let max_y = last_y.max(draw_y) + 40;

                    let width = (max_x - min_x).min(1919);
                    let height = (max_y - min_y).min(1079);

                    renderer.swap_rect(min_x, min_y, width, height);
                }

                x86_64::instructions::hlt();
            }
        }
    }

    loop { x86_64::instructions::hlt(); }
}

pub unsafe fn init_fpu_sse() {
    use core::arch::asm;
    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0);
    cr0 &= !(1 << 2); 
    cr0 |= 1 << 1;     
    asm!("mov cr0, {}", in(reg) cr0);

    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4);
    cr4 |= 1 << 9;    
    cr4 |= 1 << 10;   
    asm!("mov cr4, {}", in(reg) cr4);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    crate::serial_println!("PANIC: {:?}", _info);
    loop { x86_64::instructions::hlt(); }
}