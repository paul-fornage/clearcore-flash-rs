use crate::serial::{get_or_touch_to_bootloader, log_minor_err, touch_to_bootloader, wait_for_serial_port};
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use anyhow::{anyhow, Context, Result};
use iced::futures::{SinkExt, Stream};
use iced::{stream, Subscription};
use iced::futures::channel::mpsc;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialPortInfo, SerialPortType};
use cxx::{let_cxx_string, type_id, CxxString, ExternType};
use crate::types::{LogMsg, LogMsgType, SerialConfig, UsbId};
use crate::serial::{find_port_async, TEKNIC_BOOTLOADER_OFFSET_ADDRESS};
use std::str;

use bossa;

use tokio::time::timeout;
use crate::{log_send, log_send_blocking};

// --- Global State for FFI Callbacks ---
// These allow the C++ callbacks (which can't capture environment) to talk to our Rust async world.
static PROGRESS_STATE: Mutex<ProgressState> = Mutex::new(ProgressState::new());
static LOG_SENDER: Mutex<Option<tokio::sync::mpsc::UnboundedSender<DownloadEvent>>> = Mutex::new(None);

pub fn get_temp_download_path() -> PathBuf {
    let mut temp = std::env::temp_dir();
    temp.push("clearcore-flasher");
    // Ensure dir exists
    let _ = std::fs::create_dir_all(&temp);
    temp.push("temp-download.bin");
    temp
}


#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Log(LogMsg),
    Error(String),
    ProgressBarUpdate(DownloadProgressBar),
    Success,
}

impl DownloadEvent {
    pub fn log(log_type: LogMsgType, message: impl Into<String>) -> Self {
        Self::Log(LogMsg{message: message.into(), log_type})
    }
}
impl From<LogMsg> for DownloadEvent { fn from(msg: LogMsg) -> Self { Self::Log(msg) } }

#[derive(Debug, Clone, PartialEq, Hash, Default)]
pub struct DownloadConfig {
    // Kept for future extensibility (e.g. specifying memory regions)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadPhase {
    #[default]
    Initializing,
    Reading,
    Resetting,
}

impl Display for DownloadPhase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadPhase::Initializing => write!(f, "Initializing"),
            DownloadPhase::Reading => write!(f, "Reading Firmware"),
            DownloadPhase::Resetting => write!(f, "Resetting Device"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgressBar {
    pub phase: DownloadPhase,
    pub current: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Copy)]
struct ProgressState {
    phase: DownloadPhase,
    current: u32,
    total: u32,
}

impl ProgressState {
    const fn new() -> Self {
        Self {
            phase: DownloadPhase::Initializing,
            current: 0,
            total: 100,
        }
    }
}




pub fn listen() -> Subscription<DownloadEvent> {
    Subscription::run(|| subscription())
}

pub fn subscription() -> impl Stream<Item = DownloadEvent> {
    stream::channel(100, move |mut output| async move {
        // We use a config here in case you want to add params later, currently default
        let config = DownloadConfig::default();
        let res = execute_download_sequence(&mut output, &config).await;
        match res{
            Ok(_) => {
                log::debug!("download sequence completed successfully");
                log_minor_err(output.send(DownloadEvent::Success).await);
            }
            Err(e) => {
                log::error!("download sequence failed: {:?}", e);
                log_minor_err(output.send(DownloadEvent::Error(e.to_string())).await);
            }
        }
    })
}

async fn execute_download_sequence(output: &mut mpsc::Sender<DownloadEvent>, _config: &DownloadConfig) -> Result<()> {
    log_send!(output, Info, "Starting firmware download sequence...");

    // 1. Prepare Paths
    let temp_path = get_temp_download_path();
    let save_path_str = temp_path.to_str()
        .context("Failed to convert temp path to string")?
        .to_string();

    // 2. Put Device in Bootloader Mode
    let bootloader_port_info = get_or_touch_to_bootloader(output).await?;

    log_send!(output, Info, "Bootloader Port: {}", bootloader_port_info.port_name);

    // 3. Execute Download (Read) via Bossa
    match download_firmware(output, &bootloader_port_info.port_name, &save_path_str).await {
        Ok(_) => log_send!(output, Info, "Firmware downloaded successfully"),
        Err(e) => anyhow::bail!("Failed to download firmware: {}", e),
    }

    // 4. Wait for Reboot
    let _serial_port_info = timeout(
        Duration::from_secs(10), wait_for_serial_port(UsbId::CLEARCORE_SERIAL)
    ).await??;

    log_send!(output, Info, "ClearCore found after reboot.");

    Ok(())
}

async fn download_firmware(output: &mut mpsc::Sender<DownloadEvent>, port_name: &str, save_path: &str) -> Result<()> {
    // Channel for the blocking thread to send logs back to the async world
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let port_name = port_name.to_string();
    let save_path = save_path.to_string();

    {
        // Reset progress state
        let mut state = PROGRESS_STATE.lock().unwrap();
        *state = ProgressState::new();
    }

    // Spawn the heavy lifting on a thread where blocking is okay
    let task = tokio::task::spawn_blocking(move || {
        download_firmware_blocking(tx, &port_name, &save_path)
    });

    // Ticker loop: polls the mutex for progress, and the channel for logs
    let mut ticker = tokio::time::interval(Duration::from_millis(16));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                 // Check progress lock
                 if let Ok(state) = PROGRESS_STATE.lock() {
                     // We intentionally drop the lock immediately after cloning data
                     // This prevents "MutexGuard across await" error
                     let prog_bar = DownloadProgressBar {
                         phase: state.phase,
                         current: state.current,
                         total: state.total
                     };
                     drop(state);

                     let _ = output.try_send(DownloadEvent::ProgressBarUpdate(prog_bar));
                 }
            }
            msg = rx.recv() => {
                match msg {
                    Some(event) => {
                        log_minor_err(output.send(event).await);
                    }
                    None => {
                        // Channel closed, worker thread finished
                        break;
                    }
                }
            }
        }
    }

    task.await.context("Firmware download task panicked")??;

    Ok(())
}

// RAII Guard to ensure the global log sender is cleared if the thread panics or finishes
struct GlobalSenderGuard;

impl GlobalSenderGuard {
    fn new(sender: tokio::sync::mpsc::UnboundedSender<DownloadEvent>) -> Self {
        if let Ok(mut guard) = LOG_SENDER.lock() {
            *guard = Some(sender);
        }
        Self
    }
}

impl Drop for GlobalSenderGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = LOG_SENDER.lock() {
            *guard = None;
        }
    }
}

fn set_phase(phase: DownloadPhase) {
    if let Ok(mut state) = PROGRESS_STATE.lock() {
        state.phase = phase;
        state.current = 0;
        state.total = 100;
    }
}

// The actual Bossa Logic (Synchronous C++)
fn download_firmware_blocking(output_sender: tokio::sync::mpsc::UnboundedSender<DownloadEvent>, port_name: &str, save_path: &str) -> Result<()> {
    let _guard = GlobalSenderGuard::new(output_sender.clone());

    let mut port_factory = bossa::lib::new_port_factory();
    if port_factory.is_null() { anyhow::bail!("Failed to create Port Factory"); }
    let_cxx_string!(port_name_cxx = port_name);
    let serial_port = port_factory.pin_mut().create_port(&port_name_cxx, true);
    if serial_port.is_null() { anyhow::bail!("Failed to create serial port: {port_name}"); }
    log_send_blocking!(output_sender, Debug, "Serial port created successfully");

    let mut samba = bossa::lib::new_samba();
    if samba.is_null() { anyhow::bail!("Failed to create Samba"); }

    if !samba.pin_mut().connect(serial_port, 115200) {
        anyhow::bail!("Failed to connect to device at {port_name} via Samba at 115200 baud.");
    }
    log_send_blocking!(output_sender, Info, "Connected to device via Samba");

    let mut device = bossa::lib::new_device(samba.pin_mut());
    if device.is_null() { anyhow::bail!("Failed to create Device"); }

    device.pin_mut().create();
    log_send_blocking!(output_sender, Debug, "Device created successfully");

    // Connect C++ callbacks to our extern "C" functions below
    let log_observer = bossa::ObserverCallback(log_observer_fn);
    let prog_observer_callback = bossa::ProgressCallback(prog_observer_fn);
    let mut observer = unsafe { bossa::lib::new_bossa_observer_with_progress(log_observer, prog_observer_callback) };
    if observer.is_null() { anyhow::bail!("Failed to create Bossa Observer"); }

    let mut flasher = bossa::lib::new_flasher(
        samba.pin_mut(),
        device.pin_mut(),
        observer.pin_mut()
    );

    // Get Device Info to determine read size
    let mut flasher_info = bossa::lib::new_flasher_info();
    if flasher_info.is_null() { anyhow::bail!("Failed to create Flasher Info"); }
    flasher.pin_mut().info(flasher_info.pin_mut());

    let mut info = bossa::lib::FlasherInfoRs::default();
    bossa::lib::flasherinfo2flasherinfors(&flasher_info.pin_mut(), &mut info);
    log_send_blocking!(output_sender, Info, "Found SAMBA device: {}", info.info());
    log_send_blocking!(output_sender, Info, "Detected Flash Size: {} bytes", info.totalSize);

    // Reading Phase
    log_send_blocking!(output_sender, Info, "Reading firmware to {}", save_path);
    set_phase(DownloadPhase::Reading);
    let_cxx_string!(save_path_cxx = save_path);

    // We assume reading the application area (Total - Bootloader)
    // Adjust logic if you need the full flash including bootloader
    let read_size = info.totalSize - TEKNIC_BOOTLOADER_OFFSET_ADDRESS;

    unsafe {
        // Mapping to Bossa's read(filename, size, offset)
        flasher.pin_mut().read(
            save_path_cxx.as_c_str().as_ptr(),
            read_size,
            TEKNIC_BOOTLOADER_OFFSET_ADDRESS
        );
    }
    log_send_blocking!(output_sender, Info, "Firmware read complete");

    // Reset Phase
    log_send_blocking!(output_sender, Info, "Resetting device...");
    set_phase(DownloadPhase::Resetting);
    flasher.pin_mut().reset();
    log_send_blocking!(output_sender, Info, "Reset complete");
    Ok(())
}

// --- C++ Callbacks ---

extern "C" fn log_observer_fn(message: &CxxString) {
    if let Ok(str_slice) = message.to_str() {
        let msg = str_slice.trim().to_string();
        if let Ok(guard) = LOG_SENDER.lock() {
            if let Some(sender) = guard.as_ref() {
                let _ = sender.send(DownloadEvent::log(LogMsgType::BossaNative, msg));
            }
        }
    }
}

extern "C" fn prog_observer_fn(current: i32, total: i32) {
    if let Ok(mut state) = PROGRESS_STATE.lock() {
        state.current = current as u32;
        state.total = total as u32;
    }
}

