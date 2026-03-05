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

// Global mouse state accessible by the UI
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

    // 1. Drain the buffer to clear out any boot-time junk
    drain();

    // 2. Enable the Auxiliary (Mouse) Device
    wait_write();
    command_port.write(0xA8); 

    // 3. Enable Interrupts in the Command Byte
    wait_write();
    command_port.write(0x20); // Read Command Byte
    let mut status = wait_read();
    
    status |= 0x02;   // Bit 1: Enable Mouse IRQ
    status &= !0x20;  // Bit 5: Enable Mouse Clock (0 = enabled)
    
    wait_write();
    command_port.write(0x60); // Write Command Byte
    wait_write();
    data_port.write(status);

    // 4. Reset Mouse and check for ACK + Self-Test
    mouse_write(0xFF); // Reset
    if mouse_read() != 0xFA {
        serial_println!("Mouse: Reset ACK failed");
    }
    if mouse_read() != 0xAA {
        serial_println!("Mouse: Self-test failed");
    }
    let _id = mouse_read(); // Read extra ID byte sent by some mice

    // 5. Set Defaults
    mouse_write(0xF6);
    if mouse_read() != 0xFA {
        serial_println!("Mouse: Defaults ACK failed");
    }

    // 6. Enable Data Reporting (Start generating IRQs)
    mouse_write(0xF4);
    if mouse_read() != 0xFA { 
        serial_println!("Mouse: Enable Reporting failed"); 
    } else {
        serial_println!("PS/2 Mouse: Driver Initialized & Reporting.");
    }
}

pub unsafe fn handle_interrupt() {
    let mut data_port = Port::<u8>::new(0x60);
    let byte = data_port.read();

    match PACKET_BYTE {
        0 => {
            // Byte 0 of a packet MUST have bit 3 set to 1
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
    
    // 1. Check for overflow bits. If the mouse moved too fast, 
    // the controller sets these bits and the data is unreliable.
    if (flags & 0x40) != 0 || (flags & 0x80) != 0 {
        return;
    }

    let mut x_delta = PACKET[1] as i32;
    let mut y_delta = PACKET[2] as i32;

    // 2. STRICTOR Sign Extension
    // Relative deltas are 9-bit signed values. 
    // Bit 4 of flags is X-sign, Bit 5 is Y-sign.
    if (flags & 0x10) != 0 { x_delta -= 256; }
    if (flags & 0x20) != 0 { y_delta -= 256; }

    // 3. Sensitivity Adjustment
    // On high-res screens, a sensitivity of 1 can feel slow, but keep it 1 for now 
    // to debug the wrapping issue.
    let speed = 1;

    // 4. THE FIX: Rigid Clamping
    // We update the global MOUSE state with strict boundaries.
    // Use saturating_add if available, otherwise manual clamping.
    let new_x = MOUSE.x + (x_delta * speed);
    let new_y = MOUSE.y - (y_delta * speed); // PS/2 Y is inverted

    MOUSE.x = new_x.clamp(0, 1910); // 10px margin for cursor width
    MOUSE.y = new_y.clamp(0, 1070); // 10px margin for cursor height
    
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