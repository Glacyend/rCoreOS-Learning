#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
fn main() -> i32 {
    let current_time = user_lib::get_time();
    let wait_for = current_time + 3000;
    while user_lib::get_time() < wait_for {
        user_lib::r#yield();
    }
    println!("Test sleep OK!");

    0
}
