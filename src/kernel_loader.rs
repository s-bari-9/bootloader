use core::ops::Deref;
use alloc::borrow::ToOwned;
use alloc::ffi::CString;
use alloc::vec::Vec;
use uefi::CStr16;
use uefi::println;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::{
    file::{File, FileAttribute, FileInfo, FileMode, FileType},
    fs::SimpleFileSystem,
};
use uefi::{Result, boot};

// This file and entries_parse has duplicate logic
// TODO: merge them
pub fn load_efi_from_path(
    kernel_path: &str,
    initrd_path: Option<&str>,
    cmdline: Option<&str>,
) -> Result {
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
    let device_handle = loaded_image.device();
    let mut sfs = boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle.unwrap())?;
    let mut current_dir = sfs.open_volume()?;

    println!("Loading kernel from path: {}", kernel_path);

    let kernel_path = kernel_path.replace('/', "\\");
    let path_parts: Vec<&str> = kernel_path
        .split('\\') //.filter(|part| !part.is_empty())
        .collect();
    let filename = path_parts.last().unwrap();

    // Navigate through directories if path has subdirectories
    for &dir_name in &path_parts[..path_parts.len() - 1] {
        if !dir_name.is_empty() {
            println!("Navigating to directory: {}", dir_name);

            // Convert to UTF-16 string
            let mut dir_name_utf16 = [0u16; 256];
            let mut i = 0;
            for ch in dir_name.chars() {
                if i >= 255 {
                    break;
                }
                dir_name_utf16[i] = ch as u16;
                i += 1;
            }
            dir_name_utf16[i] = 0; // Null terminator

            let dir_handle = current_dir.open(
                unsafe { CStr16::from_u16_with_nul_unchecked(&dir_name_utf16[..=i]) },
                FileMode::Read,
                FileAttribute::empty(),
            )?;

            current_dir = match dir_handle.into_type()? {
                FileType::Dir(dir) => dir,
                FileType::Regular(_) => {
                    println!("{} is not a directory", dir_name);
                    return Err(uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()));
                }
            };
        }
    }

    let mut filename_utf16 = [0u16; 256];
    let mut i = 0;
    for ch in filename.chars() {
        if i >= 255 {
            break;
        }
        filename_utf16[i] = ch as u16;
        i += 1;
    }
    filename_utf16[i] = 0; // Null terminator

    let kernel_file_handle = current_dir.open(
        unsafe { CStr16::from_u16_with_nul_unchecked(&filename_utf16[..=i]) },
        FileMode::Read,
        FileAttribute::empty(),
    )?;

    let mut kernel_file = match kernel_file_handle.into_type()? {
        FileType::Regular(file) => file,
        FileType::Dir(_) => {
            println!("{} is a directory, not a file", filename);
            return Err(uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()));
        }
    };

    let mut info_buffer = [0u8; 200]; // FIXME: the 200 value is a placeholder because rust needs
                                      // a fixed array size
    let file_info = kernel_file.get_info::<FileInfo>(&mut info_buffer);
    let file_size = file_info.unwrap().file_size() as usize;

    println!("{} size: {} bytes", filename, file_size);

    // Allocate memory for the kernel image
    let kernel_pages = (file_size + 4095) / 4096; // Round up to page boundary
    let kernel_addr = boot::allocate_pages(
        boot::AllocateType::AnyPages,
        boot::MemoryType::LOADER_DATA,
        kernel_pages,
    )?;

    // Read the kernel file into memory
    let kernel_buffer = unsafe { core::slice::from_raw_parts_mut(kernel_addr.as_ptr(), file_size) };

    kernel_file.read(kernel_buffer)?;
    println!(
        "{} loaded into memory at 0x{:x}",
        filename,
        kernel_addr.as_ptr() as usize
    );

    // Load the image
    let kernel_image_handle = boot::load_image(
        boot::image_handle(),
        boot::LoadImageSource::FromBuffer {
            buffer: kernel_buffer,
            file_path: None,
        },
    )?;

    let mut kernel_loaded_image_device =
        boot::open_protocol_exclusive::<LoadedImage>(kernel_image_handle)?;

    /*if let Some(initrd) = initrd_path {
        if let Some(cmdline_str) = cmdline {
            let options = CString::new(cmdline_str).unwrap();
            let options_bytes = options.as_bytes_with_nul();
        }
    }*/

    let mut options_str = "".to_owned();

    if let Some(initrd) = initrd_path {
        options_str += "console=ttyS0,115200 earlyprintk=efi,keep initrd=";
        options_str += &initrd.replace("/", "\\");
        if cmdline.is_some() {
            options_str += " ";
        }
    }

    if let Some(cmdline_str) = cmdline {
        options_str += cmdline_str;
    }

    println!("{}\n{:?}\n{:?}",options_str, initrd_path, cmdline);

    let options = CString::new(options_str.clone()).unwrap();
    //let options_bytes = options.as_bytes_with_nul();
    let options_bytes : Vec<u16> = options_str.encode_utf16().collect();
    //et ptr: *const u8 = options.as_ptr() as *const u8;
    let len: u32 = options.as_bytes_with_nul().len() as u32;

    unsafe {
        kernel_loaded_image_device.set_load_options(options_bytes.as_ptr() as *const u8, len);
    }
    println!(
    "kernel_image_handle ptr = {:p}/n{:?}",
    kernel_image_handle.as_ptr(),
    kernel_image_handle
    );
    println!("{:?}", kernel_loaded_image_device.deref());
    println!("{} image loaded, starting execution...", filename);
    // Start the kernel image
    boot::start_image(kernel_image_handle)?;
    Ok(())
}
