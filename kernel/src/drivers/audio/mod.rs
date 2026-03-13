// kernel/src/drivers/audio/mod.rs
pub mod intel_hda;

pub static EVENTIDE_CHIME: &[u8] = include_bytes!("../../../assets/eventide.raw");

pub fn init_audio(bar0: usize, hhdm: usize, bdl_ptr: *mut u8) {
    unsafe {
        // Pass all three arguments now
        let mut hda = intel_hda::HdaController::new(bar0, hhdm, bdl_ptr);
        hda.setup();
        hda.discover_nodes();
        hda.play(EVENTIDE_CHIME);
    }
}