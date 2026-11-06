#![no_std]
#![no_main]

use core::ptr;

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Test sbrk start.");
    const PAGE_SIZE: usize = 0x1000;
    let origin_brk = user_lib::sbrk(0);
    println!("origin break point = {:X}", origin_brk);
    let brk = user_lib::sbrk(PAGE_SIZE as i32);
    if brk != origin_brk {
        return -1;
    }
    let brk = user_lib::sbrk(0);
    println!("one page allocated, break point = {:X}", brk);
    println!("try write to allocated page");
    let new_page =
        unsafe { &mut *ptr::slice_from_raw_parts_mut(origin_brk as usize as *mut u8, PAGE_SIZE) };
    for ele in new_page.iter_mut() {
        *ele = 1;
    }
    println!("write ok");
    user_lib::sbrk(PAGE_SIZE as i32 * 10);
    let brk = user_lib::sbrk(0);
    println!("10 pages allocated, break point = {:X}", brk);
    user_lib::sbrk(PAGE_SIZE as i32 * -11);
    let brk = user_lib::sbrk(0);
    println!("11 pages deallocated, break point = {:X}", brk);
    println!("try deallocated one more page, should be failed");
    let ret = user_lib::sbrk(-(PAGE_SIZE as i32));
    if ret != -1 {
        println!("Test sbrk failed!");
        return -1;
    }
    println!("Test sbrk almost OK!");
    println!("now write to deallocated page, should cause page fault.");
    for ele in new_page {
        *ele = 2;
    }
    0
}
