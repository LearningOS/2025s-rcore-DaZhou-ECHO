//! Process management syscalls

use crate::{mm::translated_byte_buffer, task::{change_program_brk, current_user_token, exit_current_and_run_next, get_systimes, suspend_current_and_run_next}, timer::get_time_us};

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
    let mut dst_vec = translated_byte_buffer(token, ptr, 1);
    if ptr.is_null(){
        return -1;
    }
    if trace_request == 0{
        unsafe {
            core::ptr::read(dst_vec.as_ptr() as *const u8) as isize
        }
        // *dst_vec[0]
    }else if trace_request == 1{
        // for (i,dst) in dst_vec.into_iter().enumerate(){
        //     let unit_len =dst.len();
        //     unsafe {
                
        //     }
        // }
        unsafe {
            core::ptr::write_bytes(dst_vec[0].as_mut_ptr(), data as u8, 1);
        }
        0
    }else if trace_request == 2{
        get_systimes(id)
    }else {
        -1
    }
    
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
