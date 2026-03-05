use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicU64, Ordering};

// The PIT crystal frequency in Hz
const PIT_BASE_FREQUENCY: u32 = 1193182;

// Global tick counter
static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init(frequency: u32) {
    let divisor = PIT_BASE_FREQUENCY / frequency;

    unsafe {
        // Command port (0x43): Select Channel 0, Square Wave Mode
        let mut cmd_port = Port::new(0x43);
        // Data port (0x40): Channel 0
        let mut data_port = Port::new(0x40);

        cmd_port.write(0x36 as u8);
        
        // Write low byte then high byte of the divisor
        data_port.write((divisor & 0xFF) as u8);
        data_port.write(((divisor >> 8) & 0xFF) as u8);
    }
}

pub fn tick() {
    TICKS.fetch_add(1, Ordering::SeqCst);
}

pub fn get_ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}