// kernel/src/drivers/legacy/mouse.rs
use x86_64::instructions::port::Port;
use crate::serial_println;

static mut LAST_CLICK_TICK: u64 = 0;
pub static mut IS_DOUBLE_CLICK: bool = false;

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_button: bool,
    pub right_button: bool,
}

static mut MOUSE: MouseState = MouseState { 
    x: 960, 
    y: 540, 
    left_button: false, 
    right_button: false 
};

static mut PACKET_BYTE: u8 = 0;
static mut PACKET: [u8; 3] = [0; 3];

pub unsafe fn init() {
    let mut command_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);

    drain();

    // Enable Auxiliary Device
    wait_write();
    command_port.write(0xA8); 

    // Enable Interrupts
    wait_write();
    command_port.write(0x20); 
    let mut status = wait_read();
    status |= 0x02;   
    status &= !0x20;  
    
    wait_write();
    command_port.write(0x60); 
    wait_write();
    data_port.write(status);

    // Initial reset
    reset();
}

pub unsafe fn handle_interrupt() {
    let mut data_port = Port::<u8>::new(0x60);
    let byte = data_port.read();

    match PACKET_BYTE {
        0 => {
            // Byte 0 must have bit 3 set to 1. If not, we are out of sync.
            if byte & 0x08 != 0 {
                PACKET[0] = byte;
                PACKET_BYTE = 1;
            }
        }
        1 => {
            PACKET[1] = byte;
            PACKET_BYTE = 2;
        }
        2 => {
            PACKET[2] = byte;
            PACKET_BYTE = 0;
            process_packet();
        }
        _ => PACKET_BYTE = 0,
    }
}

unsafe fn process_packet() {
    let flags = PACKET[0];
    
    // Check for overflow
    if (flags & 0x40) != 0 || (flags & 0x80) != 0 { return; }

    let mut x_delta = PACKET[1] as i32;
    let mut y_delta = PACKET[2] as i32;

    // Proper Sign Extension for 9-bit relative values
    if (flags & 0x10) != 0 { x_delta -= 256; }
    if (flags & 0x20) != 0 { y_delta -= 256; }

    // Update with strict clamping to prevent the "1.5 screen wrap"
    let new_x = MOUSE.x + x_delta;
    let new_y = MOUSE.y - y_delta; // Y is inverted in PS/2

    MOUSE.x = new_x.clamp(0, 1910);
    MOUSE.y = new_y.clamp(0, 1070);
    
    MOUSE.left_button = (flags & 0x01) != 0;
}

pub fn get_mouse_pos() -> (i32, i32) {

    unsafe { (MOUSE.x, MOUSE.y) }

}

pub fn is_left_pressed() -> bool {

    unsafe { MOUSE.left_button }

}

// --- Controller Communication Helpers ---

unsafe fn drain() {
    let mut status_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    while (status_port.read() & 0x01) != 0 {
        data_port.read();
    }
}

unsafe fn wait_write() {
    let mut port = Port::<u8>::new(0x64);
    let mut timeout = 100_000;
    while (port.read() & 0x02) != 0 && timeout > 0 {
        timeout -= 1;
    }
}

unsafe fn wait_read() -> u8 {
    let mut port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    let mut timeout = 100_000;
    while (port.read() & 0x01) == 0 && timeout > 0 {
        timeout -= 1;
    }
    if timeout == 0 { 0 } else { data_port.read() }
}

unsafe fn mouse_write(data: u8) {
    let mut command_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    wait_write();
    command_port.write(0xD4); // Route next byte to mouse
    wait_write();
    data_port.write(data);
}

unsafe fn mouse_read() -> u8 {
    wait_read()
}

pub unsafe fn check_double_click(current_ticks: u64) {
    if is_left_pressed() {
        let delta = current_ticks - LAST_CLICK_TICK;
        if delta < 300 { // 300ms window
            IS_DOUBLE_CLICK = true;
        }
        LAST_CLICK_TICK = current_ticks;
    } else {
        IS_DOUBLE_CLICK = false;
    }
}

pub unsafe fn reset() {
    // Force a hardware reset to sync the 3-byte stream
    mouse_write(0xFF);
    let _ack = mouse_read(); 
    let _test = mouse_read(); 
    let _id = mouse_read();   

    // Set Sample Rate
    mouse_write(0xF3);
    let _ = mouse_read();
    mouse_write(100);
    let _ = mouse_read();

    // Final Enable
    mouse_write(0xF4);
    let _ = mouse_read();
    
    PACKET_BYTE = 0; // Reset our internal counter
    crate::serial_println!("PS/2 Mouse: Resynced.");
}