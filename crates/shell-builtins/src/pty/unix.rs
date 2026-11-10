//! Unix pseudoterminal backend.
//!
//! The master side is opened with `posix_openpt`, the slave is made a
//! controlling terminal in the child (`setsid` + `TIOCSCTTY`), and the child's
//! standard streams are connected to the slave. Output and input are relayed
//! through the master descriptor.

use super::PtySize;
use crate::process::{ExternalCommand, LaunchSpec, ProcessError, ProcessStatus};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// A cloneable handle for changing the terminal size from the monitor thread.
#[derive(Clone, Copy)]
pub struct PtyResizer {
    master: i32,
}

// SAFETY: `ioctl(TIOCSWINSZ)` on a live terminal descriptor is thread-safe
// enough for resizing, and the descriptor is kept open by the owning
// `PtyProcess` for as long as a resizer exists.
unsafe impl Send for PtyResizer {}
unsafe impl Sync for PtyResizer {}

impl PtyResizer {
    /// Informs the pseudoterminal that its window changed size.
    ///
    /// # Errors
    ///
    /// Returns the operating-system failure. A refused resize is not fatal.
    pub fn resize(self, size: PtySize) -> io::Result<()> {
        let window = libc::winsize {
            ws_row: size.rows(),
            ws_col: size.columns(),
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        // SAFETY: the descriptor is a live terminal and `window` is a valid
        // `winsize` for the duration of the call.
        if unsafe { libc::ioctl(self.master, libc::TIOCSWINSZ, &window) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

/// A child process attached to a pseudoterminal.
pub struct PtyProcess {
    master: File,
    child: Child,
    status: Option<ProcessStatus>,
}

impl PtyProcess {
    /// Starts `command` on a new pseudoterminal.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError`] when resolution fails, the pseudoterminal cannot
    /// be created, or the child cannot be started.
    pub fn spawn(command: &ExternalCommand, size: PtySize) -> Result<Self, ProcessError> {
        let spec = command.launch_spec()?;

        let (master, slave) = open_terminal(size).map_err(|source| ProcessError::Spawn {
            program: command.program().to_owned(),
            source,
        })?;

        let stdin = slave.try_clone().map_err(|source| ProcessError::Spawn {
            program: command.program().to_owned(),
            source,
        })?;
        let stdout = slave.try_clone().map_err(|source| ProcessError::Spawn {
            program: command.program().to_owned(),
            source,
        })?;

        let mut process = build_command(command, &spec);

        process
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(slave));

        // SAFETY: the closure only calls async-signal-safe functions
        // (`setsid`, `ioctl`) and reports failures through its return value.
        unsafe {
            use std::os::unix::process::CommandExt;

            process.pre_exec(|| {
                // A new session makes this process group the foreground group of
                // the new terminal, which is what lets it receive SIGINT.
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::ioctl(0, libc::TIOCSCTTY as libc::c_ulong, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok(())
            });
        }

        let child = process.spawn().map_err(|source| ProcessError::Spawn {
            program: command.program().to_owned(),
            source,
        })?;

        Ok(Self {
            master,
            child,
            status: None,
        })
    }

    /// Takes the readable end of the terminal output stream.
    pub fn take_reader(&mut self) -> Option<Box<dyn Read + Send>> {
        self.master
            .try_clone()
            .ok()
            .map(|file| Box::new(file) as Box<dyn Read + Send>)
    }

    /// Takes the writable end of the terminal input stream.
    pub fn take_writer(&mut self) -> Option<Box<dyn Write + Send>> {
        self.master
            .try_clone()
            .ok()
            .map(|file| Box::new(file) as Box<dyn Write + Send>)
    }

    /// Handle for resizing the terminal from another thread.
    pub fn resizer(&self) -> PtyResizer {
        PtyResizer {
            master: self.master.as_raw_fd(),
        }
    }

    /// Forwards Ctrl+C into the pseudoterminal for as long as the guard lives.
    pub fn forward_interrupts(&self) -> Option<CtrlCForwarder> {
        CtrlCForwarder::install(&self.master).ok()
    }

    /// Returns the exit status when the child has already finished.
    ///
    /// Used by interactive bridges that must notice a child exiting while they
    /// are also waiting for input, without blocking either side.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError`] when polling fails.
    pub fn try_wait(&mut self) -> Result<Option<ProcessStatus>, ProcessError> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }

        let status = self.child.try_wait().map_err(|source| ProcessError::Wait {
            program: String::from("process"),
            source,
        })?;

        Ok(status.map(|status| {
            let status = ProcessStatus::from_exit_status(status);
            self.status = Some(status);
            status
        }))
    }

    /// Waits for the child to exit.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError`] when waiting fails.
    pub fn wait(&mut self) -> Result<ProcessStatus, ProcessError> {
        if let Some(status) = self.status {
            return Ok(status);
        }

        let status = self.child.wait().map_err(|source| ProcessError::Wait {
            program: String::from("process"),
            source,
        })?;

        let status = ProcessStatus::from_exit_status(status);
        self.status = Some(status);
        Ok(status)
    }

    /// Terminates the child. Callers still have to wait for it.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

impl Drop for PtyProcess {
    fn drop(&mut self) {
        // An orphaned interactive child must never outlive the session.
        if self.status.is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Builds the command with the session's directory and environment.
fn build_command(command: &ExternalCommand, spec: &LaunchSpec) -> Command {
    let mut process = Command::new(&spec.executable);
    process.args(&spec.arguments);

    if let Some(directory) = command.working_directory() {
        process.current_dir(directory);
    }

    if command.clears_environment() {
        process.env_clear();
    }

    if !command.environment_entries().is_empty() {
        process.envs(
            command
                .environment_entries()
                .iter()
                .map(|(key, value)| (key.as_os_str(), value.as_os_str())),
        );
    }

    process
}

/// Opens a pseudoterminal pair and applies `size` to it.
fn open_terminal(size: PtySize) -> io::Result<(File, File)> {
    // SAFETY: `posix_openpt` returns a new descriptor, adopted by `File`.
    let descriptor = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };

    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: the descriptor is freshly created and not owned elsewhere.
    let master = unsafe { File::from_raw_fd(descriptor) };

    // SAFETY: `master` is a valid pseudoterminal master.
    if unsafe { libc::grantpt(descriptor) } != 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: as above.
    if unsafe { libc::unlockpt(descriptor) } != 0 {
        return Err(io::Error::last_os_error());
    }

    resize_descriptor(descriptor, size)?;
    let name = slave_name(descriptor)?;
    let slave = File::options().read(true).write(true).open(&name)?;

    Ok((master, slave))
}

/// Applies a window size to a terminal descriptor.
fn resize_descriptor(descriptor: i32, size: PtySize) -> io::Result<()> {
    let window = libc::winsize {
        ws_row: size.rows(),
        ws_col: size.columns(),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // SAFETY: the descriptor is a terminal and `window` is a valid `winsize`.
    if unsafe { libc::ioctl(descriptor, libc::TIOCSWINSZ, &window) } != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

/// Path of the slave side of a pseudoterminal.
#[cfg(target_os = "linux")]
fn slave_name(descriptor: i32) -> io::Result<String> {
    let mut buffer = [0i8; 128];

    // SAFETY: the descriptor is a pseudoterminal master and `buffer` is writable
    // for its whole length.
    let result = unsafe { libc::ptsname_r(descriptor, buffer.as_mut_ptr(), buffer.len()) };

    if result != 0 {
        return Err(io::Error::from_raw_os_error(result));
    }

    // SAFETY: `ptsname_r` wrote a NUL-terminated path into `buffer`.
    Ok(unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned())
}

#[cfg(not(target_os = "linux"))]
fn slave_name(descriptor: i32) -> io::Result<String> {
    // SAFETY: `ptsname` returns a pointer to a static buffer owned by libc.
    let pointer = unsafe { libc::ptsname(descriptor) };

    if pointer.is_null() {
        return Err(io::Error::last_os_error());
    }

    let name = unsafe { std::ffi::CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();

    Ok(name)
}

/// Size of the shell's own terminal.
pub(super) fn terminal_size() -> Option<PtySize> {
    let mut window: libc::winsize = unsafe { std::mem::zeroed() };

    // SAFETY: `window` is a valid out-pointer for `TIOCGWINSZ`. The call fails
    // when stdout is not a terminal.
    let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut window) };

    if result != 0 || window.ws_col == 0 || window.ws_row == 0 {
        return None;
    }

    Some(PtySize::new(window.ws_row, window.ws_col))
}

/// Descriptor the Ctrl+C handler writes to.
static FORWARD_DESCRIPTOR: AtomicI32 = AtomicI32::new(-1);

/// Writes a Ctrl+C byte into the active pseudoterminal.
///
/// The tty line discipline on the master side turns `0x03` into `SIGINT` for the
/// foreground process group, which is the child.
extern "C" fn forward_sigint(_signal: libc::c_int) {
    let descriptor = FORWARD_DESCRIPTOR.load(Ordering::SeqCst);

    if descriptor < 0 {
        return;
    }

    let byte = 0x03u8;

    // SAFETY: `write` is async-signal-safe. The descriptor is a non-blocking
    // duplicate of the master, kept open for the guard's lifetime.
    unsafe {
        libc::write(descriptor, std::ptr::addr_of!(byte).cast(), 1);
    }
}

/// Forwards Ctrl+C into the pseudoterminal instead of killing the shell.
pub struct CtrlCForwarder {
    descriptor: i32,
    previous: libc::sighandler_t,
}

impl CtrlCForwarder {
    fn install(master: &File) -> io::Result<Self> {
        // A duplicate is used so the handler keeps working independently of the
        // reader and writer handles.
        // SAFETY: `fcntl` only inspects the descriptor and returns a new one.
        let descriptor = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };

        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }

        // A handler must never block; writes that cannot be accepted are
        // dropped instead.
        // SAFETY: as above.
        unsafe {
            let flags = libc::fcntl(descriptor, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        FORWARD_DESCRIPTOR.store(descriptor, Ordering::SeqCst);

        // SAFETY: `forward_sigint` has the `sighandler_t` signature and only
        // performs async-signal-safe work.
        let previous = unsafe {
            libc::signal(
                libc::SIGINT,
                forward_sigint as *const () as libc::sighandler_t,
            )
        };

        if previous == libc::SIG_ERR {
            let error = io::Error::last_os_error();
            FORWARD_DESCRIPTOR.store(-1, Ordering::SeqCst);
            // SAFETY: the descriptor was created above and is not owned elsewhere.
            unsafe {
                libc::close(descriptor);
            }

            return Err(error);
        }

        Ok(Self {
            descriptor,
            previous,
        })
    }
}

impl Drop for CtrlCForwarder {
    fn drop(&mut self) {
        // Stop new signals from reaching the descriptor before it is closed.
        FORWARD_DESCRIPTOR.store(-1, Ordering::SeqCst);

        // SAFETY: `previous` came from a successful `signal` call, so it is a
        // valid disposition to restore.
        unsafe {
            libc::signal(libc::SIGINT, self.previous);
            libc::close(self.descriptor);
        }
    }
}

/// Copies the user's keystrokes into the pseudoterminal until `stop` is set.
///
/// `poll` with a timeout lets the thread notice teardown instead of blocking
/// forever on the terminal, which would otherwise swallow the next command the
/// user types.
pub(super) fn relay_stdin(mut writer: Box<dyn Write + Send>, stop: &AtomicBool) {
    let descriptor = libc::STDIN_FILENO;
    let mut buffer = [0u8; 8192];

    while !stop.load(Ordering::Relaxed) {
        let mut polled = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN,
            revents: 0,
        };

        // SAFETY: `polled` describes one valid descriptor.
        let ready = unsafe { libc::poll(&mut polled, 1, 200) };

        if ready == 0 {
            continue;
        }

        if ready < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }

            break;
        }

        // SAFETY: `buffer` is writable for its whole length.
        let count = unsafe {
            libc::read(
                descriptor,
                buffer.as_mut_ptr().cast::<std::ffi::c_void>(),
                buffer.len(),
            )
        };

        if count <= 0 {
            break;
        }

        if writer.write_all(&buffer[..count as usize]).is_err() {
            break;
        }

        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_size_is_prepared_without_zero_columns() {
        let size = PtySize::new(0, 0);

        assert_eq!(size.rows(), 1);
        assert_eq!(size.columns(), 1);
    }
}
