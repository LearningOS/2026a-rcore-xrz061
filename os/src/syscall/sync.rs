use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;

pub fn sys_sleep(ms: usize) -> isize {
    let Some(expire_ms) = get_time_ms().checked_add(ms) else {
        return -1;
    };
    add_timer(expire_ms, current_task().unwrap());
    block_current_and_run_next();
    0
}

pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let id = inner.mutex_list.len();
    let mutex: Arc<dyn Mutex> = if blocking {
        Arc::new(MutexBlocking::new(id))
    } else {
        Arc::new(MutexSpin::new(id))
    };
    inner.mutex_list.push(Some(mutex));
    inner.mutex_detector.add_resource(1);
    id as isize
}

fn get_mutex(id: usize) -> Option<Arc<dyn Mutex>> {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    inner.mutex_list.get(id)?.clone()
}

fn get_semaphore(id: usize) -> Option<Arc<Semaphore>> {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    inner.semaphore_list.get(id)?.clone()
}

fn get_condvar(id: usize) -> Option<Arc<Condvar>> {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    inner.condvar_list.get(id)?.clone()
}

pub fn sys_mutex_lock(id: usize) -> isize {
    let Some(mutex) = get_mutex(id) else {
        return -1;
    };
    mutex.lock()
}

pub fn sys_mutex_unlock(id: usize) -> isize {
    let Some(mutex) = get_mutex(id) else {
        return -1;
    };
    mutex.unlock();
    0
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    if res_count > isize::MAX as usize {
        return -1;
    }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let id = inner.semaphore_list.len();
    inner
        .semaphore_list
        .push(Some(Arc::new(Semaphore::new(res_count, id))));
    inner.semaphore_detector.add_resource(res_count);
    id as isize
}

pub fn sys_semaphore_up(id: usize) -> isize {
    let Some(sem) = get_semaphore(id) else {
        return -1;
    };
    sem.up();
    0
}

pub fn sys_semaphore_down(id: usize) -> isize {
    let Some(sem) = get_semaphore(id) else {
        return -1;
    };
    sem.down()
}

pub fn sys_condvar_create() -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let id = inner.condvar_list.len();
    inner.condvar_list.push(Some(Arc::new(Condvar::new())));
    id as isize
}

pub fn sys_condvar_signal(id: usize) -> isize {
    let Some(condvar) = get_condvar(id) else {
        return -1;
    };
    condvar.signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let Some(condvar) = get_condvar(condvar_id) else {
        return -1;
    };
    let Some(mutex) = get_mutex(mutex_id) else {
        return -1;
    };
    condvar.wait(mutex)
}

pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    if enabled > 1 {
        return -1;
    }
    current_process().inner_exclusive_access().deadlock_enabled = enabled == 1;
    0
}
