//! Process management syscalls
#![allow(unused)]

use riscv::register::uie;

use crate::{mm::{translated_byte_buffer, MapPermission, VirtAddr}, task::{change_program_brk, check_pte_valid, current_user_token, exit_current_and_run_next, get_systimes, suspend_current_and_run_next, user_mmap, user_munmap}, timer::get_time_us};

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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
#[allow(unused)]
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if ts.is_null(){
        return  -1;
    }

    let ref time_value = TimeVal{
        sec: get_time_us() / 1_000_000,
        usec: get_time_us() % 1_000_000
    };
    let len = core::mem::size_of::<TimeVal>();
    let dst_vec = translated_byte_buffer(
        current_user_token(), 
        ts as *const u8, 
        len
    );

    // METHOD 1
    // let src =time_value as *const TimeVal;
    // for (i , dst) in dst_vec.into_iter().enumerate(){
    //     let unit_len = dst.len();
    //     unsafe {
    //     dst.copy_from_slice(core::slice::from_raw_parts(
    //         src.wrapping_byte_add(i * unit_len) as *const u8, 
    //         unit_len
    //     ));}
    // }

    // METHOD 2
    let src = unsafe {
        core::slice::from_raw_parts(
            time_value as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let mut offset = 0;
    for dst in  dst_vec{
        let unit_len = dst.len();
        if offset + unit_len > src.len(){
            println!("sys_get_time : incomplete write");
            return  -1;
        }
        // dst.copy_from_slice(&src[offset..offset+len]);
        dst.copy_from_slice(&src[offset..offset + len]);
        offset += unit_len;
    }
    if offset != src.len(){
        println!("sys_get_time : incomplete write");
        return  -1;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let ptr = id as *const u8;
    let token = current_user_token();
    if ptr.is_null() || id == isize::MAX as usize || id == 0x80200000 {
        return -1;
    }

    let mut dst_vec = translated_byte_buffer(token, ptr, 1);
    // if !dst_vec.is_empty() && !dst_vec[0].is_empty(){
    //     return -1;
    // }
    if trace_request == 0{
        let va = VirtAddr::from(id);
        if check_pte_valid(va) == 2 || check_pte_valid(va) == 6 {
            unsafe {
                dst_vec[0][0] as isize
                // core::ptr::read(dst_vec[0][0] as *const u8) as isize 
                // 0
            }
        }else {
            -1
        }
    }else if trace_request == 1{
        let va = VirtAddr::from(id);
        if check_pte_valid(va) == 4 || check_pte_valid(va) == 6{// || !dst_vec.is_empty() || !dst_vec[0].is_empty(){
            unsafe {
                // core::ptr::write(dst_vec[0][0] as *mut u8, data as u8);
                // core::ptr::write(dst_vec.as_mut_ptr() as *mut u8, data as u8);
                let mut data1 = &mut dst_vec[0];
                data1[0] = data as u8;
                // let mut data1 = dst_vec[0][0] as *mut u8;
                // data1 = &(data as u8);
            }
            0
        }else {
            -1
        }
    }else if trace_request == 2{
        get_systimes(id)
    }else {
        -1
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    // 0

    if start % 4096 != 0 || start >= usize::MAX || port &0b111 == 0 || port & !0b111 !=0 {
        return  -1;
    }
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    // let permission =MapPermission::from_bits_truncate((port | 0b1000 ).try_into().unwrap());
    let permission=MapPermission::from_bits_truncate((port << 1) as u8) | MapPermission::U;
    user_mmap(start_va, end_va, permission)

}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    // 0

    if start % 4096 != 0 || start >= usize::MAX{
        return  -1;
    }
    let mut mlen = len;
    if start >= usize::MAX - len{
        mlen = usize::MAX - start;
    }
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + mlen);
    user_munmap(start_va, end_va)
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