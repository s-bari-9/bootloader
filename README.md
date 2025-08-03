# Rust UEFI Bootloader

A UEFI bootloader (systemd-boot compatible(TODO))

## Directory Structure

Expected layout on the EFI System Partition (XBOOTLDR partition is **NOT** supported right now, so use a single ESP for now):

```
/EFI/BOOT/BOOTX64.EFI         ← This bootloader
/EFI/BOOT/KERNEL.EFI          ← Hello world file for testing bootloader
/loader/loader.conf           ← Global config (TODO)
/loader/entries/\*.conf       ← Per-entry configs(TODO)
````

Boot entries are supported as in [UAPI specifications](https://uapi-group.org/specifications/specs/boot_loader_specification/#type-1-boot-loader-specification-entries).

Bootloader has been tested in qemu and can load a kernel from a hardcoded path right now.

## TODO


* [X] Load UEFI executables
* [-] Support [bootloader entries](https://uapi-group.org/specifications/specs/boot_loader_specification/#type-1-boot-loader-specification-entries)
    * [x] Kernel files found in `/EFI/Linux/`
    * [x] UEFI shell `/shellx64.efi`
* [-] Boot menu selection UI
* [X] Windows/Apple chainloading
* [ ] XBOOTLDR partition support
* [ ] Bootloader conf
* [ ] Pass kernel options
* [ ] Initrd loading
* [ ] ACPI and memory map handoff

## Extra
* [ ] Encrypted XBOOTLDR support

( [-] for partial support)

## Building

* Install rust toolchain for uefi:

```sh
rustup target add x86_64-unknown-uefi
```

* Build the UEFI executable:

```sh
cargo build
```

Here is the `.cargo/config.toml`:

```toml
[build]
target = "x86_64-unknown-uefi"

[target.x86_64-unknown-uefi]
runner = "extra/efirunner.sh"
linker = "rust-lld"
```

* Test in qemu: `efirunner.sh` does this.

## License

Licensed under GPL-3.0
