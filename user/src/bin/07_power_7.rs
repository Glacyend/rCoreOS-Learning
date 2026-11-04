#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

const LEN: usize = 100;

#[unsafe(no_mangle)]
fn main() -> i32 {
    let p = 7;
    let m = 998244353;
    let iter = 160000;
    let mut s = [0_u64; LEN];
    let mut cur = 0;
    s[cur] = 1;
    for i in 1..=iter {
        let next = if cur + 1 == LEN { 0 } else { cur + 1 };
        s[next] = s[cur] * p % m;
        cur = next;
        if i % 10000 == 0 {
            println!("power_7 [{}/{}]", i, iter);
        }
    }
    println!("{}^{} = {} (mod {})", p, iter, s[cur], m);
    println!("Test power_7 OK!");
    0
}
