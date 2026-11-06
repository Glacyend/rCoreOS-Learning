#![no_std]
#![no_main]

use core::ptr;

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("store_fault app running...");
    println!("Into test store_fault, we will insert an invalid store operation...");
    println!("kernel should kill this application!");
    unsafe {
        ptr::null_mut::<u8>().write_volatile(1);
    }
    0
}
