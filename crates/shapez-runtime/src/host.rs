use crate::protocol::{Request, Response};
use anyhow::{Context, Result, bail, ensure};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::RecvTimeoutError,
    },
    thread,
    time::{Duration, Instant},
};

static PIPE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct LabPaths {
    pub root: PathBuf,
    pub repo: PathBuf,
    pub game: PathBuf,
    pub dotnet: PathBuf,
}
impl LabPaths {
    pub fn discover() -> Self {
        let root = std::env::temp_dir().join("shapez-lab");
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let dotnet = std::env::var_os("SHAPEZ_LAB_DOTNET")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("tools/dotnet/dotnet.exe"));
        let game = std::env::var_os("SHAPEZ_GAME_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/shapez 2")
            });
        Self {
            root,
            repo,
            game,
            dotnet,
        }
    }
    pub fn host(&self) -> PathBuf {
        self.root.join("host")
    }
    pub fn build(&self, project: &str) -> Result<PathBuf> {
        self.build_project(
            &self
                .repo
                .join(format!("interop/{project}/{project}.csproj")),
        )
    }
    pub fn build_project(&self, path: &Path) -> Result<PathBuf> {
        let project = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Project filename required")?;
        ensure!(
            self.dotnet.is_file(),
            ".NET SDK missing at {}. Set SHAPEZ_LAB_DOTNET to an SDK dotnet.exe",
            self.dotnet.display()
        );
        let status = Command::new(&self.dotnet)
            .arg("build")
            .arg(path)
            .args(["-c", "Release", "--nologo"])
            .arg(format!("-p:GameRoot={}", self.game.display()))
            .status()?;
        ensure!(status.success(), "{project} build failed");
        Ok(self.root.join(format!(
            "bin/{project}/Release/{}",
            if project == "HostPrep" {
                "net10.0"
            } else {
                "netstandard2.1"
            }
        )))
    }
}
pub fn prepare_host(p: &LabPaths) -> Result<()> {
    let prep = p.build("HostPrep")?;
    p.build("LabMod")?;
    let host = p.host();
    fs::create_dir_all(&host)?;
    let _lease = acquire_lease(&host.join("running.lock"))?;
    stage(&p.game, &host)?;
    fs::write(
        host.join(".shapez-lab-staged"),
        b"Local test clone; do not distribute",
    )?;
    let status = Command::new(&p.dotnet)
        .arg(prep.join("HostPrep.dll"))
        .arg(&host)
        .status()?;
    ensure!(status.success(), "Host patch failed");
    Ok(())
}
fn stage(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            stage(&e.path(), &dest)?
        } else if e.path().extension().is_some_and(|x| x == "dll")
            && e.path().components().any(|x| x.as_os_str() == "Managed")
        {
            fs::copy(e.path(), dest)?;
        } else if !dest.exists() {
            fs::hard_link(e.path(), &dest).or_else(|_| fs::copy(e.path(), &dest).map(|_| ()))?;
        }
    }
    Ok(())
}
pub struct Host {
    child: Child,
    pub run: PathBuf,
    lock: PathBuf,
    lease: Option<fs::File>,
    seq: u64,
    connection: Option<crate::transport::Connection>,
    #[cfg(windows)]
    _job: Job,
}
impl Host {
    pub fn start(p: &LabPaths, timeout: Duration) -> Result<Self> {
        let run = crate::artifacts::RunDirectory::create(&p.root, "host")?;
        Self::start_in(p, &run.path, timeout)
    }
    pub fn start_in(p: &LabPaths, run: &Path, timeout: Duration) -> Result<Self> {
        let host = p.host();
        ensure!(host.join("patch-audit.json").exists(), "Run prepare first");
        let lock = host.join("running.lock");
        let mut lock_file = acquire_lease(&lock)?;
        let result = (|| {
            let run = run.to_owned();
            let data = run.join("data");
            let mods = data.join("mods/ShapezLab");
            fs::create_dir_all(&mods)?;
            let built = p.root.join("bin/LabMod/Release/netstandard2.1");
            for f in ["ShapezLab.dll", "manifest.json"] {
                fs::copy(built.join(f), mods.join(f))?;
            }
            let mut cmd = Command::new(host.join("shapez 2.exe"));
            let pipe_name = format!(
                "shapez-lab-{}-{}",
                std::process::id(),
                PIPE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            );
            cmd.current_dir(&host)
                .env("SHAPEZ_LAB_DATA", &data)
                .env("SHAPEZ_LAB_RUN", &run)
                .env("SHAPEZ_LAB_PIPE", &pipe_name)
                .args([
                    "--disable-store-sdk",
                    "--qa",
                    "-batchmode",
                    "-noaudio",
                    "-screen-fullscreen",
                    "0",
                    "-screen-width",
                    "640",
                    "-screen-height",
                    "360",
                    "-logFile",
                ])
                .arg(run.join("player.log"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let mut child = cmd.spawn()?;
            #[cfg(windows)]
            let job = match Job::attach(&child) {
                Ok(job) => job,
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(e);
                }
            };
            lock_file.set_len(0)?;
            write!(&mut lock_file, "{}", child.id())?;
            let mut h = Self {
                child,
                run,
                lock: lock.clone(),
                lease: Some(lock_file),
                seq: 0,
                connection: None,
                #[cfg(windows)]
                _job: job,
            };
            let deadline = Instant::now() + timeout;
            let pipe_path = format!(r"\\.\pipe\{pipe_name}");
            let pipe = loop {
                match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&pipe_path)
                {
                    Ok(pipe) => break pipe,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::NotFound
                            || error.raw_os_error() == Some(231) => {}
                    Err(error) => return Err(error).context("Connecting to host pipe"),
                }
                h.check_alive()?;
                ensure!(
                    Instant::now() < deadline,
                    "Timeout connecting to host pipe; log: {}",
                    h.run.join("player.log").display()
                );
                thread::sleep(Duration::from_millis(10));
            };
            h.connection = Some(crate::transport::Connection::new(pipe));
            let ready = h.receive(deadline.saturating_duration_since(Instant::now()))?;
            ensure!(
                ready["kind"] == "ready" && ready["protocol"] == 1,
                "Unsupported host handshake: {ready}"
            );
            ensure!(
                ready["pid"].as_u64() == Some(h.child.id() as u64),
                "Readiness PID mismatch"
            );
            ensure!(
                Path::new(ready["data_path"].as_str().unwrap_or("")) == data,
                "Data isolation mismatch: {ready}"
            );
            // An archive of the handshake, never used for synchronization.
            fs::write(h.run.join("ready.json"), serde_json::to_vec_pretty(&ready)?)?;
            Ok(h)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&lock);
        }
        result
    }
    fn check_alive(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            bail!(
                "Game exited {status}; log: {}",
                self.run.join("player.log").display()
            );
        }
        Ok(())
    }
    fn receive(&mut self, timeout: Duration) -> Result<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let incoming = &self
                .connection
                .as_ref()
                .context("Host pipe closed")?
                .incoming;
            match incoming.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(Ok(value)) => {
                    ensure!(value["kind"] != "fatal", "Host fatal: {value}");
                    return Ok(value);
                }
                Ok(Err(error)) => bail!(
                    "Host pipe failed: {error}; log: {}",
                    self.run.join("player.log").display()
                ),
                Err(RecvTimeoutError::Disconnected) => bail!(
                    "Host pipe disconnected; log: {}",
                    self.run.join("player.log").display()
                ),
                Err(RecvTimeoutError::Timeout) => {
                    self.check_alive()?;
                    if Instant::now() >= deadline {
                        // A partially sent/requested operation cannot be reused safely.
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        bail!(
                            "Timeout waiting for host pipe response; log: {}",
                            self.run.join("player.log").display()
                        );
                    }
                }
            }
        }
    }
    pub fn request<T: Serialize, R: DeserializeOwned>(
        &mut self,
        operation: &str,
        args: T,
        timeout: Duration,
    ) -> Result<Response<R>> {
        self.seq += 1;
        let id = self.seq;
        let message = Request {
            id,
            operation,
            args,
        };
        self.connection
            .as_ref()
            .context("Host pipe closed")?
            .send(serde_json::to_value(message)?)?;
        let result = self.receive(timeout)?;
        let response: Response<R> = serde_json::from_value(result)?;
        ensure!(response.id == id, "Response ID mismatch");
        Ok(response)
    }
    pub fn shutdown(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_some() {
            return Ok(());
        }
        self.request::<_, Value>("shutdown", serde_json::json!({}), Duration::from_secs(10))?
            .into_result()?;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if self.child.try_wait()?.is_some() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.connection.take();
        self.lease.take();
        let _ = fs::remove_file(&self.lock);
    }
}

fn acquire_lease(path: &Path) -> Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let file = options
        .open(path)
        .context("A lab instance owns the staging directory")?;
    file.try_lock()
        .context("A lab instance owns the staging directory")?;
    Ok(file)
}

#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Job {
    fn attach(child: &Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            ensure!(!h.is_null(), "CreateJobObject failed");
            let job = Self(h);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            ensure!(
                SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as _,
                    std::mem::size_of_val(&info) as u32
                ) != 0,
                "SetInformationJobObject failed"
            );
            ensure!(
                AssignProcessToJobObject(h, child.as_raw_handle()) != 0,
                "AssignProcessToJobObject failed"
            );
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lease_excludes_concurrent_owners_and_recovers_stale_file() {
        let root = std::env::temp_dir().join("shapez-lab");
        fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("lease-test-{}.lock", std::process::id()));
        let first = acquire_lease(&path).unwrap();
        assert!(acquire_lease(&path).is_err());
        drop(first);
        assert!(path.exists());
        drop(acquire_lease(&path).unwrap());
        fs::remove_file(path).unwrap();
    }
}
