pub mod gdt;
pub mod idt;
pub mod timer;
pub mod pic;
pub mod serial;
pub mod process;
pub mod memory;

pub fn init() {
    gdt::init();
    idt::init();
    pic::init();
}