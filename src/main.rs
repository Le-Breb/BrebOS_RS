#![feature(abi_x86_interrupt)]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(brebos_rs::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod framebuffer;
mod gdt;
mod interrupts;
mod memory;
mod serial;

use core::panic::PanicInfo;
use brebos_rs::framebuffer::VGA_MEM;

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    framebuffer::FB
        .lock()
        .set_fg_color(framebuffer::FbColor::Red);
    println!("{}", info);
    framebuffer::FB
        .lock()
        .set_fg_color(framebuffer::FbColor::White);

    brebos_rs::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    brebos_rs::test_panic_handler(info);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    gdt::init();
    interrupts::init();

    //x86_64::instructions::interrupts::int3();

    #[cfg(test)]
    test_main();

    println!("The numbers are {} and {}", 42, 1.0 / 3.0);

    brebos_rs::hlt_loop();
}
