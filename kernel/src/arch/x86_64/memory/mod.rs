use limine::{MemmapRequest, HhdmRequest};
use x86_64::VirtAddr;

static MMAP_REQUEST: MemmapRequest = MemmapRequest::new(0);
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new(0);

pub mod pmm;
pub mod paging;

pub fn get_hhdm_offset() -> u64 {
    HHDM_REQUEST.get_response().get().expect("HHDM Request failed").offset
}

pub fn init() {
    let mmap = MMAP_REQUEST.get_response().get().expect("PMM: Failed to get memory map");
    let hhdm = HHDM_REQUEST.get_response().get().expect("VMM: Failed to get HHDM");
    
    let phys_offset = VirtAddr::new(hhdm.offset);

    // Initialize PMM first so paging can use the frame allocator
    pmm::init(mmap, hhdm.offset);
    
    // Initialize Paging
    unsafe {
        paging::init(phys_offset);
    }

    crate::serial_println!("Aether Grim Memory: Physical and Virtual maps are synchronized.");
}