use x86_64::{
    structures::paging::{PageTable, OffsetPageTable, Page, PhysFrame, Size4KiB, Mapper, PageTableFlags, FrameAllocator},
    VirtAddr, PhysAddr,
};
use super::pmm;

/// The BootFrameAllocator is the bridge between the x86_64 crate's 
/// memory mapping logic and our physical bitmap.
pub struct BootFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for BootFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        // Request a physical frame from the PMM bitmap.
        // We wrap the raw u64 address into the types expected by the paging crate.
        pmm::find_free_frame().map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

/// Initializes a new OffsetPageTable.
/// This uses the HHDM (Higher Half Direct Map) offset to access physical 
/// page tables from virtual memory.
pub unsafe fn init(phys_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4_table = active_level_4_table(phys_offset);
    OffsetPageTable::new(l4_table, phys_offset)
}

/// Grabs a mutable reference to the active level 4 page table.
unsafe fn active_level_4_table(phys_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    // Read the physical address of the P4 table from the CR3 register.
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    
    // Add the HHDM offset to get the virtual address.
    let virt = phys_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// A high-level helper to map a virtual page to a physical frame.
/// This handles the allocation of intermediate page table levels (P3, P2, P1) 
/// automatically using our BootFrameAllocator.
pub unsafe fn map_page(
    mapper: &mut OffsetPageTable,
    vaddr: VirtAddr,
    paddr: PhysAddr,
    flags: PageTableFlags,
) {
    let page = Page::<Size4KiB>::containing_address(vaddr);
    let frame = PhysFrame::containing_address(paddr);
    let mut frame_allocator = BootFrameAllocator;

    // map_to will fail if the page is already mapped.
    let map_to_result = mapper.map_to(page, frame, flags, &mut frame_allocator);
    map_to_result.expect("Paging Error: Failed to map page").flush();
}