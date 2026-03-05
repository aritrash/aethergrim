// kernel/src/arch/x86_64/process.rs

use core::arch::naked_asm;
use core::sync::atomic::{AtomicBool, Ordering};

/// The number of tasks the kernel can handle (Static allocation)
const MAX_TASKS: usize = 2;
const STACK_SIZE: usize = 4096 * 2; // 8KB stacks
pub static SPLASH_COMPLETE: AtomicBool = AtomicBool::new(false);

#[repr(C, align(16))]
pub struct FpuState([u8; 512]);

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct TaskContext {
    // Callee-saved registers that we must preserve during a switch
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
    // The instruction pointer where the task will resume
    rip: u64,
}

#[derive(Copy, Clone)]
pub struct Task {
    pub id: usize,
    pub rsp: u64,      // Current Stack Pointer for this task
    pub active: bool,
    pub fpu_state_ptr: *mut u8, // Pointer to the FPU state area for this task
}

impl Task {
    pub const fn empty() -> Self {
        Self { 
            id: 0, 
            rsp: 0, 
            active: false,
            // Use core::ptr::null_mut() for the empty state
            fpu_state_ptr: core::ptr::null_mut(), 
        }
    }
}

// Global Task State
pub static mut TASK_TABLE: [Task; MAX_TASKS] = [Task::empty(); MAX_TASKS];
pub static mut CURRENT_TASK_INDEX: usize = 0;

// Static stacks for our tasks
static mut STACK_TASK_0: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut STACK_TASK_1: [u8; STACK_SIZE] = [0; STACK_SIZE];

static mut FPU_STATE_TASK_0: [u8; 512] = [0; 512];
static mut FPU_STATE_TASK_1: [u8; 512] = [0; 512];

/// The Context Switcher
/// rdi = next_rsp_ptr (*const u64)
/// rsi = current_rsp_ptr (*mut u64)
#[no_mangle]
#[unsafe(naked)]
pub extern "C" fn switch_context(next_rsp: *const u64, current_rsp: *mut u64, next_fpu: *const u8, current_fpu: *mut u8) {
    unsafe {
        naked_asm!(
            // 1. Save standard context
            "push rbx", 
            "push rbp", 
            "push r12", 
            "push r13", 
            "push r14", 
            "push r15",
            
            // 2. Save FPU/SSE state (current_fpu is in RCX)
            "fxsave [rcx]",

            // 3. Swap stacks
            "mov [rsi], rsp", 
            "mov rsp, [rdi]", 

            // 4. Restore FPU/SSE state (next_fpu is in RDX)
            "fxrstor [rdx]",
            
            // 5. Restore standard context
            "pop r15", 
            "pop r14", 
            "pop r13", 
            "pop r12", 
            "pop rbp", 
            "pop rbx",
            "ret"
        );
    }
}

/// Initializes the task table and sets up the initial stack frames
pub fn init_scheduler(shell_fn: extern "C" fn() -> !) {
    unsafe {
        // Task 0: The Current Execution (GUI/Splash)
        TASK_TABLE[0] = Task {
            id: 0,
            rsp: 0, 
            active: true,
            fpu_state_ptr: FPU_STATE_TASK_0.as_mut_ptr(),
        };

        // Task 1: The Shell
        let stack_top = STACK_TASK_1.as_ptr() as u64 + STACK_SIZE as u64;
        let mut rsp = stack_top;

        // Forge the stack frame for switch_context
        rsp -= 8;
        (rsp as *mut u64).write_volatile(shell_fn as u64);

        for _ in 0..6 {
            rsp -= 8;
            (rsp as *mut u64).write_volatile(0);
        }

        TASK_TABLE[1] = Task {
            id: 1,
            rsp: rsp,
            active: true,
            fpu_state_ptr: FPU_STATE_TASK_1.as_mut_ptr(),
        };
    }
}

/// Round-robin logic to find the next task
pub fn schedule() {
    unsafe {
        let prev_index = CURRENT_TASK_INDEX;
        let next_index = (prev_index + 1) % MAX_TASKS;
        if !TASK_TABLE[next_index].active { return; }

        CURRENT_TASK_INDEX = next_index;

        switch_context(
            &TASK_TABLE[next_index].rsp as *const u64,
            &mut TASK_TABLE[prev_index].rsp as *mut u64,
            TASK_TABLE[next_index].fpu_state_ptr,
            TASK_TABLE[prev_index].fpu_state_ptr
        );
    }
}