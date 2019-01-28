// Copyright (c) 2019 Jason White
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use bincode;
use futures::Future;
use humantime;
use log;
use pretty_env_logger;
use serde::Deserialize;
use tokio::{self, runtime::current_thread::Runtime, timer::Timeout};

use super::client::Client;
use super::error::Error;
use super::Server;

/// Connects to the daemon or spawns if it isn't running and then connects to
/// it.
///
/// If the daemon already exists, returns the port for the existing daemon.

pub fn try_connect<P: AsRef<Path>>(
    root: P,
) -> Result<Option<(Client, u16)>, Error> {
    let root = root.as_ref();

    // Try connecting to the daemon.
    if let Ok(f) = fs::File::open(root.join(".button/port")) {
        let port = bincode::deserialize_from(f)?;

        if let Ok(client) = Client::new(port) {
            return Ok(Some((client, port)));
        }
    }

    Ok(None)
}

pub fn connect_or_spawn<F>(root: &Path, command: F) -> Result<Client, Error>
where
    F: FnOnce() -> Result<Command, Error>,
{
    // Try connecting to the server.
    if let Some((client, _)) = try_connect(root)? {
        return Ok(client);
    }

    // Spawn the server.
    let port = spawn(root, command()?)?;

    // TODO: Retry connections to the server?
    Client::new(port)
}

fn read_server_startup<S, T>(stream: S) -> impl Future<Item = T, Error = Error>
where
    S: tokio::io::AsyncRead,
    T: for<'de> Deserialize<'de>,
{
    tokio::io::read_exact(stream, [0u8; 8])
        .from_err::<Error>()
        .and_then(|(stream, buf)| {
            let len: u64 = bincode::deserialize(&buf).unwrap();

            let buf = vec![0; len as usize];

            tokio::io::read_exact(stream, buf)
                .from_err::<Error>()
                .and_then(|(_, buf)| Ok(bincode::deserialize(&buf)?))
        })
}

/// Spawns the server by creating a new process. We wait for the daemon process
/// to start up by creating either a domain socket on Unix or by creating
/// a named pipe on Windows.
///
/// Returns the port over which we can connect to the server.
///
/// If this fails, the caller is responsible for retrying.
#[cfg(unix)]
fn spawn(root: &Path, mut command: Command) -> Result<u16, Error> {
    use tempfile::TempDir;
    use tokio_uds::UnixListener;
    use futures::Stream;

    let mut runtime = Runtime::new()?;

    // Create a temporary domain socket for the spawned process to notify us
    // when it is fully started up.
    //
    // Note that the socket file must not exist before binding. We'll get
    // an "Address already in use" error otherwise. When the temporary directory
    // falls out of scope, the temporary socket file will get cleaned up
    // automatically.
    let tempdir = TempDir::new()?;
    let socket_path = tempdir.path().join("socket");

    let listener = UnixListener::bind(&socket_path)?;

    // Spawn the daemon process.
    let _child = command
        .env("BUTTON_STARTUP_NOTIFY", &socket_path)
        .current_dir(root)
        .spawn()?;

    // Wait for the server to send back a message telling us its port number.
    let startup = listener
        .incoming()
        .into_future()
        .map_err(|(err, _)| Error::from(err))
        .and_then(|(socket, _)| read_server_startup(socket.unwrap()));

    // Don't wait forever for the server to start up.
    let task = Timeout::new(startup, Duration::from_secs(10)).map_err(|err| {
        if err.is_elapsed() {
            Error::TimedOut
        } else {
            err.into_inner().unwrap()
        }
    });

    let message: Result<u16, String> = runtime.block_on(task)?;

    let port = message.map_err(|e| Error::Other(e.into()))?;

    Ok(port)
}

#[cfg(windows)]
fn spawn(root: &Path, mut command: Command) -> Result<u16, Error> {
    use std::process::Stdio;
    use uuid::Uuid;
    use tokio_named_pipes::NamedPipe;
    use tokio::reactor::Handle;
    use futures::future;
    use std::os::windows::process::CommandExt;
    use winapi::um::winbase::{DETACHED_PROCESS, CREATE_NEW_PROCESS_GROUP};

    let mut runtime = Runtime::new()?;

    let pipe_name = format!(r"\\.\pipe\{}", Uuid::new_v4().to_simple());

    let pipe = runtime.block_on(future::lazy(|| {
        // `Handle::default()` does not currently work with this. Not sure why
        // yet. This is wrapped inside a lazy future so that this is executed
        // within the context of our tokio Runtime.
        #[allow(deprecated)]
        let handle = &Handle::current();

        NamedPipe::new(&pipe_name, handle)
    }))?;

    // Asynchronously enables the child process we're about to spawn to
    // connect to the named pipe. Since this is async, this will return
    // `WouldBlock`.
    if let Err(err) = pipe.connect() {
        if err.kind() != io::ErrorKind::WouldBlock {
            return Err(err.into());
        }
    }

    // Spawn the daemon process.
    //
    // FIXME: The daemon process should not inherit handles from this process.
    // Unfortunately, there is currently no way to disable handle inheritance
    // with `std::os::windows::process::CommandExt` on Windows. See
    // https://github.com/rust-lang/rust/issues/38227 for more info.
    let _child = command
        .env("BUTTON_STARTUP_NOTIFY", &pipe_name)
        .current_dir(root)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .stdin(Stdio::null())
        .stdout(fs::File::create(root.join(".button/stdout"))?)
        .stderr(fs::File::create(root.join(".button/stderr"))?)
        .spawn()?;

    let startup = read_server_startup(pipe);
    let task = Timeout::new(startup, Duration::from_secs(10)).map_err(|err| {
        if err.is_elapsed() {
            Error::TimedOut
        } else {
            err.into_inner().unwrap()
        }
    });

    let message: Result<u16, String> = runtime.block_on(task)?;

    let port = message.map_err(|e| Error::Other(e.into()))?;

    Ok(port)
}

/// "Daemonizes" the process.
///
/// This does a couple of important things:
///  - Detaches the process from its parent process.
///  - Redirects stdout/stderr to a file such that logs can be viewed later.
///
/// This should only be called once we're ready to turn the current process into
/// a daemon.
#[cfg(unix)]
pub fn daemonize() -> Result<(), io::Error> {
    use daemonize::Daemonize;

    let stdout = fs::File::create(".button/stdout")?;
    let stderr = fs::File::create(".button/stderr")?;

    Daemonize::new()
        .pid_file(".button/pid")
        .working_directory(".")
        .stdout(stdout)
        .stderr(stderr)
        .start()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // At this point, the process has forked twice in order to detach itself
    // from the parent process and is now running as a daemon. After this, the
    // server can be started and begin listening for incoming connections.

    Ok(())
}

#[cfg(windows)]
pub fn daemonize() -> Result<(), io::Error> {
    // Nothing to do on Windows. When the daemon process is spawned on Windows,
    // it is already detached.
    Ok(())
}

/// Runs the server in the foreground. Daemonizing the process should be done
/// before this. It is assumed that the server is running from the current
/// working directory and that the `.button` directory already exists.
///
/// This will set up logging and create the server.
pub fn run(
    port: u16,
    idle: Option<Duration>,
    log_level: Option<log::LevelFilter>,
) -> Result<(), Error> {
    // Default of one hour.
    let idle = idle.unwrap_or(Duration::from_secs(60 * 60));

    let log_level = log_level.unwrap_or_else(|| {
        env::var("BUTTON_LOG_LEVEL")
            .ok()
            .and_then(|v| v.parse::<log::LevelFilter>().ok())
            .unwrap_or(log::LevelFilter::Info)
    });

    // Initialize logging.
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.filter_module("button", log_level);
    builder.init();

    let server = Server::new(port)?;

    // Create the `port` file. Note that the current process must lock this file
    // to prevent another daemon from spawning at the same time. The file handle
    // must be left open as long as the server is running (that is, until the
    // end of this function scope).
    let mut f = create_locked(".button/port")?;
    bincode::serialize_into(&mut f, &server.port())?;

    log::info!(
        "Listening on {}. Will shutdown if idle for {}.",
        server.addr(),
        humantime::format_duration(idle)
    );

    server.run(idle);

    Ok(())
}

/// Creates a file and exclusively locks it. This ensures that another process
/// cannot open the same file for writing (but can open for reading).
#[cfg(unix)]
fn create_locked<P: AsRef<Path>>(path: P) -> Result<fs::File, io::Error> {
    use nix::fcntl::{flock, FlockArg};
    use std::os::unix::io::AsRawFd;

    let path = path.as_ref();

    // Open the file without truncating it. We don't want to clobber the
    // contents before acquiring the lock.
    let f = fs::OpenOptions::new().write(true).create(true).open(path)?;

    // If we can't lock the file, then there must be another daemon process that
    // is holding it.
    flock(f.as_raw_fd(), FlockArg::LockExclusiveNonblock).map_err(|_| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("The file '{}' is locked", path.display()),
        )
    })?;

    // Okay to clobber the contents now that we've acquired the lock.
    f.set_len(0)?;

    Ok(f)
}

#[cfg(windows)]
fn create_locked<P: AsRef<Path>>(path: P) -> Result<fs::File, io::Error> {
    use std::os::windows::fs::OpenOptionsExt;
    use winapi::um::winnt::FILE_SHARE_READ;

    // Don't allow other processes to read this file.
    fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
}

/// Runs the server as a daemon process.
///
/// This is meant to be called to initialize the daemon process after it has
/// already been spawned. It is assumed that the server is running from the
/// current working directory.
pub fn run_daemon(
    port: u16,
    idle: Option<Duration>,
    log_level: Option<log::LevelFilter>,
) -> Result<(), Error> {
    // Remove the variable so it doesn't get inherited by child
    // processes (incase `button` is run as part of the build).
    env::remove_var("BUTTON_SERVER");

    fs::create_dir_all(".button")?;

    daemonize()?;

    run(port, idle, log_level)?;

    Ok(())
}
