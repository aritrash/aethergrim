pub mod gdt;
pub mod idt;
pub mod timer;
pub mod pic;
pub mod serial;

pub fn init() {
    gdt::init();
    idt::init();
    pic::init();
}