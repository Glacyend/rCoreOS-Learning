#![no_std]
#![no_main]

use core::ptr;

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("load_fault app running...");
    println!("Into test load_fault, we will insert an invalid load operation...");
    println!("Kernel should kill this application!");
    unsafe {
        let _i = ptr::read_volatile(ptr::null_mut::<u8>());
    }
    0
}
