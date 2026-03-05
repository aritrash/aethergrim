use pic8259::ChainedPics;
use spin::Mutex;
use crate::arch::x86_64::idt::IDT;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

// Add this to your idt::init() function
pub fn init() {
    unsafe { IDT.load() };
    unsafe { PICS.lock().initialize() }; // Remap the PIC
}