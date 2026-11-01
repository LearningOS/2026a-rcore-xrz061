//! Per-process resource allocation and pending-request accounting.
extern crate alloc;
#[cfg(not(test))]
use crate::task::{current_process, current_task};
use alloc::{vec, vec::Vec};

/// Error returned when a resource request would cause deadlock.
pub const DEADLOCK: isize = -0xdead;

#[derive(Default)]
/// Available, allocation and pending-need matrices for one resource family.
pub struct DeadlockDetector {
    available: Vec<usize>,
    allocation: Vec<Vec<usize>>,
    need: Vec<Vec<usize>>,
}

impl DeadlockDetector {
    /// Register a newly created resource.
    pub fn add_resource(&mut self, count: usize) {
        self.available.push(count);
        for row in &mut self.allocation {
            row.push(0);
        }
        for row in &mut self.need {
            row.push(0);
        }
    }

    fn ensure_thread(&mut self, tid: usize) {
        while self.allocation.len() <= tid {
            self.allocation.push(vec![0; self.available.len()]);
            self.need.push(vec![0; self.available.len()]);
        }
    }

    /// Record a pending request, reverting it if the enabled safety check fails.
    pub fn request(&mut self, tid: usize, id: usize, enabled: bool) -> bool {
        self.ensure_thread(tid);
        self.need[tid][id] += 1;
        if enabled && !self.safe() {
            self.need[tid][id] -= 1;
            return false;
        }
        true
    }

    fn safe(&self) -> bool {
        let mut work = self.available.clone();
        let mut finish = vec![false; self.allocation.len()];
        loop {
            let mut progress = false;
            for (tid, done) in finish.iter_mut().enumerate() {
                if !*done
                    && self.need[tid]
                        .iter()
                        .zip(&work)
                        .all(|(need, free)| need <= free)
                {
                    for (free, held) in work.iter_mut().zip(&self.allocation[tid]) {
                        *free += held;
                    }
                    *done = true;
                    progress = true;
                }
            }
            if !progress {
                return finish.iter().all(|done| *done);
            }
        }
    }

    /// Commit a real acquisition, including a direct wakeup handoff.
    pub fn acquire(&mut self, tid: usize, id: usize) {
        self.available[id] -= 1;
        self.need[tid][id] -= 1;
        self.allocation[tid][id] += 1;
    }

    /// Return a token and optionally transfer it to a queued waiter.
    pub fn release(&mut self, tid: usize, id: usize, next: Option<usize>) {
        self.ensure_thread(tid);
        // Semaphore producers may signal a token they did not acquire themselves.
        self.allocation[tid][id] = self.allocation[tid][id].saturating_sub(1);
        self.available[id] += 1;
        if let Some(next) = next {
            self.acquire(next, id);
        }
    }

    /// Forget an exited thread without inventing releases of its held resources.
    pub fn forget_thread(&mut self, tid: usize) {
        if tid < self.allocation.len() {
            self.allocation[tid].fill(0);
            self.need[tid].fill(0);
        }
    }
}

#[derive(Clone, Copy)]
/// Independent resource families permitted by the chapter specification.
pub enum Resource {
    /// Mutual exclusion resources.
    Mutex,
    /// Counting semaphore resources.
    Semaphore,
}

#[cfg(not(test))]
/// Current thread identifier before accessing the owning process.
pub fn current_tid() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid
}

#[cfg(not(test))]
fn with_detector<R>(kind: Resource, f: impl FnOnce(&mut DeadlockDetector, bool, usize) -> R) -> R {
    let tid = current_tid();
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let enabled = inner.deadlock_enabled;
    let detector = match kind {
        Resource::Mutex => &mut inner.mutex_detector,
        Resource::Semaphore => &mut inner.semaphore_detector,
    };
    f(detector, enabled, tid)
}

#[cfg(not(test))]
/// Check and record the current thread's request.
pub fn request(kind: Resource, id: usize) -> bool {
    with_detector(kind, |detector, enabled, tid| {
        detector.request(tid, id, enabled)
    })
}

#[cfg(not(test))]
/// Commit the current thread's successful acquisition.
pub fn acquire(kind: Resource, id: usize) {
    with_detector(kind, |detector, _, tid| detector.acquire(tid, id));
}

#[cfg(not(test))]
/// Release or hand off a resource in the current process.
pub fn release(kind: Resource, id: usize, next: Option<usize>) {
    with_detector(kind, |detector, _, tid| detector.release(tid, id, next));
}

#[cfg(test)]
mod tests {
    use super::DeadlockDetector;

    #[test]
    fn cycle_rejection_rolls_back_and_handoff_unblocks() {
        let mut detector = DeadlockDetector::default();
        detector.add_resource(1);
        detector.add_resource(1);
        assert!(detector.request(0, 0, true));
        detector.acquire(0, 0);
        assert!(detector.request(1, 1, true));
        detector.acquire(1, 1);
        assert!(detector.request(0, 1, true));
        assert!(!detector.request(1, 0, true));
        assert_eq!(detector.need[1][0], 0);
        detector.release(1, 1, Some(0));
        assert_eq!(detector.need[0][1], 0);
        assert_eq!(detector.allocation[0], [1, 1]);
        detector.release(0, 0, None);
        detector.release(0, 1, None);
        assert_eq!(detector.available, [1, 1]);
    }

    #[test]
    fn toggle_after_acquisition_and_reused_thread() {
        let mut detector = DeadlockDetector::default();
        detector.add_resource(1);
        assert!(detector.request(7, 0, false));
        detector.acquire(7, 0);
        assert!(!detector.request(7, 0, true));
        detector.release(7, 0, None);
        detector.forget_thread(7);
        assert!(detector.request(7, 0, true));
        detector.acquire(7, 0);
    }

    #[test]
    fn multi_instance_completion_is_safe() {
        let mut detector = DeadlockDetector::default();
        detector.add_resource(2);
        for tid in 0..2 {
            assert!(detector.request(tid, 0, true));
            detector.acquire(tid, 0);
        }
        assert!(detector.request(0, 0, true));
        assert!(!detector.request(1, 0, true));
        detector.release(1, 0, Some(0));
        assert_eq!(detector.allocation[0][0], 2);
    }
}
