//! Process management syscalls
use crate::mm::{checked_user_buffer, PTEFlags};
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_user_token, with_current_memory};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// Copy both time fields through validated user mappings, even across pages.
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let fields = [us / 1_000_000, us % 1_000_000];
    let bytes = unsafe {
        core::slice::from_raw_parts(
            fields.as_ptr() as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let Some(buffers) =
        checked_user_buffer(current_user_token(), ts as usize, bytes.len(), PTEFlags::W)
    else {
        return -1;
    };
    let mut offset = 0;
    for buffer in buffers {
        let end = offset + buffer.len();
        buffer.copy_from_slice(&bytes[offset..end]);
        offset = end;
    }
    0
}

/// Trace a user byte after checking U and the requested access permission.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 | 1 => {
            let permission = if trace_request == 0 {
                PTEFlags::R
            } else {
                PTEFlags::W
            };
            let Some(mut buffers) = checked_user_buffer(current_user_token(), id, 1, permission)
            else {
                return -1;
            };
            if trace_request == 0 {
                buffers[0][0] as isize
            } else {
                buffers[0][0] = data as u8;
                0
            }
        }
        2 => crate::task::syscall_count(id).map_or(-1, |count| count as isize),
        _ => -1,
    }
}

/// Allocate anonymous user memory.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    with_current_memory(|memory| memory.mmap(start, len, prot))
}

/// Release anonymous user memory and its page frames.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    with_current_memory(|memory| memory.munmap(start, len))
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
