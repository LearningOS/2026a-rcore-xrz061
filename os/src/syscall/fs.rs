//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat, ROOT_INODE};
use crate::mm::{
    checked_user_buffer, checked_user_str, translated_byte_buffer, translated_str, PTEFlags,
    UserBuffer,
};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// Copy a fully initialized stat through validated user pages.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let task = current_task().unwrap();
    let file = {
        let inner = task.inner_exclusive_access();
        let Some(Some(file)) = inner.fd_table.get(fd) else {
            return -1;
        };
        file.clone()
    };
    let Some(stat) = file.stat() else {
        return -1;
    };
    let Some(buffers) = checked_user_buffer(
        current_user_token(),
        st as usize,
        core::mem::size_of::<Stat>(),
        PTEFlags::W,
    ) else {
        return -1;
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            &stat as *const Stat as *const u8,
            core::mem::size_of::<Stat>(),
        )
    };
    let mut offset = 0;
    for buffer in buffers {
        let end = offset + buffer.len();
        buffer.copy_from_slice(&bytes[offset..end]);
        offset = end;
    }
    0
}

/// Link two names to the same inode without copying file contents.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    let token = current_user_token();
    let Some(old) = checked_user_str(token, old_name) else {
        return -1;
    };
    let Some(new) = checked_user_str(token, new_name) else {
        return -1;
    };
    if ROOT_INODE.link(&old, &new) {
        0
    } else {
        -1
    }
}

/// Remove one name and reclaim an unlinked inode after its final open handle.
pub fn sys_unlinkat(name: *const u8) -> isize {
    let Some(name) = checked_user_str(current_user_token(), name) else {
        return -1;
    };
    if ROOT_INODE.unlink(&name) {
        0
    } else {
        -1
    }
}
