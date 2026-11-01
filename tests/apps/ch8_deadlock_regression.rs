#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::sync::atomic::{AtomicUsize, Ordering};
use user_lib::{
    condvar_create, condvar_signal, condvar_wait, enable_deadlock_detect, exit,
    mutex_blocking_create, mutex_create, mutex_lock, mutex_unlock, semaphore_create,
    semaphore_down, semaphore_up, thread_create, waittid, yield_,
};

const DEADLOCK: isize = -0xdead;
static LOCKS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static READY: AtomicUsize = AtomicUsize::new(0);
static REJECTED: AtomicUsize = AtomicUsize::new(0);
static CONDITION: AtomicUsize = AtomicUsize::new(0);

fn cycle(first: usize) {
    let own = LOCKS[first].load(Ordering::SeqCst);
    let other = LOCKS[1 - first].load(Ordering::SeqCst);
    assert_eq!(mutex_lock(own), 0);
    READY.fetch_add(1, Ordering::SeqCst);
    while READY.load(Ordering::SeqCst) < 2 {
        yield_();
    }
    let result = mutex_lock(other);
    if result == DEADLOCK {
        REJECTED.fetch_add(1, Ordering::SeqCst);
    } else {
        assert_eq!(result, 0);
        mutex_unlock(other);
    }
    mutex_unlock(own);
    exit(0);
}

fn signal_condition(_: usize) {
    let lock = LOCKS[0].load(Ordering::SeqCst);
    assert_eq!(mutex_lock(lock), 0);
    condvar_signal(CONDITION.load(Ordering::SeqCst));
    mutex_unlock(lock);
    exit(0);
}

#[no_mangle]
fn main() -> i32 {
    let spin = mutex_create() as usize;
    assert_eq!(enable_deadlock_detect(false), 0);
    assert_eq!(mutex_lock(spin), 0);
    assert_eq!(enable_deadlock_detect(true), 0);
    assert_eq!(mutex_lock(spin), DEADLOCK);
    assert_eq!(mutex_lock(spin), DEADLOCK);
    mutex_unlock(spin);
    assert_eq!(mutex_lock(spin), 0);
    mutex_unlock(spin);
    assert_eq!(mutex_lock(usize::MAX), -1);
    assert_eq!(semaphore_down(usize::MAX), -1);

    let invalid: isize;
    unsafe {
        core::arch::asm!("ecall", inlateout("a0") 2usize => invalid, in("a7") 469usize);
    }
    assert_eq!(invalid, -1);

    let sem = semaphore_create(0) as usize;
    assert_eq!(semaphore_down(sem), DEADLOCK);
    semaphore_up(sem);
    assert_eq!(semaphore_down(sem), 0);
    assert_eq!(semaphore_down(sem), DEADLOCK);
    semaphore_up(sem);

    // Reject and recover from a two-thread cycle with both mutex implementations.
    for blocking in [false, true] {
        READY.store(0, Ordering::SeqCst);
        REJECTED.store(0, Ordering::SeqCst);
        for lock in &LOCKS {
            let id = if blocking {
                mutex_blocking_create()
            } else {
                mutex_create()
            };
            assert!(id >= 0);
            lock.store(id as usize, Ordering::SeqCst);
        }
        let a = thread_create(cycle as usize, 0);
        let b = thread_create(cycle as usize, 1);
        assert!(a > 0 && b > 0);
        assert_eq!(waittid(a as usize), 0);
        assert_eq!(waittid(b as usize), 0);
        assert_eq!(REJECTED.load(Ordering::SeqCst), 1);
    }

    // Condvar release/reacquisition must participate in mutex bookkeeping.
    let lock = mutex_blocking_create() as usize;
    let condition = condvar_create() as usize;
    LOCKS[0].store(lock, Ordering::SeqCst);
    CONDITION.store(condition, Ordering::SeqCst);
    assert_eq!(mutex_lock(lock), 0);
    let child = thread_create(signal_condition as usize, 0);
    assert!(child > 0);
    condvar_wait(condition, lock);
    assert_eq!(mutex_lock(lock), DEADLOCK);
    mutex_unlock(lock);
    assert_eq!(waittid(child as usize), 0);
    println!("Deadlock regression OK!");
    0
}
