#![no_main]
#![no_std]

use uefi::boot;
use uefi::boot::OpenProtocolAttributes;
use uefi::boot::OpenProtocolParams;
use uefi::prelude::*;
use uefi::println;
use uefi::proto::loaded_image::LoadedImage;

#[entry]
fn main() -> Status {
    //uefi::helpers::init().unwrap();
    //use uefi_services::println;
    let image = boot::image_handle();

    // returns a Protocol<LoadedImage> wrapper

    let my_handle = boot::image_handle();

    let params = OpenProtocolParams {
        handle: my_handle, // protocol instance to open (your own image handle)
        agent: my_handle,  // caller/agent = *your* image handle
        controller: None,
    };

    let loaded_proto = unsafe {
        boot::open_protocol::<LoadedImage>(
            params,
            OpenProtocolAttributes::GetProtocol, // non-exclusive get
        ).unwrap()
    };

    // loaded_proto behaves like ScopedProtocol; get raw pointer
    if let Some(loaded) = loaded_proto.get() {
        let ptr = loaded.load_options_as_bytes().unwrap().as_ptr() as *const u8;
        let size = loaded.load_options_as_bytes().unwrap().len();

        if !ptr.is_null() && size > 0 {
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, size) };
            let trimmed = if bytes.ends_with(&[0]) {
                &bytes[..bytes.len() - 1]
            } else {
                bytes
            };
            if let Ok(s) = core::str::from_utf8(trimmed) {
                println!("LoadOptions: {}", s);
            } else {
                println!("LoadOptions (non-UTF8): {:?}", bytes);
            }
        } else {
            println!("No load options found");
        }
    } else {
        println!("LoadedImage protocol not found");
    }
    /* println!("Hi!");
    let loaded_img = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();

    // Read load options
    if let Some(options) = loaded_img.load_options_as_bytes() {
        let len = options.len();
        println!("options={:#?}\nlen={}", options, len)
    } else {
        //st.stdout().write_str
        println!("No load options found\n");
    }*/
    boot::stall(30_000_000);

    Status::SUCCESS
}
