// kernel/src/arch/x86_64/idt.rs
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::arch::x86_64::timer;
use crate::serial_println;
use pic8259::ChainedPics;
use spin::Mutex;
use core::arch::naked_asm;
use crate::arch::x86_64::process;
use crate::drivers::legacy::keyboard;
use crate::drivers::legacy::mouse;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub const FONT_8X8: [[u8; 8]; 16] = [
    [0x3E, 0x66, 0x6E, 0x7E, 0x76, 0x66, 0x3E, 0x00], [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
    [0x3E, 0x06, 0x06, 0x3E, 0x60, 0x60, 0x3E, 0x00], [0x3E, 0x06, 0x06, 0x3E, 0x06, 0x06, 0x3E, 0x00],
    [0x66, 0x66, 0x66, 0x7E, 0x06, 0x06, 0x06, 0x00], [0x7E, 0x60, 0x60, 0x7E, 0x06, 0x06, 0x7E, 0x00],
    [0x3E, 0x60, 0x60, 0x3E, 0x66, 0x66, 0x3E, 0x00], [0x7E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x00],
    [0x3E, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3E, 0x00], [0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x3E, 0x00],
    [0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00], [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
    [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00], [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
    [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x7E, 0x00], [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x00],
];

pub fn init() {
    unsafe {
        // Point IRQ 0 (32) to our naked scheduler wrapper
        let timer_wrapper_addr = timer_interrupt_wrapper as u64;
        IDT[32].set_handler_addr(x86_64::VirtAddr::new(timer_wrapper_addr));
        IDT[33].set_handler_fn(keyboard_interrupt_handler);
        IDT[44].set_handler_fn(mouse_interrupt_handler);

        // Double Fault setup
        IDT.double_fault.set_handler_fn(double_fault_wrapper)
            .set_stack_index(crate::arch::x86_64::gdt::DOUBLE_FAULT_IST_INDEX);

        IDT.load();
        
        let mut pics = PICS.lock();
        pics.initialize();
        pics.write_masks(0xFB, 0xEF);
    }
    serial_println!("IDT Loaded with Preemptive Scheduler hook.");
}

#[no_mangle]
#[unsafe(naked)]
pub extern "C" fn timer_interrupt_wrapper() {
    unsafe {
        naked_asm!(
            // 1. Save caller-saved registers
            "push rax",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",

            // 2. Align stack to 16 bytes for the Rust call
            "mov rbp, rsp",
            "and rsp, -16",

            // 3. Call the Logic
            "call {logic}",

            // 4. Restore stack pointer and registers
            "mov rsp, rbp",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rax",

            // 5. Interrupt Return
            "iretq",
            logic = sym timer_tick_logic,
        );
    }
}

extern "C" fn timer_tick_logic() {
    timer::tick();
    
    // Notify PIC
    unsafe {
        PICS.lock().notify_end_of_interrupt(32);
    }

    // Call the scheduler to check for task switches
    process::schedule();
}

#[no_mangle]
#[unsafe(naked)]
pub extern "x86-interrupt" fn double_fault_wrapper(_sf: InterruptStackFrame, _ec: u64) -> ! {
    unsafe {
        naked_asm!(
            "cli",
            "push rbp",
            "mov rbp, rsp",
            "and rsp, -16",
            "mov rdi, rbp", 
            "add rdi, 8",
            "mov rsi, [rbp + 8]",
            "call {logic}",
            logic = sym plum_logic,
        );
    }
}

extern "C" fn plum_logic(stack_frame: &InterruptStackFrame, _error_code: u64) -> ! {
    let plum_bg = 0x004e2a4f;
    serial_println!("--- DOUBLE FAULT DETECTED ---");
    serial_println!("RIP: {:#x}", stack_frame.instruction_pointer.as_u64());

    if let Some(response) = crate::FRAMEBUFFER_REQUEST.get_response().get() {
        if let Some(fb_ptr) = response.framebuffers().iter().next() {
            let fb = unsafe { &*fb_ptr.as_ptr() };
            let pixel_ptr = fb.address.as_ptr().unwrap() as *mut u32;
            let stride = fb.pitch / 4;

            for i in 0..(fb.height * stride) as isize {
                unsafe { pixel_ptr.offset(i).write_volatile(plum_bg); }
            }

            let rip = stack_frame.instruction_pointer.as_u64();
            unsafe {
                draw_hex_rip(pixel_ptr, stride, fb.height, (fb.width - 144) / 2, fb.height / 2, rip);
            }
        }
    }

    loop { x86_64::instructions::hlt(); }
}

#[inline(never)]
unsafe fn draw_hex_rip(pixel_ptr: *mut u32, stride: u64, fb_height: u64, x: u64, y: u64, rip: u64) {
    let max_index = (fb_height * stride) as isize;
    for i in 0..16 {
        let nibble = (rip >> ((15 - i) * 4)) & 0xF;
        let char_x = x + (i as u64 * 9);
        let font_char = FONT_8X8[nibble as usize];
        for row in 0..8 {
            let row_data = font_char[row];
            for col in 0..8 {
                if (row_data >> (7 - col)) & 1 == 1 {
                    let cur_x = char_x + col as u64;
                    let cur_y = y + row as u64;
                    let offset = (cur_y * stride) + cur_x;
                    if (offset as isize) < max_index {
                        pixel_ptr.offset(offset as isize).write_volatile(0x00FFFFFF);
                    }
                }
            }
        }
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::<u8>::new(0x60);
    let scancode = unsafe { port.read() };

    crate::drivers::legacy::keyboard::push_scancode(scancode);

    unsafe {
        PICS.lock().notify_end_of_interrupt(33);
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        mouse::handle_interrupt();
        PICS.lock().notify_end_of_interrupt(44);
    }
}