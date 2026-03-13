use core::ptr::{read_volatile, write_volatile, addr_of_mut};

#[repr(C, align(16))]
struct BdlEntry {
    address: u64,
    length: u32,
    flags: u32, // Bit 0 = Interrupt on completion
}

pub struct HdaController {
    base: usize,
    hhdm: usize,
    bdl_virt: *mut BdlEntry, // Manual allocation
}

impl HdaController {
    pub unsafe fn new(bar0: usize, hhdm: usize, bdl_ptr: *mut u8) -> Self {
        Self { 
            base: bar0 + hhdm, 
            hhdm,
            bdl_virt: bdl_ptr as *mut BdlEntry,
        }
    }

    pub unsafe fn setup(&mut self) {
        let gctl = (self.base + 0x08) as *mut u32;
        
        // Controller Reset
        write_volatile(gctl, read_volatile(gctl) & !1);
        while (read_volatile(gctl) & 1) != 0 { core::hint::spin_loop(); }
        
        // Exit Reset + Enable Bus Control (Bit 0 and Bit 1)
        write_volatile(gctl, 0x1 | 0x2); 
        while (read_volatile(gctl) & 1) == 0 { core::hint::spin_loop(); }
        
        crate::serial_println!("HDA: Controller Online (BCE Enabled).");
    }

    /// Automatically scans the codec to find the right DAC and Pin for playback
    pub unsafe fn discover_nodes(&mut self) {
        crate::serial_println!("HDA: Starting Codec Node Discovery...");
        
        // Get number of function groups from the root (Node 0)
        // Verb 0xF0004 = Get Subordinate Node Count
        let resp = self.send_verb(0, 0xF0004);
        let start_node = (resp >> 16) & 0xFF;
        let node_count = resp & 0xFF;
        
        for i in 0..node_count {
            let nid = start_node + i;
            // Verb 0xF0005 = Get Function Group Type (0x01 is Audio)
            let fg_type = self.send_verb(nid, 0xF0005) & 0xFF;
            
            if fg_type == 0x01 { 
                crate::serial_println!("HDA: Found Audio Function Group at Node {}", nid);
                self.setup_afg(nid);
                return;
            }
        }
    }

    unsafe fn setup_afg(&mut self, afg_nid: u32) {
        let resp = self.send_verb(afg_nid, 0xF0004);
        let start_node = (resp >> 16) & 0xFF;
        let node_count = resp & 0xFF;

        let mut dac_node = 0;
        let mut pin_node = 0;

        for nid in start_node..(start_node + node_count) {
            // Verb 0xF0009 = Audio Widget Capabilities
            let caps = self.send_verb(nid, 0xF0009);
            let w_type = (caps >> 20) & 0xF;

            match w_type {
                0x0 => { // Audio Output (DAC)
                    if dac_node == 0 { dac_node = nid; }
                }
                0x4 => { // Pin Complex
                    // Verb 0xF000C = Pin Capabilities
                    let pin_caps = self.send_verb(nid, 0xF000C);
                    if (pin_caps & (1 << 4)) != 0 { // Check 'Output' bit
                        if pin_node == 0 { pin_node = nid; }
                    }
                }
                _ => {}
            }
        }

        if dac_node != 0 && pin_node != 0 {
            crate::serial_println!("HDA: Routing DAC (Node {}) to Pin (Node {})", dac_node, pin_node);
            self.route_audio(dac_node, pin_node);
        } else {
            crate::serial_println!("HDA: Failed to find valid DAC/Pin pair.");
        }
    }

    unsafe fn route_audio(&mut self, dac: u32, pin: u32) {
        // Wake up nodes (Set Power State D0)
        self.send_verb(dac, 0x70500); 
        self.send_verb(pin, 0x70500);

        // 1. Configure DAC: Set Stream 1, Channel 0
        self.send_verb(dac, 0x70601); 
        // Unmute DAC Output Amp
        self.send_verb(dac, 0x3907F); 

        // 2. Configure Pin: Enable Output and Unmute
        self.send_verb(pin, 0x70740); 
        self.send_verb(pin, 0x3B07F); 
        
        crate::serial_println!("HDA: Audio Path Initialized.");
    }

    pub unsafe fn play(&mut self, data: &[u8]) {
        let stream_off = 0x80 + (4 * 0x20); // Output Stream 0
        let sd_base = (self.base + stream_off) as *mut u32;

        // 1. Force Reset the Stream
        write_volatile(sd_base, 0); 
        for _ in 0..1000 { core::hint::spin_loop(); }

        // 2. Setup BDL (Must be 128-byte aligned, which our PMM frame is)
        let phys_data = (data.as_ptr() as usize) - self.hhdm;
        let bdl_entry = &mut *self.bdl_virt;
        bdl_entry.address = phys_data as u64;
        bdl_entry.length = data.len() as u32;
        bdl_entry.flags = 0x1; // Interrupt on completion

        let bdl_addr_ptr = (sd_base as usize + 0x18) as *mut u64;
        write_volatile(bdl_addr_ptr, (self.bdl_virt as usize - self.hhdm) as u64);

        // 3. Set Stream ID to 1
        // CRITICAL: We also need to tell the DAC (Node 2) to listen to Stream 1
        // (This was in your route_audio, but let's double check it here)
        let mut ctrl = 1 << 20; // Stream ID 1
        write_volatile(sd_base, ctrl);

        // 4. Set cyclic buffer params
        write_volatile((sd_base as usize + 0x08) as *mut u32, data.len() as u32); // CBL
        write_volatile((sd_base as usize + 0x0C) as *mut u16, 0); // LVI (0 = 1 entry)
        write_volatile((sd_base as usize + 0x12) as *mut u16, 0x11); // Try 48kHz (0x11) instead of 44.1

        crate::serial_println!("HDA: DMA registers written. Pushing RUN...");

        // 5. RUN!
        write_volatile(sd_base, read_volatile(sd_base) | 0x2);
    }

    pub unsafe fn send_verb(&mut self, node: u32, verb: u32) -> u32 {
        let ic_ptr = (self.base + 0x60) as *mut u32;      
        let ir_ptr = (self.base + 0x64) as *mut u32;      
        let ics_ptr = (self.base + 0x68) as *mut u16;     

        let mut timeout = 0;
        while (read_volatile(ics_ptr) & 0x1) != 0 {
            core::hint::spin_loop();
            timeout += 1;
            if timeout > 100000 { return 0xFFFFFFFF; }
        }

        write_volatile(ics_ptr, read_volatile(ics_ptr) | 0x2);

        let command = (node << 20) | verb;
        write_volatile(ic_ptr, command);
        write_volatile(ics_ptr, read_volatile(ics_ptr) | 0x1);

        timeout = 0;
        while (read_volatile(ics_ptr) & 0x2) == 0 {
            core::hint::spin_loop();
            timeout += 1;
            if timeout > 100000 { return 0xFFFFFFFF; }
        }

        let response = read_volatile(ir_ptr);
        crate::serial_println!("HDA Verb: [Node {} Verb {:#x}] -> Resp: {:#x}", node, verb, response);
        response
    }

    pub unsafe fn verify_codec(&mut self) {
        let vendor = self.send_verb(0, 0xF0000);
        crate::serial_println!("HDA Codec Vendor ID: {:#x}", vendor);
    }
}