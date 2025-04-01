//! File and filesystem-related syscalls
#![allow(unused)]
use easy_fs::Inode;
use crate::fs::{create_hard_link, delete_hard_link, open_file, OSInode, OpenFlags, Stat, StatMode};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

use core::any::Any;
use core::panic;
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

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let current_task = current_task().unwrap();
    let inner = current_task.inner_exclusive_access();
    if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() || st.is_null(){
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let any: &dyn Any = file.as_any();
        let os_node = any.downcast_ref::<OSInode>().unwrap();
        // let (ino,mode , nlink) =(1,StatMode::FILE , 2);
        let (ino,mode , nlink) = os_node.get_stat().unwrap();
        drop(inner);
        let ref stat = Stat{
            dev:0,
            ino,
            mode,
            nlink,
            pad:[0;7],
        };
        let len = core::mem::size_of::<Stat>();
        let dst_vec = translated_byte_buffer(current_user_token(), st as *const u8, len);
        let src_vec = stat as *const Stat;
        for (idx , dst) in dst_vec.into_iter().enumerate(){
            let unit_len = dst.len();
            unsafe {
                dst.copy_from_slice(core::slice::from_raw_parts(
                src_vec.wrapping_add(idx * unit_len) as *const u8, 
                unit_len));
            }
        }
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    // trace!(
    //     "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
    //     current_task().unwrap().pid.0
    // );
    // -1
    let task = current_task().unwrap();
    let token = current_user_token();
    let old_path = translated_str(token, old_name);
    let new_path = translated_str(token, new_name);
    if old_path == new_path{
        return -1;
        // panic!();
    }
    if let Some(os_inode) = open_file(&old_path, OpenFlags::from_bits_truncate(2)){
        create_hard_link(&old_path,&new_path)
        // drop(inode);
        // 0
    }else {
        // panic!();
        -1
    }
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let unlink_path = translated_str(token, name);
    if let Some(os_inode) = open_file(&unlink_path, OpenFlags::from_bits_truncate(2)){
        delete_hard_link(&unlink_path)
    }else {
        -1
        // panic!()
    }
}
