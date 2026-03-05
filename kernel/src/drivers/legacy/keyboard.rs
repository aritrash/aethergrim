use x86_64::instructions::port::Port;
use crossbeam_queue::ArrayQueue;
use crate::serial_println;
use spin::Mutex;

// A small queue to buffer scancodes so we don't lose keypresses
static mut SHIFT_PRESSED: bool = false;
static SCANCODE_QUEUE: Mutex<Option<ArrayQueue<u8>>> = Mutex::new(None);

pub unsafe fn init() {
    let mut command_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);

    {
        let mut queue = SCANCODE_QUEUE.lock();
        *queue = Some(ArrayQueue::new(128));
    }

    // 1. Enable the first PS/2 port (Keyboard)
    wait_write();
    command_port.write(0xAE);

    // 2. Enable Interrupts in the Controller Command Byte
    wait_write();
    command_port.write(0x20); // Read Command Byte
    let mut status = wait_read();
    
    status |= 0x01;   // Bit 0: Enable Keyboard IRQ (IRQ 1)
    status &= !0x10;  // Bit 4: Disable Keyboard Clock (0 = enabled)

    wait_write();
    command_port.write(0x60); // Write Command Byte
    wait_write();
    data_port.write(status);

    // 3. Reset Keyboard and check for ACK
    kbd_write(0xFF);
    if kbd_read() == 0xFA {
        crate::serial_println!("PS/2 Keyboard: Initialized with dynamic queue.");
    }
}

/// Called by the IDT interrupt handler
pub fn push_scancode(scancode: u8) {
    if let Some(queue) = SCANCODE_QUEUE.lock().as_ref() {
        let _ = queue.push(scancode);
    }
}

/// Consumed by the Shell or the Desktop loop
pub fn pop_scancode() -> Option<u8> {
    SCANCODE_QUEUE.lock().as_ref()?.pop()
}



pub fn scancode_to_ascii(scancode: u8) -> Option<char> {
    unsafe {
        match scancode {
            // Shift Keys (Press)
            0x2A | 0x36 => { SHIFT_PRESSED = true; None }
            // Shift Keys (Release)
            0xAA | 0xB6 => { SHIFT_PRESSED = false; None }

            // Printable Characters (Standard US Layout)
            0x1E => Some(if SHIFT_PRESSED { 'A' } else { 'a' }),
            0x30 => Some(if SHIFT_PRESSED { 'B' } else { 'b' }),
            0x2E => Some(if SHIFT_PRESSED { 'C' } else { 'c' }),
            0x20 => Some(if SHIFT_PRESSED { 'D' } else { 'd' }),
            0x12 => Some(if SHIFT_PRESSED { 'E' } else { 'e' }),
            0x21 => Some(if SHIFT_PRESSED { 'F' } else { 'f' }),
            0x22 => Some(if SHIFT_PRESSED { 'G' } else { 'g' }),
            0x23 => Some(if SHIFT_PRESSED { 'H' } else { 'h' }),
            0x17 => Some(if SHIFT_PRESSED { 'I' } else { 'i' }),
            0x24 => Some(if SHIFT_PRESSED { 'J' } else { 'j' }),
            0x25 => Some(if SHIFT_PRESSED { 'K' } else { 'k' }),
            0x26 => Some(if SHIFT_PRESSED { 'L' } else { 'l' }),
            0x32 => Some(if SHIFT_PRESSED { 'M' } else { 'm' }),
            0x31 => Some(if SHIFT_PRESSED { 'N' } else { 'n' }),
            0x18 => Some(if SHIFT_PRESSED { 'O' } else { 'o' }),
            0x19 => Some(if SHIFT_PRESSED { 'P' } else { 'p' }),
            0x10 => Some(if SHIFT_PRESSED { 'Q' } else { 'q' }),
            0x13 => Some(if SHIFT_PRESSED { 'R' } else { 'r' }),
            0x1F => Some(if SHIFT_PRESSED { 'S' } else { 's' }),
            0x14 => Some(if SHIFT_PRESSED { 'T' } else { 't' }),
            0x16 => Some(if SHIFT_PRESSED { 'U' } else { 'u' }),
            0x2F => Some(if SHIFT_PRESSED { 'V' } else { 'v' }),
            0x11 => Some(if SHIFT_PRESSED { 'W' } else { 'w' }),
            0x2D => Some(if SHIFT_PRESSED { 'X' } else { 'x' }),
            0x15 => Some(if SHIFT_PRESSED { 'Y' } else { 'y' }),
            0x2C => Some(if SHIFT_PRESSED { 'Z' } else { 'z' }),

            // Numbers
            0x02 => Some(if SHIFT_PRESSED { '!' } else { '1' }),
            0x03 => Some(if SHIFT_PRESSED { '@' } else { '2' }),
            0x04 => Some(if SHIFT_PRESSED { '#' } else { '3' }),
            0x05 => Some(if SHIFT_PRESSED { '$' } else { '4' }),
            0x06 => Some(if SHIFT_PRESSED { '%' } else { '5' }),
            0x07 => Some(if SHIFT_PRESSED { '^' } else { '6' }),
            0x08 => Some(if SHIFT_PRESSED { '&' } else { '7' }),
            0x09 => Some(if SHIFT_PRESSED { '*' } else { '8' }),
            0x0A => Some(if SHIFT_PRESSED { '(' } else { '9' }),
            0x0B => Some(if SHIFT_PRESSED { ')' } else { '0' }),

            // Special Keys
            0x39 => Some(' '),  // Space
            0x1C => Some('\n'), // Enter
            0x0E => Some('\x08'), // Backspace (represented as \x08)
            0x35 => Some(if SHIFT_PRESSED { '?' } else { '/' }),
            
            _ => None,
        }
    }
}

// --- Controller Communication Helpers ---

unsafe fn wait_write() {
    let mut port = Port::<u8>::new(0x64);
    while (port.read() & 0x02) != 0 {}
}

unsafe fn wait_read() -> u8 {
    let mut port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    while (port.read() & 0x01) == 0 {}
    data_port.read()
}

unsafe fn kbd_write(data: u8) {
    wait_write();
    let mut data_port = Port::<u8>::new(0x60);
    data_port.write(data);
}

unsafe fn kbd_read() -> u8 {
    wait_read()
}