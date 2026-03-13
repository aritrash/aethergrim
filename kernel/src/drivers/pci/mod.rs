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

    /// Writes a 32-bit dword to the PCI configuration space.
    pub fn write_config(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
        let address = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset as u32 & 0xFC)
            | 0x8000_0000;

        unsafe {
            Port::new(CONFIG_ADDRESS).write(address);
            Port::<u32>::new(CONFIG_DATA).write(value);
        }
    }

    pub fn read_config_u16(&self, offset: u8) -> u16 {
        let val = Self::read_config(self.bus, self.slot, self.func, offset);
        // If offset is 0x02, we want the upper 16 bits of the dword at 0x00.
        // If offset is 0x04, we want the lower 16 bits of the dword at 0x04.
        (val >> ((offset & 2) * 8)) as u16
    }

    pub fn write_config_u16(&self, offset: u8, value: u16) {
        let mut val = Self::read_config(self.bus, self.slot, self.func, offset);
        if (offset & 2) == 0 {
            val = (val & 0xFFFF0000) | (value as u32);
        } else {
            val = (val & 0x0000FFFF) | ((value as u32) << 16);
        }
        Self::write_config(self.bus, self.slot, self.func, offset, val);
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

/// Scans the PCI bus for devices, including multi-function devices.
pub fn scan_bus() -> vec::Vec<PciDevice> {
    let mut devices = vec::Vec::new();

    for bus in 0..255 {
        for slot in 0..32 {
            // A single slot can have up to 8 functions
            for func in 0..8 {
                let vendor_id = (PciDevice::read_config(bus as u8, slot, func, 0) & 0xFFFF) as u16;
                if vendor_id == 0xFFFF { 
                    // If func 0 doesn't exist, the whole slot is empty
                    if func == 0 { break; } 
                    // If a higher func doesn't exist, just continue to the next func
                    continue; 
                }

                let header = PciDevice::read_config(bus as u8, slot, func, 0x08);
                let class = (header >> 24) as u8;
                let subclass = (header >> 16) as u8;
                let prog_if = (header >> 8) as u8;

                devices.push(PciDevice {
                    bus: bus as u8,
                    slot,
                    func,
                    vendor_id,
                    device_id: (PciDevice::read_config(bus as u8, slot, func, 0) >> 16) as u16,
                    class,
                    subclass,
                    prog_if,
                });

                // Check if this is a multi-function device
                if func == 0 {
                    let header_type = (PciDevice::read_config(bus as u8, slot, 0, 0x0C) >> 16) as u8;
                    // Bit 7 indicates multi-function
                    if (header_type & 0x80) == 0 {
                        break; // Not multi-function, skip other functions
                    }
                }
            }
        }
    }
    devices
}