#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::fmt::Write;
mod framebuffer;

use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    for _y in 0..25
    {
        for _x in 0..80
        {
            print!("c");
        }
    }

    for _x in 0..80
    {
        print!("n");
    }

    print!("The numbers are {} and {}", 42, 1.0 / 3.0);

    loop {}
}