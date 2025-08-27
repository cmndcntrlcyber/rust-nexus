# BOF/COFF Execution Guide

Rust-Nexus includes a comprehensive BOF (Beacon Object File) loader that enables execution of COFF (Common Object File Format) files, providing compatibility with the Cobalt Strike BOF ecosystem and custom Windows payloads.

## Overview

The BOF execution system consists of:
- **COFF Parser**: Complete COFF file format parser using the goblin crate
- **API Resolver**: Dynamic Windows API resolution for BOF imports
- **Memory Manager**: Safe memory allocation and protection management
- **Argument Marshalling**: Support for various argument types and calling conventions
- **Integration Layer**: Seamless integration with fiber execution and gRPC communication

## BOF Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   COFF File     │───►│   BOF Loader    │───►│   Execution     │
│                 │    │                 │    │                 │
│ • Sections      │    │ • Parse COFF    │    │ • Function Call │
│ • Symbols       │    │ • Load Sections │    │ • Argument Pass │
│ • Relocations   │    │ • Resolve APIs  │    │ • Result Return │
│ • Imports       │    │ • Apply Relocs  │    │ • Memory Clean  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
           │                     │                     │
           ▼                     ▼                     ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Raw Binary     │    │ Memory Mapped   │    │ Executed Code   │
│     Data        │    │   Executable    │    │   + Results     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Basic Usage

### Loading and Executing BOFs

```rust
use nexus_infra::{BOFLoader, BofArgument};

// Create BOF loader with Windows API resolution
let loader = BOFLoader::new();

// Load BOF from file
let bof_data = std::fs::read("whoami.obj")?;
let loaded_bof = loader.load_bof(&bof_data)?;

// Prepare arguments
let args = vec![
    BofArgument::string("target_computer"),
    BofArgument::int32(1),
];

// Execute BOF function
let result = loader.execute_bof(&loaded_bof, "go", &args)?;
println!("BOF Result: {}", result);
```

### Argument Types

The BOF loader supports all standard Cobalt Strike argument types:

```rust
// Integer arguments
let int32_arg = BofArgument::int32(12345);
let int16_arg = BofArgument::int16(789);

// String arguments (ANSI)
let str_arg = BofArgument::string("Hello, World!");

// Wide string arguments (UTF-16)
let wstr_arg = BofArgument::wide_string("Unicode String");

// Binary data
let binary_arg = BofArgument::binary(vec![0x41, 0x42, 0x43, 0x44]);

// Execute with mixed arguments
let args = vec![int32_arg, str_arg, wstr_arg, binary_arg];
let result = loader.execute_bof(&loaded_bof, "main", &args)?;
```

## BOF Development

### Creating Compatible BOFs

BOFs for Rust-Nexus should follow standard Cobalt Strike conventions:

```c
// example_bof.c
#include <windows.h>
#include <stdio.h>

// BOF API declarations
DECLSPEC_IMPORT KERNEL32 DWORD GetCurrentProcessId();
DECLSPEC_IMPORT KERNEL32 HANDLE GetCurrentProcess();
DECLSPEC_IMPORT KERNEL32 DWORD GetLastError();

// Entry point function
void go(char* args, int length) {
    // Parse arguments from buffer
    int arg_count = *(int*)args;
    args += 4;
    
    if (arg_count > 0) {
        // Read first argument (string)
        int str_len = *(int*)args;
        args += 4;
        char* target = args;
        args += str_len;
        
        printf("Target: %s\n", target);
    }
    
    // Perform BOF operations
    DWORD pid = GetCurrentProcessId();
    printf("Current PID: %lu\n", pid);
    
    // Return results through stdout or callback
}
```

### Compilation

```bash
# Compile BOF with MinGW or Visual Studio
x86_64-w64-mingw32-gcc -c example_bof.c -o example_bof.obj

# Or with Visual Studio Build Tools
cl.exe /c example_bof.c /Fo:example_bof.obj
```

### BOF Integration with gRPC

```rust
use nexus_infra::proto::*;

// Create BOF execution request
let bof_request = BofRequest {
    agent_id: "agent-123".to_string(),
    bof_data: std::fs::read("example_bof.obj")?,
    function_name: "go".to_string(),
    arguments: vec![
        BofArgument {
            r#type: 3, // String
            value: "target_system".as_bytes().to_vec(),
        },
    ],
    options: std::collections::HashMap::new(),
};

// Execute via gRPC
let response = grpc_client.execute_bof(bof_request).await?;
println!("BOF executed: {}", response.success);
```

## Advanced Features

### Custom API Resolution

```rust
// Add custom APIs to resolver
let mut loader = BOFLoader::new();
loader.add_api("CustomFunction", custom_function as usize);

// The BOF can now call CustomFunction
extern "C" fn custom_function(arg1: i32, arg2: *const u8) -> i32 {
    println!("Custom function called with: {}", arg1);
    0 // Success
}
```

### Memory Management

```rust
// BOF memory is automatically managed
{
    let loaded_bof = loader.load_bof(&bof_data)?;
    // Use the BOF
    let result = loader.execute_bof(&loaded_bof, "go", &args)?;
    
    // Memory is automatically freed when loaded_bof goes out of scope
}
```

### Error Handling

```rust
match loader.execute_bof(&loaded_bof, "go", &args) {
    Ok(result) => println!("Success: {}", result),
    Err(InfraError::BofError(msg)) => {
        eprintln!("BOF execution failed: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Integration with Fiber Execution

BOFs can be combined with fiber execution for enhanced stealth:

```rust
// Execute BOF within fiber context
let fiber_executor = FiberExecutor::new();

// Create fiber wrapper for BOF
let bof_shellcode = create_bof_wrapper(&loaded_bof, &args)?;
let result = fiber_executor.execute_direct_fiber(&bof_shellcode).await?;
```

## Supported BOF Categories

### **Information Gathering**
- System information collection
- Process enumeration
- Network interface discovery
- Registry queries
- File system enumeration

### **Credential Operations**
- LSASS dumping
- SAM database extraction
- Browser credential harvesting
- Kerberos ticket extraction
- Token manipulation

### **Lateral Movement**  
- SMB operations
- WMI execution
- Remote service management
- Share enumeration
- Remote registry access

### **Persistence**
- Registry key creation
- Scheduled task installation
- Service installation
- WMI event subscriptions
- COM object hijacking

### **Defense Evasion**
- Process hollowing
- DLL injection
- Token stealing
- Hook bypassing
- ETW evasion

## BOF Security Considerations

### **Safe Execution**
- BOFs run in controlled memory space
- Automatic cleanup prevents memory leaks
- API resolution limits available functions
- Execution timeouts prevent infinite loops

### **Validation**
- COFF format validation before loading
- Symbol resolution verification
- Memory boundary checking
- Function signature validation

### **Isolation**
- BOFs cannot access arbitrary memory
- API calls are mediated through resolver
- Error handling prevents crashes
- Resource limits prevent abuse

## Common BOF Patterns

### Information Collection BOF

```c
void go(char* args, int length) {
    // Get system information
    SYSTEM_INFO sysInfo;
    GetSystemInfo(&sysInfo);
    
    printf("Processors: %d\n", sysInfo.dwNumberOfProcessors);
    printf("Page size: %d\n", sysInfo.dwPageSize);
    printf("Architecture: %d\n", sysInfo.wProcessorArchitecture);
}
```

### Registry Query BOF

```c
void go(char* args, int length) {
    // Parse key path from arguments
    char* keyPath = parse_string_arg(&args);
    
    HKEY hKey;
    if (RegOpenKeyExA(HKEY_LOCAL_MACHINE, keyPath, 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
        // Query registry values
        DWORD dataSize = 1024;
        char data[1024];
        if (RegQueryValueExA(hKey, NULL, NULL, NULL, (LPBYTE)data, &dataSize) == ERROR_SUCCESS) {
            printf("Value: %s\n", data);
        }
        RegCloseKey(hKey);
    }
}
```

### Process Injection BOF

```c
void go(char* args, int length) {
    DWORD pid = parse_int32_arg(&args);
    char* shellcode = parse_binary_arg(&args, &shellcode_len);
    
    HANDLE hProcess = OpenProcess(PROCESS_ALL_ACCESS, FALSE, pid);
    if (hProcess) {
        LPVOID remoteMemory = VirtualAllocEx(hProcess, NULL, shellcode_len, 
                                           MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (remoteMemory) {
            WriteProcessMemory(hProcess, remoteMemory, shellcode, shellcode_len, NULL);
            // Continue with injection...
        }
        CloseHandle(hProcess);
    }
}
```

## Debugging BOFs

### Enable Debug Logging

```rust
// Enable debug logging for BOF operations
std::env::set_var("RUST_LOG", "nexus_infra::bof_loader=debug");
env_logger::init();

let loader = BOFLoader::new();
// BOF operations will now produce detailed debug output
```

### Memory Debugging

```rust
// Check available APIs before execution
let available_apis = loader.get_available_apis();
for api in &available_apis {
    println!("Available API: {}", api);
}

// Verify BOF symbols after loading
for (name, symbol) in &loaded_bof.symbols {
    println!("Symbol: {} -> 0x{:x} (function: {}, imported: {})", 
             name, symbol.address, symbol.is_function, symbol.is_imported);
}
```

## Performance Considerations

### **Memory Usage**
- BOFs are loaded into dedicated memory regions
- Automatic cleanup prevents memory leaks  
- Memory usage scales with BOF size
- Pool allocators for frequent BOF execution

### **Execution Speed**
- Direct function calls with minimal overhead
- API resolution cached for repeated calls
- Lazy loading of symbols and sections
- Optimized argument marshalling

### **Scalability**
- Multiple BOFs can execute concurrently
- Thread-safe API resolution
- Memory isolation between BOF instances
- Resource limits prevent system exhaustion

## Best Practices

### **BOF Development**
- Keep BOFs small and focused on single tasks
- Minimize dependencies on external libraries
- Use standard Windows APIs when possible
- Implement proper error handling
- Test thoroughly in isolated environments

### **Operational Use**
- Validate BOFs before deployment
- Monitor memory usage during execution
- Implement execution timeouts
- Log BOF execution for audit trails
- Use appropriate argument validation

### **Security**
- Review BOF source code for malicious content
- Test BOFs in sandboxed environments first
- Monitor API calls made by BOFs
- Implement allowlists for permitted operations
- Have incident response procedures for BOF failures

## Next Steps

1. **[Advanced Fiber Integration](fiber-advanced.md)** - Combining BOFs with fiber techniques
2. **[BOF Development Toolkit](coff-development.md)** - Tools and templates for BOF creation
3. **[Production BOF Library](../examples/bof-library/)** - Ready-to-use BOF collection
4. **[Security Guidelines](../configuration/security-hardening.md)** - BOF security best practices
