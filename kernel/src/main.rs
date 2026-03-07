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
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::{Mapper, Page, PageTableFlags as Flags, PhysFrame, Size4KiB};
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

    // 4. PCI Enumeration & Hardware Discovery
    crate::serial_println!("--- PCI SCAN START ---");
    let pci_devices = drivers::pci::scan_bus();
    
    if pci_devices.is_empty() {
        crate::serial_println!("WARNING: No PCI devices detected. Check Port I/O logic.");
    }

    for dev in pci_devices {
        let bar0 = dev.get_bar0();
        crate::serial_println!(
            "[PCI] Bus {:02x}:Dev {:02x} | ID {:04x}:{:04x} | Class {:02x}:{:02x}:{:02x} | BAR0: {:#x}",
            dev.bus, dev.slot, dev.vendor_id, dev.device_id, dev.class, dev.subclass, dev.prog_if, bar0
        );

        // Specific check for xHCI (USB 3.0)
        if dev.class == 0x0C && dev.subclass == 0x03 && dev.prog_if == 0x30 {
            let bar0 = dev.get_bar0() as usize;
            let hhdm = arch::x86_64::memory::get_hhdm_offset() as usize;

            crate::serial_println!(">>> xHCI Detected at {:#x}", bar0);

            // 1. Manually allocate a physical frame for the driver data
            // This is safer than using the heap for 4096-aligned structures
            use x86_64::structures::paging::FrameAllocator;
            let frame = frame_allocator.allocate_frame().expect("No physical memory for xHCI");
            let data_virt_ptr = (frame.start_address().as_u64() as usize + hhdm) as *mut drivers::usb::xhci::XhciData;

            unsafe {
                let mut xhci = drivers::usb::xhci::XhciController::new(bar0, hhdm, data_virt_ptr);
                xhci.reset();
                xhci.init_rings();
                xhci.enable();
                // xhci.probe_ports(); // Add this back if you want to see connected devices
            }
        }
            crate::serial_println!("--- PCI SCAN END ---");
    }

    // 5. Initialize Legacy Input Drivers (Fallback)
    unsafe {
        drivers::legacy::keyboard::init();
        drivers::legacy::mouse::init();
        drivers::legacy::mouse::reset();
    }
    
    crate::serial_println!("Input Systems Online.");

    // 6. Setup Graphics & Compositor
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if let Some(fb_ptr) = response.framebuffers().iter().next() {
            let fb = unsafe { &*fb_ptr.as_ptr() };

            crate::serial_println!("Display: {}x{} (Pitch: {} bytes)", fb.width, fb.height, fb.pitch);

            // Safety check: If we got something other than 1920x1080, 
            // our static BACKBUFFER will overflow or mismatch.
            if fb.width != 1920 || fb.height != 1080 {
                crate::serial_println!("WARNING: Resolution mismatch! Assets will look corrupted.");
            }

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

            // Initialize the Compositor
            let mut compositor = Compositor::new();

            crate::serial_println!("Aether Desktop Ready. Enabling Interrupts...");
            x86_64::instructions::interrupts::enable();

            unsafe { 
                renderer.clear_screen(0x000D1117);
                compositor.draw_icons(&renderer);
                renderer.swap_buffers(); 
            }

            loop {
                let (mx, my) = drivers::legacy::mouse::get_mouse_pos();
                let draw_x = mx as usize;
                let draw_y = my as usize;

                let last_x = renderer.last_cursor_x;
                let last_y = renderer.last_cursor_y;

                compositor.handle_click(mx, my);
                if let Some(sc) = drivers::legacy::keyboard::pop_scancode() {
                    compositor.handle_keyboard(sc);
                }

                renderer.draw_rect(last_x.saturating_sub(5), last_y.saturating_sub(5), 40, 40, 0x000D1117);
                compositor.draw_icons(&renderer);
                compositor.render(&renderer);
                renderer.draw_cursor(draw_x, draw_y);

                unsafe {
                    let min_x = last_x.min(draw_x).saturating_sub(10);
                    let min_y = last_y.min(draw_y).saturating_sub(10);
                    let max_x = last_x.max(draw_x) + 40;
                    let max_y = last_y.max(draw_y) + 40;

                    renderer.swap_rect(
                        min_x, 
                        min_y, 
                        (max_x - min_x).min(1919), 
                        (max_y - min_y).min(1079)
                    );
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