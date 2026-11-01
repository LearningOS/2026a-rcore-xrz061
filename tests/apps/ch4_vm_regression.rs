#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{mmap, munmap, trace_read, trace_write};

const PAGE: usize = 4096;

#[no_mangle]
fn main() -> i32 {
    let base = 0x2000_0000;
    assert_eq!(mmap(base, PAGE * 3, 3), 0);
    assert_eq!(trace_write((base + 2 * PAGE) as *const u8, 77), 0);
    assert_eq!(munmap(base + PAGE, PAGE), 0);
    assert_eq!(trace_read((base + PAGE) as *const u8), None);
    assert_eq!(trace_read((base + 2 * PAGE) as *const u8), Some(77));
    // A rejected range must leave its mapped prefix intact.
    assert_eq!(munmap(base, PAGE * 2), -1);
    assert_eq!(trace_read(base as *const u8), Some(0));
    assert_eq!(mmap(base + PAGE, PAGE, 3), 0);
    assert_eq!(munmap(base, PAGE * 3), 0);
    assert_eq!(mmap(base, 0, 3), 0);
    assert_eq!(munmap(base, 0), 0);
    assert_eq!(mmap(base, usize::MAX, 3), -1);
    assert_eq!(munmap(base, usize::MAX), -1);
    assert_eq!(mmap(1usize << 39, PAGE, 3), -1);
    assert_eq!(trace_read((1usize << 39) as *const u8), None);

    assert_eq!(mmap(base, PAGE * 2, 3), 0);
    // Invoke gettimeofday directly with an unaligned, cross-page TimeVal.
    let split = base + PAGE - 3;
    let mut result: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") split => result,
            in("a1") 0usize,
            in("a7") 169usize,
        );
        assert_eq!(result, 0);
        let usec = core::ptr::read_unaligned((split + core::mem::size_of::<usize>()) as *const usize);
        assert!(usec < 1_000_000);
    }
    assert_eq!(munmap(base + PAGE, PAGE), 0);
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") split => result,
            in("a1") 0usize,
            in("a7") 169usize,
        );
    }
    assert_eq!(result, -1);
    println!("VM regression OK!");
    0
}
