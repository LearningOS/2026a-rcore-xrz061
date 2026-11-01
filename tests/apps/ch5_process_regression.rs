#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, fork, mmap, munmap, set_priority, spawn, waitpid};

const PAGE: usize = 4096;

fn spawn_ptr(path: *const u8) -> isize {
    let result: isize;
    unsafe {
        core::arch::asm!("ecall", inlateout("a0") path as usize => result, in("a7") 400usize);
    }
    result
}

#[no_mangle]
fn main() -> i32 {
    assert_eq!(spawn("\0"), -1);
    assert_eq!(spawn("missing-program\0"), -1);
    assert_eq!(spawn_ptr(core::ptr::null()), -1);
    assert_eq!(spawn_ptr(usize::MAX as *const u8), -1);
    assert_eq!(spawn_ptr(0x8020_0000 as *const u8), -1);
    assert_eq!(spawn_ptr([0xffu8, 0].as_ptr()), -1);
    assert_eq!(set_priority(isize::MAX), isize::MAX);
    assert_eq!(set_priority(16), 16);

    let base = 0x2000_0000;
    assert_eq!(mmap(base, 3 * PAGE, 3), 0);
    unsafe {
        core::ptr::write_volatile(base as *mut u8, 41);
        core::ptr::write_volatile((base + 2 * PAGE) as *mut u8, 77);
    }
    assert_eq!(munmap(base + PAGE, PAGE), 0);
    let child = fork();
    assert!(child >= 0);
    if child == 0 {
        unsafe {
            assert_eq!(core::ptr::read_volatile(base as *const u8), 41);
            assert_eq!(core::ptr::read_volatile((base + 2 * PAGE) as *const u8), 77);
            core::ptr::write_volatile(base as *mut u8, 91);
        }
        assert_eq!(mmap(base + PAGE, PAGE, 3), 0);
        assert_eq!(munmap(base, 3 * PAGE), 0);
        exit(42);
    }
    let mut code = 0;
    assert_eq!(waitpid(child as usize, &mut code), child);
    assert_eq!(code, 42);
    unsafe { assert_eq!(core::ptr::read_volatile(base as *const u8), 41); }
    assert_eq!(mmap(base + PAGE, PAGE, 3), 0);

    // A filename may cross the boundary between independently allocated frames.
    let path = base + PAGE - 4;
    let name = b"ch5_exit0\0";
    unsafe { core::ptr::copy_nonoverlapping(name.as_ptr(), path as *mut u8, name.len()); }
    let child = spawn_ptr(path as *const u8);
    assert!(child >= 0);
    assert_eq!(waitpid(child as usize, &mut code), child);
    assert_eq!(code, 66778);
    assert_eq!(munmap(base, 3 * PAGE), 0);
    println!("Process regression OK!");
    0
}
