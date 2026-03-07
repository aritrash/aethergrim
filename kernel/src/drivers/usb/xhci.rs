use core::ptr::{read_volatile, write_volatile, addr_of, addr_of_mut};

pub const TRBS_PER_RING: usize = 256;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Trb {
    pub data: u64,
    pub status: u32,
    pub control: u32,
}

#[repr(C, align(4096))]
pub struct XhciData {
    pub dcbaa: [u64; 256],
    pub command_ring: [Trb; TRBS_PER_RING],
    pub event_ring: [Trb; TRBS_PER_RING],
    pub event_ring_ste: [u64; 2],
}

#[repr(C)]
pub struct CapabilityRegisters {
    pub cap_length: u8,
    _reserved: u8,
    pub hci_version: u16,
    pub hcs_params1: u32,
    pub hcs_params2: u32,
    pub hcs_params3: u32,
    pub hcc_params1: u32,
    pub d_off: u32,
    pub r_off: u32,
    pub hcc_params2: u32,
}

#[repr(C)]
pub struct OperationalRegisters {
    pub usb_cmd: u32,
    pub usb_sts: u32,
    pub page_size: u32,
    _res1: [u32; 2],
    pub dn_ctrl: u32,
    pub crcr: u64,
    _res2: [u32; 4],
    pub dcbaap: u64,
    pub config: u32,
}

pub struct XhciController {
    base_addr: usize,
    hhdm_offset: usize,
    cap_regs: *const CapabilityRegisters,
    op_regs: *mut OperationalRegisters,
    data: *mut XhciData, // Stored as a raw pointer now
}

impl XhciController {
    pub unsafe fn new(phys_addr: usize, hhdm_offset: usize, data_ptr: *mut XhciData) -> Self {
        let virt_addr = phys_addr + hhdm_offset;
        let cap_regs = virt_addr as *const CapabilityRegisters;
        let cap_len = read_volatile(addr_of!((*cap_regs).cap_length)) as usize;
        let op_regs = (virt_addr + cap_len) as *mut OperationalRegisters;

        // Zero out the hardware data structures
        core::ptr::write_bytes(data_ptr, 0, 1);

        Self {
            base_addr: virt_addr,
            hhdm_offset,
            cap_regs,
            op_regs,
            data: data_ptr,
        }
    }

    pub unsafe fn reset(&mut self) {
        let usb_cmd_ptr = addr_of_mut!((*self.op_regs).usb_cmd);
        let usb_sts_ptr = addr_of!((*self.op_regs).usb_sts);

        // Stop
        let mut cmd = read_volatile(usb_cmd_ptr);
        cmd &= !0x1;
        write_volatile(usb_cmd_ptr, cmd);
        while (read_volatile(usb_sts_ptr) & 0x1) == 0 { core::hint::spin_loop(); }

        // Reset
        write_volatile(usb_cmd_ptr, 0x2);
        while (read_volatile(usb_cmd_ptr) & 0x2) != 0 { core::hint::spin_loop(); }
        
        // Wait for Controller Not Ready (CNR) bit to clear
        while (read_volatile(usb_sts_ptr) & (1 << 11)) != 0 { core::hint::spin_loop(); }

        crate::serial_println!("xHCI: Reset Successful.");
    }

    pub unsafe fn init_rings(&mut self) {
        let data_virt = self.data as usize;
        let data_phys = data_virt - self.hhdm_offset;

        // DCBAAP
        let dcbaa_phys = data_phys + (addr_of!((*self.data).dcbaa) as usize - data_virt);
        write_volatile(addr_of_mut!((*self.op_regs).dcbaap), dcbaa_phys as u64);

        // Command Ring
        let cr_phys = data_phys + (addr_of!((*self.data).command_ring) as usize - data_virt);
        write_volatile(addr_of_mut!((*self.op_regs).crcr), (cr_phys as u64) | 1);

        // Event Ring
        let runtime_off = read_volatile(addr_of!((*self.cap_regs).r_off)) as usize;
        let ir0 = (self.base_addr + runtime_off + 0x20) as *mut u32;

        let er_phys = data_phys + (addr_of!((*self.data).event_ring) as usize - data_virt);
        let ste_phys = data_phys + (addr_of!((*self.data).event_ring_ste) as usize - data_virt);

        (*self.data).event_ring_ste[0] = er_phys as u64;
        (*self.data).event_ring_ste[1] = TRBS_PER_RING as u64;

        write_volatile(ir0.add(2), 1); // ERSTSZ
        write_volatile(ir0.add(4) as *mut u64, ste_phys as u64); // ERSTBA
        write_volatile(ir0.add(6) as *mut u64, er_phys as u64);   // ERDP
        
        crate::serial_println!("xHCI: Rings Initialized.");
    }

    pub unsafe fn enable(&mut self) {
        let hcs1 = read_volatile(addr_of!((*self.cap_regs).hcs_params1));
        write_volatile(addr_of_mut!((*self.op_regs).config), hcs1 & 0xFF);

        let mut cmd = read_volatile(addr_of_mut!((*self.op_regs).usb_cmd));
        cmd |= 0x1; // Run
        write_volatile(addr_of_mut!((*self.op_regs).usb_cmd), cmd);

        while (read_volatile(addr_of!((*self.op_regs).usb_sts)) & 0x1) != 0 { core::hint::spin_loop(); }
        crate::serial_println!("xHCI: Controller is RUNNING.");
    }
}