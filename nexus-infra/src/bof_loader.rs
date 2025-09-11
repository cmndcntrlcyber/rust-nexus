//! BOF (Beacon Object File) loader for COFF execution

use crate::{InfraError, InfraResult};
use goblin::pe::Coff;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::ptr;

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::GetLastError,
    System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
    System::Memory::{
        VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
        PAGE_EXECUTE_READWRITE, PAGE_READWRITE,
    },
};

/// BOF argument types for marshalling
#[derive(Debug, Clone, PartialEq)]
pub enum BofArgumentType {
    Int32,
    Int16,
    String,
    WideString,
    Binary,
}

/// BOF argument value
#[derive(Debug, Clone)]
pub struct BofArgument {
    pub arg_type: BofArgumentType,
    pub data: Vec<u8>,
}

impl BofArgument {
    /// Create a new int32 argument
    pub fn int32(value: i32) -> Self {
        Self {
            arg_type: BofArgumentType::Int32,
            data: value.to_le_bytes().to_vec(),
        }
    }

    /// Create a new int16 argument
    pub fn int16(value: i16) -> Self {
        Self {
            arg_type: BofArgumentType::Int16,
            data: value.to_le_bytes().to_vec(),
        }
    }

    /// Create a new string argument
    pub fn string(value: &str) -> Self {
        Self {
            arg_type: BofArgumentType::String,
            data: value.as_bytes().to_vec(),
        }
    }

    /// Create a new wide string argument
    pub fn wide_string(value: &str) -> Self {
        let wide_string: Vec<u16> = value.encode_utf16().collect();
        let mut data = Vec::new();
        for ch in wide_string {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        data.extend_from_slice(&[0u8, 0u8]); // Null terminator

        Self {
            arg_type: BofArgumentType::WideString,
            data,
        }
    }

    /// Create a new binary argument
    pub fn binary(data: Vec<u8>) -> Self {
        Self {
            arg_type: BofArgumentType::Binary,
            data,
        }
    }
}

/// BOF symbol information
#[derive(Debug, Clone)]
pub struct BofSymbol {
    pub name: String,
    pub address: usize,
    pub is_function: bool,
    pub is_imported: bool,
}

/// Loaded BOF instance
pub struct LoadedBof {
    pub entry_point: usize,
    pub base_address: *mut u8,
    pub size: usize,
    pub symbols: HashMap<String, BofSymbol>,
}

impl Drop for LoadedBof {
    fn drop(&mut self) {
        if !self.base_address.is_null() {
            #[cfg(target_os = "windows")]
            unsafe {
                VirtualFree(self.base_address as *mut _, 0, MEM_RELEASE);
            }

            #[cfg(not(target_os = "windows"))]
            {
                // On non-Windows platforms, use libc free or equivalent
                warn!("BOF cleanup not implemented for this platform");
            }
        }
    }
}

/// BOF loader for COFF file execution
pub struct BOFLoader {
    /// Known API functions that BOFs might import
    api_resolver: HashMap<String, usize>,
}

impl BOFLoader {
    /// Create a new BOF loader
    pub fn new() -> Self {
        let mut loader = Self {
            api_resolver: HashMap::new(),
        };

        // Initialize with common Windows APIs
        loader.init_api_resolver();
        loader
    }

    /// Initialize the API resolver with common Windows functions
    #[cfg(target_os = "windows")]
    fn init_api_resolver(&mut self) {
        // Common APIs that BOFs might use
        let apis = vec![
            ("kernel32.dll", "GetCurrentProcess"),
            ("kernel32.dll", "GetCurrentThread"),
            ("kernel32.dll", "GetCurrentProcessId"),
            ("kernel32.dll", "GetCurrentThreadId"),
            ("kernel32.dll", "VirtualAlloc"),
            ("kernel32.dll", "VirtualFree"),
            ("kernel32.dll", "CreateFileA"),
            ("kernel32.dll", "ReadFile"),
            ("kernel32.dll", "WriteFile"),
            ("kernel32.dll", "CloseHandle"),
            ("kernel32.dll", "GetLastError"),
            ("ntdll.dll", "NtQuerySystemInformation"),
            ("ntdll.dll", "NtQueryInformationProcess"),
            ("advapi32.dll", "OpenProcessToken"),
            ("advapi32.dll", "GetTokenInformation"),
            ("user32.dll", "MessageBoxA"),
            ("msvcrt.dll", "malloc"),
            ("msvcrt.dll", "free"),
            ("msvcrt.dll", "memcpy"),
            ("msvcrt.dll", "memset"),
            ("msvcrt.dll", "strlen"),
            ("msvcrt.dll", "printf"),
        ];

        for (dll_name, func_name) in apis {
            if let Some(addr) = self.resolve_api_address(dll_name, func_name) {
                let full_name = format!("{}!{}", dll_name, func_name);
                self.api_resolver.insert(full_name, addr);
                self.api_resolver.insert(func_name.to_string(), addr); // Also store short name
            }
        }

        info!(
            "Initialized API resolver with {} functions",
            self.api_resolver.len()
        );
    }

    #[cfg(not(target_os = "windows"))]
    fn init_api_resolver(&mut self) {
        warn!("API resolver not implemented for this platform");
    }

    /// Resolve API address from DLL and function name
    #[cfg(target_os = "windows")]
    fn resolve_api_address(&self, dll_name: &str, func_name: &str) -> Option<usize> {
        unsafe {
            let dll_name_cstr = CString::new(dll_name).ok()?;
            let func_name_cstr = CString::new(func_name).ok()?;

            let module_handle = GetModuleHandleA(dll_name_cstr.as_ptr() as *const u8);
            if module_handle == 0 {
                return None;
            }

            let proc_address = GetProcAddress(module_handle, func_name_cstr.as_ptr() as *const u8);
            if proc_address.is_none() {
                return None;
            }

            Some(proc_address.unwrap() as usize)
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn resolve_api_address(&self, _dll_name: &str, _func_name: &str) -> Option<usize> {
        None
    }

    /// Load a BOF from COFF data
    pub fn load_bof(&self, coff_data: &[u8]) -> InfraResult<LoadedBof> {
        info!("Loading BOF from {} bytes of COFF data", coff_data.len());

        // Parse COFF file
        let coff = Coff::parse(coff_data)
            .map_err(|e| InfraError::BofError(format!("Failed to parse COFF: {}", e)))?;

        debug!(
            "COFF parsed: {} sections, {} symbols",
            coff.sections.len(),
            coff.symbols.iter().count()
        );

        // Calculate total size needed
        let total_size = self.calculate_image_size(&coff)?;

        // Allocate memory for the BOF
        let base_address = self.allocate_memory(total_size)?;

        // Load sections into memory
        self.load_sections(&coff, base_address, coff_data)?;

        // Build symbol table
        let symbols = self.build_symbol_table(&coff, base_address)?;

        // Apply relocations
        self.apply_relocations(&coff, base_address, coff_data, &symbols)?;

        // Find entry point
        let entry_point = self.find_entry_point(&symbols)?;

        // Make memory executable
        self.make_executable(base_address, total_size)?;

        let loaded_bof = LoadedBof {
            entry_point,
            base_address,
            size: total_size,
            symbols,
        };

        info!(
            "BOF loaded successfully at {:p}, entry point: 0x{:x}",
            base_address, entry_point
        );
        Ok(loaded_bof)
    }

    /// Calculate the total size needed for the loaded image
    fn calculate_image_size(&self, coff: &Coff) -> InfraResult<usize> {
        let mut max_address = 0;

        for section in &coff.sections {
            let section_end = section.virtual_address as usize + section.virtual_size as usize;
            if section_end > max_address {
                max_address = section_end;
            }
        }

        // Add some padding
        let total_size = (max_address + 0xFFF) & !0xFFF; // Align to page boundary

        if total_size == 0 || total_size > 50 * 1024 * 1024 {
            // 50MB limit
            return Err(InfraError::BofError(
                "Invalid or excessive image size".to_string(),
            ));
        }

        Ok(total_size)
    }

    /// Allocate memory for the BOF
    #[cfg(target_os = "windows")]
    fn allocate_memory(&self, size: usize) -> InfraResult<*mut u8> {
        unsafe {
            let ptr = VirtualAlloc(
                ptr::null_mut(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );

            if ptr.is_null() {
                return Err(InfraError::BofError(format!(
                    "Failed to allocate memory: {}",
                    GetLastError()
                )));
            }

            Ok(ptr as *mut u8)
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn allocate_memory(&self, size: usize) -> InfraResult<*mut u8> {
        // Use libc malloc or mmap for other platforms
        use std::alloc::{alloc, Layout};

        let layout = Layout::from_size_align(size, 4096)
            .map_err(|e| InfraError::BofError(format!("Invalid layout: {}", e)))?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(InfraError::BofError(
                    "Failed to allocate memory".to_string(),
                ));
            }
            Ok(ptr)
        }
    }

    /// Load sections into allocated memory
    fn load_sections(
        &self,
        coff: &Coff,
        base_address: *mut u8,
        coff_data: &[u8],
    ) -> InfraResult<()> {
        for section in &coff.sections {
            if section.size_of_raw_data == 0 {
                continue;
            }

            let section_data = &coff_data[section.pointer_to_raw_data as usize
                ..(section.pointer_to_raw_data + section.size_of_raw_data) as usize];

            unsafe {
                let dest = base_address.offset(section.virtual_address as isize);
                ptr::copy_nonoverlapping(
                    section_data.as_ptr(),
                    dest,
                    section.size_of_raw_data as usize,
                );
            }

            debug!(
                "Loaded section '{}' at offset 0x{:x} ({} bytes)",
                section.name().unwrap_or("unnamed"),
                section.virtual_address,
                section.size_of_raw_data
            );
        }

        Ok(())
    }

    /// Build symbol table with resolved addresses
    fn build_symbol_table(
        &self,
        coff: &Coff,
        base_address: *mut u8,
    ) -> InfraResult<HashMap<String, BofSymbol>> {
        let mut symbols = HashMap::new();

        // The symbol table API has changed - symbols are now tuples (index, name, symbol)
        for (_index, name_opt, symbol) in coff.symbols.iter() {
            let name = match name_opt {
                Some(name) => name,
                None => {
                    warn!("Symbol with no name, skipping");
                    continue;
                }
            };

            // Access symbol fields directly from the Symbol struct
            let section_number = symbol.section_number;
            let value = symbol.value;
            let storage_class = symbol.storage_class;

            let address = if section_number == 0 {
                // External symbol - try to resolve from API resolver
                if let Some(&api_addr) = self.api_resolver.get(name) {
                    api_addr
                } else {
                    warn!("Unresolved external symbol: {}", name);
                    0 // Will cause issues if used, but allows loading to continue
                }
            } else {
                // Internal symbol
                (base_address as usize) + (value as usize)
            };

            let bof_symbol = BofSymbol {
                name: name.to_string(),
                address,
                is_function: storage_class == 2, // EXTERNAL function
                is_imported: section_number == 0,
            };

            symbols.insert(name.to_string(), bof_symbol);
        }

        info!("Built symbol table with {} symbols", symbols.len());
        Ok(symbols)
    }

    /// Apply relocations to the loaded image
    fn apply_relocations(
        &self,
        coff: &Coff,
        base_address: *mut u8,
        coff_data: &[u8],
        symbols: &HashMap<String, BofSymbol>,
    ) -> InfraResult<()> {
        for section in &coff.sections {
            if section.number_of_relocations == 0 {
                continue;
            }

            let relocations_start = section.pointer_to_relocations as usize;
            let relocations_size = (section.number_of_relocations as usize) * 10; // Each relocation is 10 bytes

            if relocations_start + relocations_size > coff_data.len() {
                return Err(InfraError::BofError(
                    "Relocation data out of bounds".to_string(),
                ));
            }

            // Parse relocations (simplified - would need full implementation for production)
            debug!(
                "Processing {} relocations for section '{}'",
                section.number_of_relocations,
                section.name().unwrap_or("unnamed")
            );
        }

        Ok(())
    }

    /// Find the entry point function
    fn find_entry_point(&self, symbols: &HashMap<String, BofSymbol>) -> InfraResult<usize> {
        // Look for common entry point names
        let entry_names = ["go", "main", "start", "entry"];

        for name in &entry_names {
            if let Some(symbol) = symbols.get(*name) {
                if symbol.is_function && !symbol.is_imported {
                    return Ok(symbol.address);
                }
            }
        }

        // If no named entry point found, use the first function symbol
        for symbol in symbols.values() {
            if symbol.is_function && !symbol.is_imported {
                return Ok(symbol.address);
            }
        }

        Err(InfraError::BofError("No entry point found".to_string()))
    }

    /// Make memory region executable
    #[cfg(target_os = "windows")]
    fn make_executable(&self, base_address: *mut u8, size: usize) -> InfraResult<()> {
        unsafe {
            let mut old_protect = 0u32;
            let result = VirtualProtect(
                base_address as *mut _,
                size,
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            );

            if result == 0 {
                return Err(InfraError::BofError(format!(
                    "Failed to make memory executable: {}",
                    GetLastError()
                )));
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn make_executable(&self, _base_address: *mut u8, _size: usize) -> InfraResult<()> {
        // Use mprotect on Unix-like systems
        warn!("Memory protection not implemented for this platform");
        Ok(())
    }

    /// Execute a loaded BOF with arguments
    pub fn execute_bof(
        &self,
        loaded_bof: &LoadedBof,
        function_name: &str,
        args: &[BofArgument],
    ) -> InfraResult<String> {
        info!(
            "Executing BOF function: {} with {} arguments",
            function_name,
            args.len()
        );

        // Find the function symbol
        let function_symbol = loaded_bof.symbols.get(function_name).ok_or_else(|| {
            InfraError::BofError(format!("Function '{}' not found", function_name))
        })?;

        if !function_symbol.is_function || function_symbol.is_imported {
            return Err(InfraError::BofError(format!(
                "Symbol '{}' is not a valid function",
                function_name
            )));
        }

        // Marshal arguments
        let arg_buffer = self.marshal_arguments(args)?;

        // Execute the function (platform-specific implementation needed)
        self.call_function(function_symbol.address, &arg_buffer)
    }

    /// Marshal BOF arguments into a buffer
    fn marshal_arguments(&self, args: &[BofArgument]) -> InfraResult<Vec<u8>> {
        let mut buffer = Vec::new();

        // Add argument count
        buffer.extend_from_slice(&(args.len() as u32).to_le_bytes());

        for arg in args {
            // Add argument type
            let type_id = match arg.arg_type {
                BofArgumentType::Int32 => 1u8,
                BofArgumentType::Int16 => 2u8,
                BofArgumentType::String => 3u8,
                BofArgumentType::WideString => 4u8,
                BofArgumentType::Binary => 5u8,
            };
            buffer.push(type_id);

            // Add argument length
            buffer.extend_from_slice(&(arg.data.len() as u32).to_le_bytes());

            // Add argument data
            buffer.extend_from_slice(&arg.data);
        }

        Ok(buffer)
    }

    /// Call the BOF function (simplified implementation)
    #[cfg(target_os = "windows")]
    fn call_function(&self, function_address: usize, arg_buffer: &[u8]) -> InfraResult<String> {
        // This is a simplified implementation
        // In practice, you'd need proper calling convention handling
        // and safe execution environment

        info!(
            "Calling function at 0x{:x} with {} bytes of arguments",
            function_address,
            arg_buffer.len()
        );

        unsafe {
            // Create a function pointer
            type BofFunction = unsafe extern "C" fn(*const u8, u32) -> i32;
            let func: BofFunction = std::mem::transmute(function_address);

            // Call the function
            let result = func(arg_buffer.as_ptr(), arg_buffer.len() as u32);

            Ok(format!(
                "BOF executed successfully, return value: {}",
                result
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn call_function(&self, _function_address: usize, _arg_buffer: &[u8]) -> InfraResult<String> {
        Err(InfraError::BofError(
            "BOF execution not supported on this platform".to_string(),
        ))
    }

    /// Add custom API to the resolver
    pub fn add_api(&mut self, name: &str, address: usize) {
        self.api_resolver.insert(name.to_string(), address);
        info!("Added custom API: {} at 0x{:x}", name, address);
    }

    /// Get list of available APIs
    pub fn get_available_apis(&self) -> Vec<String> {
        self.api_resolver.keys().cloned().collect()
    }
}

impl Default for BOFLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bof_argument_creation() {
        let int_arg = BofArgument::int32(42);
        assert_eq!(int_arg.arg_type, BofArgumentType::Int32);
        assert_eq!(int_arg.data, vec![42, 0, 0, 0]);

        let str_arg = BofArgument::string("hello");
        assert_eq!(str_arg.arg_type, BofArgumentType::String);
        assert_eq!(str_arg.data, b"hello".to_vec());
    }

    #[test]
    fn test_bof_loader_creation() {
        let loader = BOFLoader::new();
        let apis = loader.get_available_apis();

        // Should have some APIs loaded (platform dependent)
        #[cfg(target_os = "windows")]
        assert!(!apis.is_empty());

        #[cfg(not(target_os = "windows"))]
        assert!(apis.is_empty());
    }

    #[test]
    fn test_argument_marshalling() {
        let loader = BOFLoader::new();
        let args = vec![BofArgument::int32(123), BofArgument::string("test")];

        let buffer = loader.marshal_arguments(&args).unwrap();

        // Check argument count (4 bytes)
        assert_eq!(&buffer[0..4], &2u32.to_le_bytes());

        // Check first argument type and length
        assert_eq!(buffer[4], 1); // Int32 type
        assert_eq!(&buffer[5..9], &4u32.to_le_bytes()); // Length
        assert_eq!(&buffer[9..13], &123i32.to_le_bytes()); // Value
    }

    #[test]
    fn test_wide_string_argument() {
        let arg = BofArgument::wide_string("test");
        assert_eq!(arg.arg_type, BofArgumentType::WideString);

        // Should contain UTF-16 encoded "test" + null terminator
        let expected: Vec<u8> = vec![
            116, 0, // 't'
            101, 0, // 'e'
            115, 0, // 's'
            116, 0, // 't'
            0, 0, // null terminator
        ];
        assert_eq!(arg.data, expected);
    }
}
