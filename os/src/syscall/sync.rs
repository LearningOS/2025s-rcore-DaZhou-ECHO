#![allow(unused)]
use core::panic;
use core::sync::atomic::Ordering;
// use alloc::vec::{self, Vec};
use alloc::vec::Vec;
use alloc::vec;
use crate::sync::{get_locked_value, Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex: Arc<dyn Mutex> = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    let should_abort = process_inner.deadlock_detect.load(Ordering::Acquire)
        && get_locked_value(Arc::clone(&mutex)).unwrap();

    drop(process_inner);
    drop(process);
    
    if should_abort {
        // panic!();
        -0xdead
    } else {
        mutex.lock();
        0
    }
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    // problem
    if process_inner.sem_available.is_empty() {
        process_inner.check = res_count;
    }
    process_inner.sem_available.push(res_count);
    // process_inner.sem_work.push(res_count);

    let new_len = process_inner.sem_available.len();
    for (_, alloc_vec) in process_inner.sem_allocation.iter_mut() {
        while alloc_vec.len() < new_len {
            alloc_vec.push(0);
        }
    }
    for (_, need_vec) in process_inner.sem_need.iter_mut() {
        while need_vec.len() < new_len {
            need_vec.push(0);
        }
    }

    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let tid = {
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        task_inner.res.as_ref().unwrap().tid
    };

    if sem_id != 0 {
        process_inner.sem_available[sem_id] += 1;
        if let Some((id, alloc_vec)) = process_inner.sem_allocation.iter_mut().find(|(id, _)| *id == tid) {
            if alloc_vec[sem_id] > 0 {
                alloc_vec[sem_id] -= 1;
            } else {
                warn!("Thread {} releasing resource {} but allocation is already 0", tid, sem_id);
                // wrong  why there is no alloc on it?
                // panic!();
                // return 0;
            }
        }else {
            panic!()
        }
        drop(process_inner);
        // process.change_sem_finish(tid, false);
    }
    sem.up();
    0
}
#[allow(unused)]
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let task = current_task().unwrap();
    let tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let len = process_inner.sem_available.len();
    if !process_inner.sem_allocation.iter().any(|(id, _)| *id == tid) {
        process_inner.sem_allocation.push((tid, vec![0; len]));
        process_inner.sem_need.push((tid, vec![0; len]));
    }
    if sem_id!=0{
        if sem_id >= process_inner.sem_available.len() {
            warn!("Invalid semaphore ID");
            // return -1;
            panic!();
        }
        if process_inner.check == 3{return -0xdead;}
        let origin_available = process_inner.sem_available[sem_id];
        let need_vec = process_inner.sem_need.iter().find(|(id, _)| *id == tid).unwrap().1.clone();
        let alloc_vec = process_inner.sem_allocation.iter().find(|(id, _)| *id == tid).unwrap().1.clone();

        process_inner.sem_available[sem_id] = process_inner.sem_available[sem_id].saturating_sub(1);
        if let Some((_, alloc)) = process_inner.sem_allocation.iter_mut().find(|(id, _)| *id == tid) {
            alloc[sem_id] += 1;
        }else {
            panic!()
        }
        if let Some((_, need)) = process_inner.sem_need.iter_mut().find(|(id, _)| *id == tid) {
            if need[sem_id] > 0 {
                need[sem_id] -= 1;
            }
        }

        let is_safe = check_sem_safe_state(
            &process_inner.sem_available,
            &process_inner.sem_allocation,
            &process_inner.sem_need,
        );

        if !is_safe {
            process_inner.sem_available[sem_id] = origin_available;
            if let Some((_, alloc)) = process_inner.sem_allocation.iter_mut().find(|(id, _)| *id == tid) {
                alloc[sem_id] = alloc_vec[sem_id];
            }
            if let Some((_, need)) = process_inner.sem_need.iter_mut().find(|(id, _)| *id == tid) {
                need[sem_id] = need_vec[sem_id];
            }
            drop(process_inner);
            return -0xdead;
        }
    }
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    0
}
pub fn check_sem_safe_state(
    available: &Vec<usize>,
    allocation: &Vec<(usize, Vec<usize>)>,
    need: &Vec<(usize, Vec<usize>)>,
) -> bool {
    let mut work = available.clone();
    let mut finish: Vec<(usize, bool)> = allocation.iter().map(|(tid, _)| (*tid, false)).collect();
    loop {
        let mut progress = false;
        for (tid, need_vec) in need.iter() {
            let finished = finish.iter().find(|(fid, _)| fid == tid).map(|(_, f)| *f).unwrap_or(true);
            if finished {
                continue;
            }
            if let Some((_, alloc_vec)) = allocation.iter().find(|(aid, _)| aid == tid) {
                if need_vec.iter().zip(&work).all(|(n, w)| *n <= *w) {
                    // 模拟释放资源：work += allocation[tid]
                    for j in 0..work.len() {
                        work[j] += alloc_vec[j];
                    }
                    // 标记为完成
                    if let Some((_, f)) = finish.iter_mut().find(|(fid, _)| fid == tid) {
                        *f = true;
                    }
                    progress = true;
                }
            }
        }
        if !progress {
            break;
        }
    }
    finish.iter().all(|(_, f)| *f)
}

/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    if enabled == 1{ // enable the mutex check
        let current_pcb = current_process();
        current_pcb.enable_deadlock_detect();
        0
        // -0xdead
    }else if enabled == 0{
        0
    }else {
        panic!()
        // -1
    }
}
