#![feature(abi_x86_interrupt)]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(brebos_rs::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use x86_64::VirtAddr;
use brebos_rs::memory::BootInfoFrameAllocator;
use brebos_rs::allocator;
use brebos_rs::memory;
use brebos_rs::{println};

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    brebos_rs::framebuffer::FB
        .lock()
        .set_fg_color(brebos_rs::framebuffer::FbColor::Red);
    println!("{}", info);
    brebos_rs::framebuffer::FB
        .lock()
        .set_fg_color(brebos_rs::framebuffer::FbColor::White);

    brebos_rs::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    brebos_rs::test_panic_handler(info);
}

// cf https://os.phil-opp.com/paging-implementation/
entry_point!(main);

#[unsafe(no_mangle)]
pub fn main(boot_info: &'static BootInfo) -> ! {
    brebos_rs::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // new
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    // allocate a number on the heap
    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));


    //x86_64::instructions::interrupts::int3();

    #[cfg(test)]
    test_main();

    println!("The numbers are {} and {}", 42, 1.0 / 3.0);

    brebos_rs::hlt_loop();
}
