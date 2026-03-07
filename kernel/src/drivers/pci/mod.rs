use x86_64::instructions::port::Port;
use alloc::vec;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
}

impl PciDevice {
    /// Reads a 32-bit dword from the PCI configuration space.
    pub fn read_config(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset as u32 & 0xFC)
            | 0x8000_0000;

        unsafe {
            Port::new(CONFIG_ADDRESS).write(address);
            Port::<u32>::new(CONFIG_DATA).read()
        }
    }

    /// Reads BAR0 (Base Address Register 0)
    pub fn get_bar0(&self) -> u64 {
        let bar0 = Self::read_config(self.bus, self.slot, self.func, 0x10);
        // Check if it's a 64-bit BAR (Type 2)
        if (bar0 & 0b110) == 0b100 {
            let bar1 = Self::read_config(self.bus, self.slot, self.func, 0x14);
            ((bar1 as u64) << 32) | (bar0 as u64 & !0xF)
        } else {
            bar0 as u64 & !0xF
        }
    }
}

/// Scans the PCI bus for devices, specifically looking for the xHCI controller.
pub fn scan_bus() -> vec::Vec<PciDevice> {
    let mut devices = vec::Vec::new();

    for bus in 0..255 {
        for slot in 0..32 {
            let vendor_id = (PciDevice::read_config(bus as u8, slot, 0, 0) & 0xFFFF) as u16;
            if vendor_id == 0xFFFF { continue; } // Device doesn't exist

            let header = PciDevice::read_config(bus as u8, slot, 0, 0x08);
            let class = (header >> 24) as u8;
            let subclass = (header >> 16) as u8;
            let prog_if = (header >> 8) as u8;

            devices.push(PciDevice {
                bus: bus as u8,
                slot,
                func: 0,
                vendor_id,
                device_id: (PciDevice::read_config(bus as u8, slot, 0, 0) >> 16) as u16,
                class,
                subclass,
                prog_if,
            });
        }
    }
    devices
}