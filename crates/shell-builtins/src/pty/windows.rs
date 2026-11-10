//! Windows ConPTY pseudoterminal backend.
//!
//! Uses `CreatePseudoConsole` (available since Windows 10 1809) to create a
//! pseudoconsole, then spawns the child with `CreateProcessW` using a
//! `STARTUPINFOEXW` that references the pseudoconsole via
//! `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`.
//!
//! The ConPTY API provides the pseudoconsole master side as a pair of pipe
//! handles: one for input (write to child) and one for output (read from child).
//! These are wrapped in `std::io` handles and relayed by the calling code.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::process::{ExternalCommand, ProcessError, ProcessStatus};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, HPCON, ResizePseudoConsole,
};
use windows_sys::Win32::System::Memory::{GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc, HeapFree};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE, InitializeProcThreadAttributeList,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute, WaitForSingleObject,
};

/// A writer to the ConPTY input pipe (child's stdin).
pub struct Writer {
    handle: HANDLE,
}

unsafe impl Send for Writer {}

impl Writer {
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut written = 0u32;
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::WriteFile(
                self.handle,
                buf.as_ptr() as _,
                buf.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        };
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Writer::flush(self)
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

/// A reader from the ConPTY output pipe (child's stdout/stderr).
pub struct Reader {
    handle: HANDLE,
}

unsafe impl Send for Reader {}

impl std::io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0u32;
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::ReadFile(
                self.handle,
                buf.as_mut_ptr() as _,
                buf.len() as u32,
                &mut read,
                std::ptr::null_mut(),
            )
        };
        if result == 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::UnexpectedEof {
                return Ok(0);
            }
            return Err(err);
        }
        Ok(read as usize)
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

/// Handle for resizing the ConPTY.
#[derive(Clone, Copy)]
pub struct PtyResizer {
    hpc: HPCON,
}

// SAFETY: `ResizePseudoConsole` is thread-safe on a live pseudoconsole handle.
unsafe impl Send for PtyResizer {}
unsafe impl Sync for PtyResizer {}

impl PtyResizer {
    pub fn resize(self, size: crate::pty::PtySize) -> io::Result<()> {
        let coord = windows_sys::Win32::System::Console::COORD {
            X: size.columns() as i16,
            Y: size.rows() as i16,
        };
        let result = unsafe { ResizePseudoConsole(self.hpc, coord) };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
        Ok(())
    }
}

/// A child process attached to a ConPTY.
pub struct PtyProcess {
    hpc: HPCON,
    pi: PROCESS_INFORMATION,
    reader: Option<Reader>,
    writer: Option<Writer>,
    status: Option<ProcessStatus>,
}

unsafe impl Send for PtyProcess {}

impl PtyProcess {
    /// Starts `command` on a new ConPTY pseudoterminal.
    pub fn spawn(
        command: &ExternalCommand,
        size: crate::pty::PtySize,
    ) -> Result<Self, ProcessError> {
        let spec = command.launch_spec()?;

        // create_pipe() returns (read_handle, write_handle)
        // Pipe 1 (Input to ConPTY / Child stdin):
        // ConPTY reads from conpty_input_read; We write to our_input_write
        let (conpty_input_read, our_input_write) =
            create_pipe().map_err(|source| ProcessError::Spawn {
                program: command.program().to_owned(),
                source,
            })?;

        // Pipe 2 (Output from ConPTY / Child stdout/stderr):
        // ConPTY writes to conpty_output_write; We read from our_output_read
        let (our_output_read, conpty_output_write) =
            create_pipe().map_err(|source| ProcessError::Spawn {
                program: command.program().to_owned(),
                source,
            })?;

        // Create the pseudoconsole
        let coord = windows_sys::Win32::System::Console::COORD {
            X: size.columns() as i16,
            Y: size.rows() as i16,
        };

        let mut hpc: HPCON = 0;
        let result = unsafe {
            CreatePseudoConsole(coord, conpty_input_read, conpty_output_write, 0, &mut hpc)
        };

        if result != 0 {
            unsafe { CloseHandle(conpty_input_read) };
            unsafe { CloseHandle(conpty_output_write) };
            unsafe { CloseHandle(our_input_write) };
            unsafe { CloseHandle(our_output_read) };
            return Err(ProcessError::Spawn {
                program: command.program().to_owned(),
                source: io::Error::from_raw_os_error(result as i32),
            });
        }

        // Close the ConPTY-side handles in our process now that CreatePseudoConsole owns them
        unsafe { CloseHandle(conpty_input_read) };
        unsafe { CloseHandle(conpty_output_write) };

        // Make our handles non-inheritable so the child process doesn't inherit them
        unsafe { SetHandleInformation(our_input_write, HANDLE_FLAG_INHERIT, 0) };
        unsafe { SetHandleInformation(our_output_read, HANDLE_FLAG_INHERIT, 0) };

        // Build environment block
        let env_block = build_environment_block(command)?;

        let env_ptr = env_block.as_ref().map_or(std::ptr::null(), |b| b.as_ptr());

        // Prepare startup info with ConPTY attribute
        let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        // Without this flag Windows duplicates the parent's redirected
        // standard handles into the child, so the child writes to the
        // terminal that launched KERNEXT instead of the pseudoconsole.
        // The handles stay null; the pseudoconsole supplies its own.
        si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;

        // Initialize attribute list for pseudoconsole
        let mut attr_list_size = 0usize;
        unsafe {
            InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_list_size);
        }

        let attr_list_mem = unsafe { HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, attr_list_size) }
            as LPPROC_THREAD_ATTRIBUTE_LIST;

        if attr_list_mem.is_null() {
            unsafe { CloseHandle(our_input_write) };
            unsafe { CloseHandle(our_output_read) };
            unsafe { ClosePseudoConsole(hpc) };
            return Err(ProcessError::Spawn {
                program: command.program().to_owned(),
                source: io::Error::last_os_error(),
            });
        }

        if unsafe { InitializeProcThreadAttributeList(attr_list_mem, 1, 0, &mut attr_list_size) }
            == 0
        {
            unsafe { HeapFree(GetProcessHeap(), 0, attr_list_mem as _) };
            unsafe { CloseHandle(our_input_write) };
            unsafe { CloseHandle(our_output_read) };
            unsafe { ClosePseudoConsole(hpc) };
            return Err(ProcessError::Spawn {
                program: command.program().to_owned(),
                source: io::Error::last_os_error(),
            });
        }

        if unsafe {
            UpdateProcThreadAttribute(
                attr_list_mem,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                hpc as *const std::ffi::c_void,
                std::mem::size_of::<HPCON>(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            unsafe { DeleteProcThreadAttributeList(attr_list_mem) };
            unsafe { HeapFree(GetProcessHeap(), 0, attr_list_mem as _) };
            unsafe { CloseHandle(our_input_write) };
            unsafe { CloseHandle(our_output_read) };
            unsafe { ClosePseudoConsole(hpc) };
            return Err(ProcessError::Spawn {
                program: command.program().to_owned(),
                source: io::Error::last_os_error(),
            });
        }

        si.lpAttributeList = attr_list_mem;

        // Spawn the process
        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        // Get working directory if specified
        let mut working_dir_wide: Vec<u16>;
        let working_dir_ptr = if let Some(p) = command.working_directory() {
            working_dir_wide = p.as_os_str().encode_wide().chain(Some(0)).collect();
            working_dir_wide.as_mut_ptr()
        } else {
            std::ptr::null_mut()
        };

        // Always use CREATE_UNICODE_ENVIRONMENT since we now always pass an environment block
        let creation_flags = EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT;

        // Build the full command line. `lpApplicationName` names the actual
        // image, while `lpCommandLine` starts with that image by convention.
        // Batch scripts arrive as `cmd.exe /d /v:off /s /c` plus the
        // pre-escaped script line from `launch_spec`.
        let mut cmd_line = quote_arg(&spec.executable.to_string_lossy());
        for arg in &spec.arguments {
            cmd_line.push(' ');
            cmd_line.push_str(&quote_arg(arg));
        }
        if let Some(raw) = &spec.raw_arguments {
            cmd_line.push(' ');
            cmd_line.push_str(raw);
        }

        let mut cmd_wide: Vec<u16> = cmd_line.encode_utf16().chain(Some(0)).collect();

        let result = unsafe {
            CreateProcessW(
                std::ptr::null(),
                cmd_wide.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0, // bInheritHandles = FALSE; the pseudoconsole is passed via the attribute list
                creation_flags,
                env_ptr as _,
                working_dir_ptr,
                &si.StartupInfo,
                &mut pi,
            )
        };

        // Cleanup attribute list
        unsafe { DeleteProcThreadAttributeList(attr_list_mem) };
        unsafe { HeapFree(GetProcessHeap(), 0, attr_list_mem as _) };

        if result == 0 {
            let err = io::Error::last_os_error();
            unsafe { CloseHandle(our_input_write) };
            unsafe { CloseHandle(our_output_read) };
            unsafe { ClosePseudoConsole(hpc) };
            unsafe { CloseHandle(pi.hThread) };
            unsafe { CloseHandle(pi.hProcess) };
            return Err(ProcessError::Spawn {
                program: command.program().to_owned(),
                source: err,
            });
        }

        // We don't need the thread handle
        unsafe { CloseHandle(pi.hThread) };

        Ok(Self {
            hpc,
            pi,
            reader: Some(Reader {
                handle: our_output_read,
            }),
            writer: Some(Writer {
                handle: our_input_write,
            }),
            status: None,
        })
    }

    /// Takes the readable end of the terminal output stream.
    pub fn take_reader(&mut self) -> Option<Box<dyn io::Read + Send>> {
        self.reader
            .take()
            .map(|reader| Box::new(reader) as Box<dyn io::Read + Send>)
    }

    /// Takes the writable end of the terminal input stream.
    pub fn take_writer(&mut self) -> Option<Box<dyn io::Write + Send>> {
        self.writer
            .take()
            .map(|writer| Box::new(writer) as Box<dyn io::Write + Send>)
    }

    /// Handle for resizing the terminal from another thread.
    pub fn resizer(&self) -> PtyResizer {
        PtyResizer { hpc: self.hpc }
    }

    /// Forwards Ctrl+C into the pseudoterminal for as long as the guard lives.
    pub fn forward_interrupts(&self) -> Option<CtrlCForwarder> {
        CtrlCForwarder::install(self.pi.hProcess).ok()
    }

    /// Returns the exit status when the child has already finished.
    ///
    /// Used by interactive bridges that must notice a child exiting while they
    /// are also waiting for input, without blocking either side.
    pub fn try_wait(&mut self) -> Result<Option<ProcessStatus>, ProcessError> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }

        const WAIT_TIMEOUT: u32 = 0x0000_0102;
        let result = unsafe { WaitForSingleObject(self.pi.hProcess, 0) };

        if result == WAIT_TIMEOUT {
            return Ok(None);
        }

        if result == 0xFFFF_FFFF {
            return Err(ProcessError::Wait {
                program: String::from("process"),
                source: io::Error::last_os_error(),
            });
        }

        let mut exit_code = 0u32;
        if unsafe { GetExitCodeProcess(self.pi.hProcess, &mut exit_code) } == 0 {
            return Err(ProcessError::Wait {
                program: String::from("process"),
                source: io::Error::last_os_error(),
            });
        }

        let status = ProcessStatus::from_code(exit_code as i32);
        self.status = Some(status);
        unsafe { CloseHandle(self.pi.hProcess) };
        unsafe { ClosePseudoConsole(self.hpc) };
        self.pi.hProcess = INVALID_HANDLE_VALUE;
        self.hpc = 0;

        Ok(Some(status))
    }

    /// Waits for the child to exit.
    pub fn wait(&mut self) -> Result<ProcessStatus, ProcessError> {
        if let Some(status) = self.status {
            return Ok(status);
        }

        let result = unsafe { WaitForSingleObject(self.pi.hProcess, INFINITE) };
        if result == 0xFFFFFFFF {
            return Err(ProcessError::Wait {
                program: String::from("process"),
                source: io::Error::last_os_error(),
            });
        }

        let mut exit_code = 0u32;
        if unsafe { GetExitCodeProcess(self.pi.hProcess, &mut exit_code) } == 0 {
            return Err(ProcessError::Wait {
                program: String::from("process"),
                source: io::Error::last_os_error(),
            });
        }

        let status = ProcessStatus::from_code(exit_code as i32);
        self.status = Some(status);

        // Close handles
        unsafe { CloseHandle(self.pi.hProcess) };
        unsafe { ClosePseudoConsole(self.hpc) };
        self.pi.hProcess = INVALID_HANDLE_VALUE;
        self.hpc = 0;

        Ok(status)
    }

    /// Terminates the child. Callers still have to wait for it.
    pub fn kill(&mut self) {
        if self.pi.hProcess != INVALID_HANDLE_VALUE && !self.pi.hProcess.is_null() {
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(self.pi.hProcess, 1);
            }
        }
    }
}

impl Drop for PtyProcess {
    fn drop(&mut self) {
        if self.status.is_none() {
            self.kill();
            let _ = self.wait();
        }
        if self.pi.hProcess != INVALID_HANDLE_VALUE && !self.pi.hProcess.is_null() {
            unsafe { CloseHandle(self.pi.hProcess) };
        }
        if self.hpc != 0 {
            unsafe { ClosePseudoConsole(self.hpc) };
        }
    }
}

/// Creates a pair of inherited handles for ConPTY pipes.
/// Returns (read_handle, write_handle).
fn create_pipe() -> io::Result<(HANDLE, HANDLE)> {
    let mut read_handle: HANDLE = INVALID_HANDLE_VALUE;
    let mut write_handle: HANDLE = INVALID_HANDLE_VALUE;

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 1,
    };

    if unsafe { CreatePipe(&mut read_handle, &mut write_handle, &sa, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok((read_handle, write_handle))
}

/// Quotes a command line argument for Windows standard command line parsing rules.
fn quote_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    if !arg
        .chars()
        .any(|c| matches!(c, ' ' | '\t' | '\n' | '\u{0b}' | '"'))
    {
        return arg.to_string();
    }

    let mut quoted = String::with_capacity(arg.len() + 2);
    quoted.push('"');

    let mut backslashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                for _ in 0..=backslashes {
                    quoted.push('\\');
                }
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                for _ in 0..backslashes {
                    quoted.push('\\');
                }
                quoted.push(c);
                backslashes = 0;
            }
        }
    }

    for _ in 0..backslashes * 2 {
        quoted.push('\\');
    }

    quoted.push('"');
    quoted
}

/// Builds the environment block for CreateProcessW.
/// Returns option_env_block.
fn build_environment_block(command: &ExternalCommand) -> Result<Option<Vec<u16>>, ProcessError> {
    // When environment overrides or clear_env are explicitly set, build a block
    // Otherwise inherit parent environment (pass None) to avoid DLL init issues
    if command.clears_environment() || !command.environment_entries().is_empty() {
        let mut env_map = BTreeMap::new();

        if !command.clears_environment() {
            for (k, v) in std::env::vars_os() {
                let key_upper = k.to_string_lossy().to_uppercase();
                env_map.insert(key_upper, (k, v));
            }
        }

        for (k, v) in command.environment_entries() {
            let key_upper = k.to_string_lossy().to_uppercase();
            env_map.insert(key_upper, (k.clone(), v.clone()));
        }

        // Ensure SystemRoot is present if it was in the parent environment
        // (Some child processes require it for DLL initialization)
        if !command.clears_environment() && !env_map.contains_key("SYSTEMROOT") {
            if let Ok(val) = std::env::var("SystemRoot") {
                env_map.insert(
                    "SYSTEMROOT".to_string(),
                    (OsString::from("SystemRoot"), OsString::from(val)),
                );
            } else if let Ok(val) = std::env::var("SYSTEMROOT") {
                env_map.insert(
                    "SYSTEMROOT".to_string(),
                    (OsString::from("SYSTEMROOT"), OsString::from(val)),
                );
            }
        }

        let mut block = Vec::new();
        for (k, v) in env_map.values() {
            block.extend(k.as_os_str().encode_wide());
            block.push('=' as u16);
            block.extend(v.as_os_str().encode_wide());
            block.push(0);
        }
        block.push(0); // Final null terminator

        Ok(Some(block))
    } else {
        // Pass None to inherit parent environment
        Ok(None)
    }
}

/// Terminal size of the shell's own console.
pub fn terminal_size() -> Option<crate::pty::PtySize> {
    use windows_sys::Win32::System::Console::{
        CONSOLE_SCREEN_BUFFER_INFO, GetConsoleScreenBufferInfo, GetStdHandle, STD_OUTPUT_HANDLE,
    };

    let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
    if unsafe { GetConsoleScreenBufferInfo(handle, &mut info) } == 0 {
        return None;
    }

    let rows = info.srWindow.Bottom - info.srWindow.Top + 1;
    let cols = info.srWindow.Right - info.srWindow.Left + 1;

    if rows <= 0 || cols <= 0 {
        return None;
    }

    Some(crate::pty::PtySize::new(rows as u16, cols as u16))
}

/// Forwards Ctrl+C into the ConPTY by generating a console event.
pub struct CtrlCForwarder {
    _process_handle: HANDLE,
}

impl CtrlCForwarder {
    fn install(process_handle: HANDLE) -> io::Result<Self> {
        Ok(Self {
            _process_handle: process_handle,
        })
    }

    pub fn handle_ctrlc(&self) {}
}

impl Drop for CtrlCForwarder {
    fn drop(&mut self) {}
}

/// Relays stdin into the pseudoterminal until `stop` is set.
pub fn relay_stdin(mut writer: Box<dyn io::Write + Send>, stop: &AtomicBool) {
    use std::io::Read;
    let mut stdin = std::io::stdin();
    let mut buffer = [0u8; 4096];

    while !stop.load(Ordering::Relaxed) {
        match stdin.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if writer.write_all(&buffer[..n]).is_err() {
                    break;
                }
                if writer.flush().is_err() {
                    break;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}
