#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod arch;
mod gui;

use limine::FramebufferRequest;
use core::panic::PanicInfo;
use gui::utils::Renderer;
use gui::splash::draw_splash;
use core::arch::naked_asm;

// Macro for easy serial logging
#[macro_use]
mod serial {
    pub use crate::arch::x86_64::serial::_print;
}

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

extern "C" fn kernel_main() -> ! {
    // 1. Initial Hardware Setup
    // Note: serial_println! macro must be imported or available here
    crate::serial_println!("Aether Grim: Booting...");

    unsafe { init_fpu_sse(); }
    crate::serial_println!("SSE Initialized.");

    arch::x86_64::gdt::init(); 
    crate::serial_println!("GDT/TSS Loaded.");

    arch::x86_64::idt::init();
    // IDT Loaded log is inside idt::init()

    // 2. Graphics Initialization
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if let Some(fb_ptr) = response.framebuffers().iter().next() {
            let fb = unsafe { &*fb_ptr.as_ptr() };
            let renderer = Renderer::new(fb);

            // Enable interrupts to allow the timer for sleep() to work
            x86_64::instructions::interrupts::enable();

            // Inside kernel_main in main.rs
            unsafe {
                // Single global clear
                renderer.clear_screen(0x000D1117);
                
                // Smooth loop
                for p in 0..=100 {
                    draw_splash(&renderer, p as u64);
                    
                    // Increase to 2 or 3 if your emulator is struggling with bus speed
                    sleep(1, &renderer); 
                }

                // Hold the finished state
                sleep(50, &renderer);

                // Final wipe to OS state
                renderer.clear_screen(0x000D1117);
                
                crate::serial_println!("Transition to Slate complete.");
            }
        }
    } else {
        // Fallback if no framebuffer is found
        crate::serial_println!("Error: Framebuffer request failed.");
    }

    // 5. Final Kernel Idle State
    loop {
        x86_64::instructions::hlt();
    }
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

fn sleep(ticks: u64, _renderer: &Renderer) {
    let start_time = arch::x86_64::timer::get_ticks();
    while arch::x86_64::timer::get_ticks() < start_time + ticks {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    crate::serial_println!("PANIC: {:?}", _info);
    loop { x86_64::instructions::hlt(); }
}