use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=bofs/keylogger/nexus_keylogger.c");
    println!("cargo:rerun-if-changed=bofs/keylogger/Makefile");

    // Only build BOF on Windows targets
    if cfg!(target_os = "windows") {
        build_keylogger_bof();
    } else {
        // Create empty file for non-Windows builds
        create_empty_bof_placeholder();
    }
}

#[cfg(target_os = "windows")]
fn build_keylogger_bof() {
    use std::fs;
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    let keylogger_dir = PathBuf::from(&manifest_dir).join("bofs").join("keylogger");
    let out_path = PathBuf::from(&out_dir);
    
    println!("Building keylogger BOF from: {:?}", keylogger_dir);
    println!("Output directory: {:?}", out_path);
    
    // Check if we have the required tools
    let has_msvc = Command::new("cl.exe")
        .arg("/?")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    
    if !has_msvc {
        println!("cargo:warning=Microsoft C/C++ compiler (cl.exe) not found. Keylogger BOF will not be available.");
        create_empty_bof_placeholder();
        return;
    }
    
    // Compile the keylogger BOF
    let output = Command::new("cl.exe")
        .current_dir(&keylogger_dir)
        .args(&[
            "/c",                    // Compile only, don't link
            "/GS-",                 // Disable security checks
            "/Gs9999999",           // Disable stack checking
            "/O2",                  // Optimize for speed
            "/MT",                  // Static runtime
            "/kernel",              // Kernel mode (minimal runtime)
            "nexus_keylogger.c",
            &format!("/Fo:{}", out_path.join("nexus_keylogger.obj").display()),
        ])
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("Successfully compiled keylogger BOF");
                
                // Copy the object file to the expected location
                let obj_src = out_path.join("nexus_keylogger.obj");
                let obj_dst = out_path.join("nexus_keylogger.o");
                
                if obj_src.exists() {
                    if let Err(e) = fs::copy(&obj_src, &obj_dst) {
                        println!("cargo:warning=Failed to copy BOF object file: {}", e);
                        create_empty_bof_placeholder();
                    } else {
                        println!("BOF object file created at: {:?}", obj_dst);
                    }
                } else {
                    println!("cargo:warning=BOF object file not found at expected location");
                    create_empty_bof_placeholder();
                }
            } else {
                println!("cargo:warning=Failed to compile keylogger BOF:");
                println!("cargo:warning=stdout: {}", String::from_utf8_lossy(&output.stdout));
                println!("cargo:warning=stderr: {}", String::from_utf8_lossy(&output.stderr));
                create_empty_bof_placeholder();
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute cl.exe: {}", e);
            create_empty_bof_placeholder();
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn build_keylogger_bof() {
    create_empty_bof_placeholder();
}

fn create_empty_bof_placeholder() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let placeholder_path = PathBuf::from(out_dir).join("nexus_keylogger.o");
    
    // Create an empty file as placeholder
    std::fs::write(&placeholder_path, &[]).unwrap_or_else(|e| {
        panic!("Failed to create BOF placeholder file: {}", e);
    });
    
    println!("Created empty BOF placeholder at: {:?}", placeholder_path);
}
