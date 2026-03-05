// kernel/src/arch/x86_64/idt.rs
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::arch::x86_64::timer;
use crate::serial_println;
use pic8259::ChainedPics;
use spin::Mutex;
use core::arch::naked_asm;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

// Must be public for pic.rs and init sequences
pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

const FONT_8X8: [[u8; 8]; 16] = [
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
        IDT[32].set_handler_fn(timer_interrupt_handler);

        // Map Double Fault to the wrapper and the IST index
        IDT.double_fault.set_handler_fn(double_fault_wrapper)
            .set_stack_index(crate::arch::x86_64::gdt::DOUBLE_FAULT_IST_INDEX);

        IDT.load();
        
        let mut pics = PICS.lock();
        pics.initialize();
        pics.write_masks(0xFE, 0xFF); 
    }
    serial_println!("IDT Loaded.");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    timer::tick();
    unsafe { PICS.lock().notify_end_of_interrupt(32); }
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
            "add rdi, 8",   // Arg 1: Pointer to InterruptStackFrame
            "mov rsi, [rbp + 8]", // Arg 2: Error code (pushed by CPU)
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