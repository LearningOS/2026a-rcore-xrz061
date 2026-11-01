//! Mutex (spin-like and blocking(sleep))

use super::deadlock::{self, Resource, DEADLOCK};
use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self) -> isize;
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    id: usize,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new(id: usize) -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
            id,
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexSpin::lock");
        if !deadlock::request(Resource::Mutex, self.id) {
            return DEADLOCK;
        }
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                drop(locked);
                deadlock::acquire(Resource::Mutex, self.id);
                return 0;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
        deadlock::release(Resource::Mutex, self.id, None);
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
    id: usize,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new(id: usize) -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            id,
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexBlocking::lock");
        if !deadlock::request(Resource::Mutex, self.id) {
            return DEADLOCK;
        }
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
            drop(mutex_inner);
            deadlock::acquire(Resource::Mutex, self.id);
        }
        0
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            let tid = waking_task
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .tid;
            deadlock::release(Resource::Mutex, self.id, Some(tid));
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
            deadlock::release(Resource::Mutex, self.id, None);
        }
    }
}
