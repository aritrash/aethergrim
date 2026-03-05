#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod arch;
mod gui;

use limine::FramebufferRequest;
use core::panic::PanicInfo;
use gui::utils::Renderer;
use gui::splash::draw_splash;

// The Limine Framebuffer Request with revision 0
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new(0);

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe { init_fpu_sse(); }
    arch::x86_64::init();

    if let Some(response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if let Some(fb_ptr) = response.framebuffers().iter().next() {
            let fb = unsafe { &*fb_ptr.as_ptr() };
            let renderer = Renderer::new(fb);

            unsafe {
                // 10% -> Wait 1s
                draw_splash(&renderer, 10);
                sleep_pseudo(1);

                // 30% -> Wait 2s
                draw_splash(&renderer, 30);
                sleep_pseudo(2);

                // 65% -> Wait 1s
                draw_splash(&renderer, 65);
                sleep_pseudo(1);

                // 85% -> Wait 0.5s
                draw_splash(&renderer, 85);
                sleep_pseudo(1); // Slightly shorter loop in real life

                // 100% -> Finish
                draw_splash(&renderer, 100);
                
                // Final transition to Slate
                renderer.clear_screen(0x000D1117);
            }
        }
    }

    loop { x86_64::instructions::hlt(); }
}

/// A very rough pseudo-sleep function using spin loops
unsafe fn sleep_pseudo(seconds: u64) {
    for _ in 0..(seconds * 1_000_000) {
        core::hint::spin_loop();
    }
}

/// Enables SSE/FPU to satisfy the x86_64-aether ABI requirements
pub unsafe fn init_fpu_sse() {
    use core::arch::asm;
    
    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0);
    cr0 &= !(1 << 2); // Clear EM (Emulation)
    cr0 |= 1 << 1;    // Set MP (Monitor Coprocessor)
    asm!("mov cr0, {}", in(reg) cr0);

    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4);
    cr4 |= 1 << 9;    // Set OSFXSR (FXSAVE/FXRSTOR support)
    cr4 |= 1 << 10;   // Set OSXMMEXCPT (Unmasked Exception support)
    asm!("mov cr4, {}", in(reg) cr4);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}