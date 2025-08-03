#![no_main]
#![no_std]

use uefi::prelude::*;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    //use uefi_services::println;
    use uefi::println;
    use uefi::boot;
    println!("Hi!");
    boot::stall(3_000_000);
    Status::SUCCESS
}
