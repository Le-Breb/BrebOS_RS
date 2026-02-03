#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub use core::fmt::Write;
use core::ops::Fn;
use core::concat;
use core::format_args;
use core::panic::PanicInfo;
#[cfg(test)]
use bootloader::{entry_point, BootInfo};

extern crate alloc;
pub mod allocator;
pub mod memory;

pub mod serial;
pub mod framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}


pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

#[cfg(test)]
entry_point!(test_kernel_main);

pub mod gdt;
pub mod interrupts;
pub fn init()
{
    gdt::init();
    interrupts::init();
}

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        let mut fb = framebuffer::FB.lock();
        for _ in 0..200 {
            writeln!(fb, "test_println_many output").expect("write failed");
        }
    })
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let mut fb = framebuffer::FB.lock();

    interrupts::without_interrupts(|| {
        fb.clear_screen();
        let s = "Some test string that fits on a single line";
        writeln!(fb, "{}", s).expect("write failed");

        for (i, c) in s.chars().enumerate() {
            let screen_char =
                fb.get_buf()[i].c;
            assert_eq!(screen_char, c);
        }
    })
}

#[test_case]
fn test_print_wrap()
{
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        let mut fb = framebuffer::FB.lock();
        fb.clear_screen();
        for _y in 0..framebuffer::FB_HEIGHT
        {
            for _x in 0..framebuffer::FB_WIDTH
            {
                write!(fb, "c").expect("print failed");
            }
        }

        for i in 0..framebuffer::FB_WIDTH{
            let screen_char =
                fb.get_buf()[(framebuffer::FB_HEIGHT - 1) * framebuffer::FB_WIDTH + i].c;
            assert_eq!(screen_char, ' ');
        }

        const N: usize = 5;

        for _x in 0..N
        {
            write!(fb, "n").expect("write failed");
        }

        for i in 0..N{
            let screen_char =
                fb.get_buf()[(framebuffer::FB_HEIGHT - 1) * framebuffer::FB_WIDTH + i].c;
            assert_eq!(screen_char, 'n');
        }
    })
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}