use limine::{MemmapResponse, MemoryMapEntryType};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::serial_println;

static mut BITMAP_ADDR: *mut u8 = core::ptr::null_mut();
static mut BITMAP_SIZE: usize = 0;
static TOTAL_PAGES: AtomicUsize = AtomicUsize::new(0);
static FREE_PAGES: AtomicUsize = AtomicUsize::new(0);

pub fn init(mmap: &MemmapResponse, hhdm_offset: u64) {
    // 1. Memory Map Debug Printout
    serial_println!("--- PHYSICAL MEMORY MAP ---");
    for entry in mmap.memmap().iter() {
        let type_str = match entry.typ {
            MemoryMapEntryType::Usable => "Usable",
            MemoryMapEntryType::Reserved => "Reserved",
            MemoryMapEntryType::AcpiReclaimable => "ACPI Reclaim",
            MemoryMapEntryType::AcpiNvs => "ACPI NVS",
            MemoryMapEntryType::BadMemory => "Bad",
            MemoryMapEntryType::BootloaderReclaimable => "Bootloader Reclaim",
            MemoryMapEntryType::KernelAndModules => "Kernel/Modules",
            MemoryMapEntryType::Framebuffer => "Framebuffer",
            _ => "Unknown",
        };
        serial_println!("  [0x{:012x} - 0x{:012x}] : {}", entry.base, entry.base + entry.len, type_str);
    }

    // 2. Calculate Bitmap Size
    let last_entry = mmap.memmap().iter().last().unwrap();
    let total_mem = last_entry.base + last_entry.len;
    let page_count = (total_mem / 4096) as usize;
    TOTAL_PAGES.store(page_count, Ordering::SeqCst);

    let bitmap_needed_bytes = (page_count + 7) / 8;

    // 3. Find a hole for the bitmap
    let mut bitmap_phys_addr: u64 = 0;
    for entry in mmap.memmap().iter() {
        if entry.typ == MemoryMapEntryType::Usable && entry.len >= bitmap_needed_bytes as u64 {
            bitmap_phys_addr = entry.base;
            break;
        }
    }

    if bitmap_phys_addr == 0 {
        panic!("PMM: Could not find a memory region for the bitmap!");
    }

    unsafe {
        BITMAP_ADDR = (bitmap_phys_addr + hhdm_offset) as *mut u8;
        BITMAP_SIZE = bitmap_needed_bytes;

        // Initialize bitmap to 0xFF (All memory locked)
        core::ptr::write_bytes(BITMAP_ADDR, 0xFF, BITMAP_SIZE);
    }

    // 4. Mark Usable regions as FREE
    let mut free_count = 0;
    for entry in mmap.memmap().iter() {
        if entry.typ == MemoryMapEntryType::Usable {
            for page in 0..(entry.len / 4096) {
                let phys_addr = entry.base + (page * 4096);
                free_frame(phys_addr);
                free_count += 1;
            }
        }
    }
    
    // 5. Re-lock the bitmap's own pages
    for i in 0..(bitmap_needed_bytes + 4095) / 4096 {
        lock_frame(bitmap_phys_addr + (i as u64 * 4096));
        free_count -= 1;
    }

    FREE_PAGES.store(free_count, Ordering::SeqCst);
    serial_println!("PMM: Bitmap at 0x{:x}, Tracking {} pages ({} free).", 
        bitmap_phys_addr, page_count, free_count);
}

pub fn lock_frame(addr: u64) {
    let index = (addr / 4096) as usize;
    unsafe {
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        (*BITMAP_ADDR.add(byte_idx)) |= 1 << bit_idx;
    }
}

pub fn free_frame(addr: u64) {
    let index = (addr / 4096) as usize;
    unsafe {
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        (*BITMAP_ADDR.add(byte_idx)) &= !(1 << bit_idx);
    }
}

/// Core function used by the BootFrameAllocator in paging.rs
pub fn find_free_frame() -> Option<u64> {
    unsafe {
        for i in 0..BITMAP_SIZE {
            let byte = *BITMAP_ADDR.add(i);
            if byte != 0xFF { // Some bits are zero
                for bit in 0..8 {
                    if (byte & (1 << bit)) == 0 {
                        let addr = ((i * 8) + bit) as u64 * 4096;
                        lock_frame(addr);
                        FREE_PAGES.fetch_sub(1, Ordering::SeqCst);
                        return Some(addr);
                    }
                }
            }
        }
    }
    None
}