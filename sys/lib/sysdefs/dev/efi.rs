/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

/* Public Domain */
/* </LICENSES> */

/* <CODE> */
//! `<dev/efi/efi.h>` as data: the UEFI tables, memory map and runtime services the kernel
//! reads, for efi(4) (`kernel::dev::arm64::efi_machdep`), its runtime call (`hal::cpu::efi`)
//! and the boot-time memory map (`bsd`'s arm64 `machdep.rs`).
//!
//! Upstream: sys/dev/efi/efi.h @ 3ce1f3f79392
//! LZ: sys/dev/efi/efi.rs@e8249449d905
//!
//! ## Deviations
//! - The scalar typedefs (`UINT8`, ..., `CHAR16`) are the Rust integer types; `UINTN` (64 bits
//!   on both machines), `EFI_STATUS` and the address types keep a name.
//! - `EFI_MEMORY_TYPE` and `EFI_RESET_TYPE` are integer constants (the descriptor's `Type`
//!   is a `UINT32`), named as the C enumerators.
//! - `NextMemoryDescriptor` has no item: a walk steps `mmap_desc_size` bytes and reads each
//!   descriptor with `hal::user::pod_from_bytes` (the map's descriptor size may exceed the
//!   structure's).
//!
//! ## Redesign
//! - Every pointer of the firmware's tables (`FirmwareVendor`, `RuntimeServices`,
//!   `ConfigurationTable`, `VendorTable`, the handles and the protocols) is the `u64` word it
//!   is in memory: the tables are read from the firmware's mapping, never followed as Rust
//!   pointers. LZ's `*mut` members (and its `c_void` handles) go.
//! - `EFI_RUNTIME_SERVICES`' entries (LZ: `Option`s of seven `unsafe extern "efiapi"` function
//!   types, 7 `unsafe`, called only through `bsd`'s assembly trampoline) are `u64` slots too:
//!   a runtime service is called by its slot's offset (`offset_of!`) through `hal::cpu::efi`'s
//!   typed methods, so no function type of the firmware is written here.
//! - [`EfiGuid`], [`EfiMemoryDescriptor`], [`EfiTime`] and [`EfiConfigurationTable`] are
//!   `AbiPod` (`crate::abi_pod!`): read whole from the bytes of a map or a table. LZ's
//!   `EfiMemoryDescriptor::read_at` (one `unsafe` `read_unaligned`) goes.
//! - `efi_guidcmp(a, b) == 0`, LZ's `bool` function, is the GUIDs' `==` (derived).

#![allow(non_upper_case_globals)] // the C enumerators are CamelCase constants
#![allow(non_snake_case)] // the C field names

/// `UINTN`: the native unsigned integer, 64 bits on amd64 and arm64.
pub type Uintn = u64;
/// `EFI_PHYSICAL_ADDRESS`.
pub type EfiPhysicalAddress = u64;
/// `EFI_VIRTUAL_ADDRESS`.
pub type EfiVirtualAddress = u64;
/// `EFI_STATUS`: 0 or an `EFIERR` code.
pub type EfiStatus = Uintn;

/// `EFIERR(x)`: an error status, the top bit set.
pub const fn EFIERR(x: Uintn) -> EfiStatus {
    (1 << (Uintn::BITS - 1)) | x
}

/// `EFI_GUID`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EfiGuid {
    /// `Data1`.
    pub Data1: u32,
    /// `Data2`.
    pub Data2: u16,
    /// `Data3`.
    pub Data3: u16,
    /// `Data4`.
    pub Data4: [u8; 8],
}

crate::abi_pod!(EfiGuid {
    Data1: u32,
    Data2: u16,
    Data3: u16,
    Data4: [u8; 8],
});

/// `EFI_ACPI_20_TABLE_GUID`.
pub const EFI_ACPI_20_TABLE_GUID: EfiGuid = EfiGuid {
    Data1: 0x8868e871,
    Data2: 0xe4f1,
    Data3: 0x11d3,
    Data4: [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
};

/// `SMBIOS_TABLE_GUID`.
pub const SMBIOS_TABLE_GUID: EfiGuid = EfiGuid {
    Data1: 0xeb9d2d31,
    Data2: 0x2d88,
    Data3: 0x11d3,
    Data4: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

/// `SMBIOS3_TABLE_GUID`.
pub const SMBIOS3_TABLE_GUID: EfiGuid = EfiGuid {
    Data1: 0xf2fd1544,
    Data2: 0x9794,
    Data3: 0x4a2c,
    Data4: [0x99, 0x2e, 0xe5, 0xbb, 0xcf, 0x20, 0xe3, 0x94],
};

/// `EFI_SYSTEM_RESOURCE_TABLE_GUID`.
pub const EFI_SYSTEM_RESOURCE_TABLE_GUID: EfiGuid = EfiGuid {
    Data1: 0xb122a263,
    Data2: 0x3661,
    Data3: 0x4f68,
    Data4: [0x99, 0x29, 0x78, 0xf8, 0xb0, 0xd6, 0x21, 0x80],
};

/// `EFI_GLOBAL_VARIABLE`.
pub const EFI_GLOBAL_VARIABLE: EfiGuid = EfiGuid {
    Data1: 0x8be4df61,
    Data2: 0x93ca,
    Data3: 0x11d2,
    Data4: [0xaa, 0x0d, 0x00, 0xe0, 0x98, 0x03, 0x2b, 0x8c],
};

// EFI_MEMORY_TYPE

/// `EfiReservedMemoryType`.
pub const EfiReservedMemoryType: u32 = 0;
/// `EfiLoaderCode`.
pub const EfiLoaderCode: u32 = 1;
/// `EfiLoaderData`.
pub const EfiLoaderData: u32 = 2;
/// `EfiBootServicesCode`.
pub const EfiBootServicesCode: u32 = 3;
/// `EfiBootServicesData`.
pub const EfiBootServicesData: u32 = 4;
/// `EfiRuntimeServicesCode`.
pub const EfiRuntimeServicesCode: u32 = 5;
/// `EfiRuntimeServicesData`.
pub const EfiRuntimeServicesData: u32 = 6;
/// `EfiConventionalMemory`.
pub const EfiConventionalMemory: u32 = 7;
/// `EfiUnusableMemory`.
pub const EfiUnusableMemory: u32 = 8;
/// `EfiACPIReclaimMemory`.
pub const EfiACPIReclaimMemory: u32 = 9;
/// `EfiACPIMemoryNVS`.
pub const EfiACPIMemoryNVS: u32 = 10;
/// `EfiMemoryMappedIO`.
pub const EfiMemoryMappedIO: u32 = 11;
/// `EfiMemoryMappedIOPortSpace`.
pub const EfiMemoryMappedIOPortSpace: u32 = 12;
/// `EfiPalCode`.
pub const EfiPalCode: u32 = 13;
/// `EfiPersistentMemory`.
pub const EfiPersistentMemory: u32 = 14;
/// `EfiMaxMemoryType`.
pub const EfiMaxMemoryType: u32 = 15;

/// `EFI_MEMORY_UC`.
pub const EFI_MEMORY_UC: u64 = 0x0000000000000001;
/// `EFI_MEMORY_WC`.
pub const EFI_MEMORY_WC: u64 = 0x0000000000000002;
/// `EFI_MEMORY_WT`.
pub const EFI_MEMORY_WT: u64 = 0x0000000000000004;
/// `EFI_MEMORY_WB`.
pub const EFI_MEMORY_WB: u64 = 0x0000000000000008;
/// `EFI_MEMORY_UCE`.
pub const EFI_MEMORY_UCE: u64 = 0x0000000000000010;
/// `EFI_MEMORY_WP`.
pub const EFI_MEMORY_WP: u64 = 0x0000000000001000;
/// `EFI_MEMORY_RP`.
pub const EFI_MEMORY_RP: u64 = 0x0000000000002000;
/// `EFI_MEMORY_XP`.
pub const EFI_MEMORY_XP: u64 = 0x0000000000004000;
/// `EFI_MEMORY_NV`.
pub const EFI_MEMORY_NV: u64 = 0x0000000000008000;
/// `EFI_MEMORY_MORE_RELIABLE`.
pub const EFI_MEMORY_MORE_RELIABLE: u64 = 0x0000000000010000;
/// `EFI_MEMORY_RO`.
pub const EFI_MEMORY_RO: u64 = 0x0000000000020000;
/// `EFI_MEMORY_RUNTIME`.
pub const EFI_MEMORY_RUNTIME: u64 = 0x8000000000000000;

/// `EFI_MEMORY_DESCRIPTOR_VERSION`.
pub const EFI_MEMORY_DESCRIPTOR_VERSION: u32 = 1;

/// `EFI_MEMORY_DESCRIPTOR`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EfiMemoryDescriptor {
    /// `Type`: an `EFI_MEMORY_TYPE`.
    pub Type: u32,
    /// `Pad`.
    pub Pad: u32,
    /// `PhysicalStart`.
    pub PhysicalStart: EfiPhysicalAddress,
    /// `VirtualStart`: 0 unless the loader called `SetVirtualAddressMap`.
    pub VirtualStart: EfiVirtualAddress,
    /// `NumberOfPages`: 4 KiB pages.
    pub NumberOfPages: u64,
    /// `Attribute`: the `EFI_MEMORY_*` bits.
    pub Attribute: u64,
}

crate::abi_pod!(EfiMemoryDescriptor {
    Type: u32,
    Pad: u32,
    PhysicalStart: u64,
    VirtualStart: u64,
    NumberOfPages: u64,
    Attribute: u64,
});

// EFI_RESET_TYPE

/// `EfiResetCold`.
pub const EfiResetCold: u32 = 0;
/// `EfiResetWarm`.
pub const EfiResetWarm: u32 = 1;
/// `EfiResetShutdown`.
pub const EfiResetShutdown: u32 = 2;
/// `EfiResetPlatformSpecific`.
pub const EfiResetPlatformSpecific: u32 = 3;

/// `EFI_TABLE_HEADER`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EfiTableHeader {
    /// `Signature`.
    pub Signature: u64,
    /// `Revision`: major in the high 16 bits, minor (times ten) in the low.
    pub Revision: u32,
    /// `HeaderSize`.
    pub HeaderSize: u32,
    /// `CRC32`.
    pub CRC32: u32,
    /// `Reserved`.
    pub Reserved: u32,
}

/// `EFI_TIME`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EfiTime {
    /// `Year`: 1900 to 9999.
    pub Year: u16,
    /// `Month`: 1 to 12.
    pub Month: u8,
    /// `Day`: 1 to 31.
    pub Day: u8,
    /// `Hour`.
    pub Hour: u8,
    /// `Minute`.
    pub Minute: u8,
    /// `Second`.
    pub Second: u8,
    /// `Pad1`.
    pub Pad1: u8,
    /// `Nanosecond`.
    pub Nanosecond: u32,
    /// `TimeZone`: minutes from UTC.
    pub TimeZone: i16,
    /// `Daylight`.
    pub Daylight: u8,
    /// `Pad2`.
    pub Pad2: u8,
}

crate::abi_pod!(EfiTime {
    Year: u16,
    Month: u8,
    Day: u8,
    Hour: u8,
    Minute: u8,
    Second: u8,
    Pad1: u8,
    Nanosecond: u32,
    TimeZone: i16,
    Daylight: u8,
    Pad2: u8,
});

/// `EFI_RUNTIME_SERVICES`: the header and the entries' slots, each the address of the service
/// (a word in the firmware's runtime data, read under the runtime pmap).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EfiRuntimeServices {
    /// `Hdr`.
    pub Hdr: EfiTableHeader,
    /// `GetTime`.
    pub GetTime: u64,
    /// `SetTime`.
    pub SetTime: u64,
    /// `GetWakeupTime`.
    pub GetWakeupTime: u64,
    /// `SetWakeupTime`.
    pub SetWakeupTime: u64,

    /// `SetVirtualAddressMap`.
    pub SetVirtualAddressMap: u64,
    /// `ConvertPointer`.
    pub ConvertPointer: u64,

    /// `GetVariable`.
    pub GetVariable: u64,
    /// `GetNextVariableName`.
    pub GetNextVariableName: u64,
    /// `SetVariable`.
    pub SetVariable: u64,

    /// `GetNextHighMonotonicCount`.
    pub GetNextHighMonotonicCount: u64,
    /// `ResetSystem`.
    pub ResetSystem: u64,
}

/// `EFI_CONFIGURATION_TABLE`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EfiConfigurationTable {
    /// `VendorGuid`.
    pub VendorGuid: EfiGuid,
    /// `VendorTable`: the table's address.
    pub VendorTable: u64,
}

crate::abi_pod!(EfiConfigurationTable {
    VendorGuid: EfiGuid,
    VendorTable: u64,
});

/// `EFI_SYSTEM_TABLE`: its members' offsets are what efi(4) reads (`offset_of!`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EfiSystemTable {
    /// `Hdr`.
    pub Hdr: EfiTableHeader,
    /// `FirmwareVendor`: the address of a NUL-terminated UCS-2 string.
    pub FirmwareVendor: u64,
    /// `FirmwareRevision`.
    pub FirmwareRevision: u32,
    /// `ConsoleInHandle`.
    pub ConsoleInHandle: u64,
    /// `ConIn`.
    pub ConIn: u64,
    /// `ConsoleOutHandle`.
    pub ConsoleOutHandle: u64,
    /// `ConOut`.
    pub ConOut: u64,
    /// `StandardErrorHandle`.
    pub StandardErrorHandle: u64,
    /// `StdErr`.
    pub StdErr: u64,
    /// `RuntimeServices`.
    pub RuntimeServices: u64,
    /// `BootServices`.
    pub BootServices: u64,
    /// `NumberOfTableEntries`.
    pub NumberOfTableEntries: Uintn,
    /// `ConfigurationTable`: the address of `NumberOfTableEntries` entries.
    pub ConfigurationTable: u64,
}

/// `EFI_SYSTEM_RESOURCE_ENTRY`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EfiSystemResourceEntry {
    /// `FwClass`.
    pub FwClass: EfiGuid,
    /// `FwType`.
    pub FwType: u32,
    /// `FwVersion`.
    pub FwVersion: u32,
    /// `LowestSupportedFwVersion`.
    pub LowestSupportedFwVersion: u32,
    /// `CapsuleFlags`.
    pub CapsuleFlags: u32,
    /// `LastAttemptVersion`.
    pub LastAttemptVersion: u32,
    /// `LastAttemptStatus`.
    pub LastAttemptStatus: u32,
}

/// `EFI_SYSTEM_RESOURCE_TABLE`: the header; `FwResourceCount` entries follow it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EfiSystemResourceTable {
    /// `FwResourceCount`.
    pub FwResourceCount: u32,
    /// `FwResourceCountMax`.
    pub FwResourceCountMax: u32,
    /// `FwResourceVersion`.
    pub FwResourceVersion: u64,
    /// `Entries[]`: the flexible array member.
    pub Entries: [EfiSystemResourceEntry; 0],
}

/// `EFI_SUCCESS`.
pub const EFI_SUCCESS: EfiStatus = 0;

/// `EFI_INVALID_PARAMETER`.
pub const EFI_INVALID_PARAMETER: EfiStatus = EFIERR(2);
/// `EFI_UNSUPPORTED`.
pub const EFI_UNSUPPORTED: EfiStatus = EFIERR(3);
/// `EFI_BUFFER_TOO_SMALL`.
pub const EFI_BUFFER_TOO_SMALL: EfiStatus = EFIERR(5);
/// `EFI_DEVICE_ERROR`.
pub const EFI_DEVICE_ERROR: EfiStatus = EFIERR(7);
/// `EFI_WRITE_PROTECTED`.
pub const EFI_WRITE_PROTECTED: EfiStatus = EFIERR(8);
/// `EFI_OUT_OF_RESOURCES`.
pub const EFI_OUT_OF_RESOURCES: EfiStatus = EFIERR(9);
/// `EFI_NOT_FOUND`.
pub const EFI_NOT_FOUND: EfiStatus = EFIERR(14);
/// `EFI_SECURITY_VIOLATION`.
pub const EFI_SECURITY_VIOLATION: EfiStatus = EFIERR(26);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn layouts() {
        assert_eq!(size_of::<EfiGuid>(), 16);
        assert_eq!(size_of::<EfiMemoryDescriptor>(), 40);
        assert_eq!(size_of::<EfiTableHeader>(), 24);
        assert_eq!(size_of::<EfiTime>(), 16);
        assert_eq!(size_of::<EfiRuntimeServices>(), 24 + 11 * 8);
        assert_eq!(offset_of!(EfiRuntimeServices, GetTime), 24);
        assert_eq!(offset_of!(EfiRuntimeServices, SetTime), 32);
        assert_eq!(size_of::<EfiConfigurationTable>(), 24);
        assert_eq!(size_of::<EfiSystemTable>(), 24 + 12 * 8);
        assert_eq!(offset_of!(EfiSystemTable, FirmwareVendor), 24);
        assert_eq!(offset_of!(EfiSystemTable, FirmwareRevision), 32);
        assert_eq!(offset_of!(EfiSystemTable, RuntimeServices), 88);
        assert_eq!(offset_of!(EfiSystemTable, NumberOfTableEntries), 104);
        assert_eq!(offset_of!(EfiSystemTable, ConfigurationTable), 112);
        assert_eq!(size_of::<EfiSystemResourceTable>(), 16);
    }

    #[test]
    fn status_codes() {
        assert_eq!(EFI_INVALID_PARAMETER, 0x8000_0000_0000_0002);
        assert_eq!(EFI_SECURITY_VIOLATION, 0x8000_0000_0000_001a);
    }

    #[test]
    fn walk_descriptors() {
        // The fields' offsets a walk reads at each `mmap_desc_size` step (the walk itself,
        // over `pod_from_bytes`, is efi(4)'s and is tested there).
        assert_eq!(offset_of!(EfiMemoryDescriptor, Type), 0);
        assert_eq!(offset_of!(EfiMemoryDescriptor, PhysicalStart), 8);
        assert_eq!(offset_of!(EfiMemoryDescriptor, VirtualStart), 16);
        assert_eq!(offset_of!(EfiMemoryDescriptor, NumberOfPages), 24);
        assert_eq!(offset_of!(EfiMemoryDescriptor, Attribute), 32);
    }

    #[test]
    fn guids() {
        assert_eq!(SMBIOS3_TABLE_GUID, SMBIOS3_TABLE_GUID);
        assert_ne!(SMBIOS_TABLE_GUID, SMBIOS3_TABLE_GUID);
        assert_ne!(EFI_ACPI_20_TABLE_GUID, SMBIOS_TABLE_GUID);
    }
}
/* </TESTS> */
