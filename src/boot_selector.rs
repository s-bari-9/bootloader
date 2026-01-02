use crate::BootEntry; // adjust if needed
use alloc::fmt::format;
use alloc::string::String;
use alloc::vec::Vec;
use uefi::boot::SearchType;
use uefi::proto::console::text::{Input, Key, Output, ScanCode};
use uefi::{Char16, Identify, Result, ResultExt, boot};
//use core::fmt;
use uefi::println;

//pub fn boot_menu(entries: &Vec<BootEntry>, input: &mut Input) -> Result<Option<&BootEntry>> {
/*pub fn boot_menu(entries: &Vec<BootEntry>, input: &mut Input) -> Result<Option<String>> {
    if entries.is_empty() {
        println!("No boot entries found.");
        return Ok(None);
    }

    println!("\nAvailable Boot Entries:");
    for (i, entry) in entries.iter().enumerate() {
        println!("{}. {}", i + 1, entry.title);
        if let Some(ver) = &entry.version {
            println!("   Version: {}", ver);
        }
        if let Some(opts) = &entry.options {
            println!("   Options: {}", opts);
        }
        println!();
    }

    println!("Select an entry by pressing a number key, or ESC to cancel.");

    loop {
        let mut events = unsafe { [input.wait_for_key_event().unwrap()] };
        boot::wait_for_event(&mut events).discard_errdata()?;

        if let Some(key) = input.read_key()? {
            match key {
                Key::Printable(c) => {
                    if let Some(digit) = char::from(c).to_digit(10) {
                        let index = digit as usize - 1;
                        if index < entries.len() {
                            let selected = &entries[index];
                            println!("\nSelected: {}", selected.title);
                            return Ok(Some(selected.linux.clone()));
                        } else {
                            println!("Invalid selection: {}", digit);
                        }
                    }
                }
                Key::Special(ScanCode::ESCAPE) => {
                    println!("Canceled boot selection.");
                    return Ok(None);
                }
                _ => {}
            }
        }
    }
}*/

fn clear() {
    let handle = *boot::locate_handle_buffer(SearchType::ByProtocol(&Output::GUID))
        .unwrap()
        .first()
        .expect("No handle supports Output protocol");
    let mut loaded_image = boot::open_protocol_exclusive::<Output>(handle).expect("err1");
    loaded_image.clear().expect("err2");
}

pub fn boot_menu(entries: &Vec<BootEntry>, input: &mut Input) -> Result<Option<BootEntry>> {
    if entries.is_empty() {
        println!("No boot entries found.");
        return Ok(None);
    }

    let mut selected = 0;

    loop {
        //Clearing the screen is actually very important!
        clear();

        println!("BOOTLOADER — Select Entry (↑ ↓, Enter to boot, ESC to cancel)\n");

        for (i, entry) in entries.iter().enumerate() {
            if i == selected {
                println!(
                    "> {}{}",
                    entry.title,
                    entry
                        .version
                        .as_ref()
                        .map_or("".into(), |v| format(format_args!(" ({})", v)))
                );
                if let Some(opts) = &entry.options {
                    println!("    {}", opts);
                }
            } else {
                println!("  {}", entry.title);
            }
        }

        let mut events =// unsafe {
            [input.wait_for_key_event().unwrap()]
        ; //};
        boot::wait_for_event(&mut events).discard_errdata()?;

        if let Some(key) = input.read_key()? {
            match key {
                Key::Special(ScanCode::UP) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                Key::Special(ScanCode::DOWN) => {
                    if selected + 1 < entries.len() {
                        selected += 1;
                    }
                }
                Key::Special(ScanCode::ESCAPE) => {
                    println!("\nCanceled boot selection.");
                    return Ok(None);
                }
                Key::Printable(c) => {
                    if c == Char16::try_from('\r').unwrap() {
                        let chosen = entries[selected].clone();
                        println!("\nSelected: {}", chosen.title);
                        /*return Ok(Some(if let Some(linux_path) = chosen.linux.clone() {
                            (chosen, true) // true = kernel
                        } else if let Some(efi_path) = chosen.efi.clone() {
                            (efi_path, false) // false = efi
                        } else {
                            return Ok(None); // or handle error if neither exists
                        }));*/
                        return Ok(Some(chosen));
                    }
                }
                _ => {}
            }
        }
    }
}
