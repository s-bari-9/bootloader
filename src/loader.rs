// kernel_loader.rs
// Loads a Linux bzImage and jumps to it from UEFI

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::{ptr, slice};
use uefi::prelude::*;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::media::file::{File, FileMode, FileType};
use uefi::table::boot::{AllocateType, MemoryType};
use uefi::CStr16;

const KERNEL_LOAD_ADDR: usize = 0x100000;
const BOOT_PARAMS_ADDR: usize = 0x90000;
const SETUP_SECTS_OFFSET: usize = 0x1F1;
const HEADER_MAGIC_OFFSET: usize = 0x202;
const HEADER_MAGIC: u32 = 0x53726448; // "HdrS"

#[repr(C, packed)]
struct SetupHeader {
    setup_sects: u8,
    root_flags: u16,
    syssize: u32,
    ramdisk_image: u32,
    ramdisk_size: u32,
    cmd_line_ptr: u32,
    type_of_loader: u8,
    loadflags: u8,
    setup_move_size: u16,
    code32_start: u32,
    ramdisk_max: u32,
    kernel_alignment: u32,
    relocatable_kernel: u8,
    min_alignment: u8,
    cmdline_size: u16,
    hardware_subarch: u32,
    payload_offset: u64,
    payload_length: u64,
    pref_address: u64,
    init_size: u32,
    handover_offset: u32,
    kernel_info_offset: u32,
}

pub fn load_kernel_image(
    st: &SystemTable<Boot>,
    kernel_path: &str,
    initrd_path: Option<&str>,
    cmdline: Option<&str>,
) -> Result<!> {
    let kernel_data = read_file_to_vec(st, kernel_path)?;

    // Check Linux boot protocol header magic
    let hdr_magic = u32::from_le_bytes([
        kernel_data[HEADER_MAGIC_OFFSET],
        kernel_data[HEADER_MAGIC_OFFSET + 1],
        kernel_data[HEADER_MAGIC_OFFSET + 2],
        kernel_data[HEADER_MAGIC_OFFSET + 3],
    ]);
    if hdr_magic != HEADER_MAGIC {
        panic!("Invalid kernel header magic");
    }

    // Load kernel image to 0x100000
    let pages = (kernel_data.len() + 0xFFF) / 0x1000;
    let kernel_addr = st.boot_services().allocate_pages(
        AllocateType::Address(KERNEL_LOAD_ADDR),
        MemoryType::LOADER_DATA,
        pages,
    )?;
    unsafe {
        ptr::copy_nonoverlapping(kernel_data.as_ptr(), KERNEL_LOAD_ADDR as *mut u8, kernel_data.len());
    }

    // Load initrd
    let (initrd_addr, initrd_size) = if let Some(path) = initrd_path {
        let initrd = read_file_to_vec(st, path)?;
        let pages = (initrd.len() + 0xFFF) / 0x1000;
        let addr = st.boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            pages,
        )?;
        unsafe {
            ptr::copy_nonoverlapping(initrd.as_ptr(), addr as *mut u8, initrd.len());
        }
        (addr, initrd.len())
    } else {
        (0, 0)
    };

    // Setup boot_params structure at 0x90000
    let boot_params = BOOT_PARAMS_ADDR as *mut u8;
    unsafe {
        ptr::write_bytes(boot_params, 0, 4096);
        // Copy first 0x1f1 bytes from kernel image
        ptr::copy_nonoverlapping(kernel_data.as_ptr(), boot_params, 0x1F1);
    }

    // Add initrd and cmdline pointers to setup header
    let setup_hdr = unsafe { &mut *(boot_params.add(0x1F1) as *mut SetupHeader) };
    setup_hdr.ramdisk_image = initrd_addr as u32;
    setup_hdr.ramdisk_size = initrd_size as u32;

    if let Some(cmdline_str) = cmdline {
        let bytes = cmdline_str.as_bytes();
        let cmdline_pages = (bytes.len() + 1 + 0xFFF) / 0x1000;
        let cmdline_addr = st.boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            cmdline_pages,
        )?;
        unsafe {
            let dst = cmdline_addr as *mut u8;
            ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
            *dst.add(bytes.len()) = 0;
        }
        setup_hdr.cmd_line_ptr = cmdline_addr as u32;
    }

    // Exit boot services
    let (_st, _map_key) = st.exit_boot_services();

    // Jump to kernel entry
    let kernel_entry = KERNEL_LOAD_ADDR + 0x200; // Entry point offset for bzImage
    let entry_fn: extern "C" fn() -> ! = unsafe { core::mem::transmute(kernel_entry as *const ()) };
    entry_fn()
}

/// Helper to read a file into a Vec<u8>
fn read_file_to_vec(st: &SystemTable<Boot>, path: &str) -> Result<Vec<u8>> {
    let image = st.boot_services().handle_protocol::<uefi::proto::loaded_image::LoadedImage>(
        st.boot_services().image_handle())?
        .interface;
    let device = unsafe { (*image.get()).device().unwrap() };
    let fs = st.boot_services().handle_protocol::<SimpleFileSystem>(device)?.interface;
    let mut root = unsafe { (*fs.get()).open_volume()? };

    // Convert path to UTF-16
    let mut utf16 = [0u16; 256];
    let mut i = 0;
    for ch in path.encode_utf16() {
        if i >= 255 { break; }
        utf16[i] = ch;
        i += 1;
    }
    utf16[i] = 0;

    let handle = root.open(unsafe { CStr16::from_u16_with_nul_unchecked(&utf16[..=i]) }, FileMode::Read, Default::default())?;
    let mut file = match handle.into_type()? {
        FileType::Regular(f) => f,
        _ => panic!("{} is not a regular file", path),
    };

    let mut buf = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        let read = file.read(&mut chunk)?;
        if read == 0 { break; }
        buf.extend_from_slice(&chunk[..read]);
    }
    Ok(buf)
}
