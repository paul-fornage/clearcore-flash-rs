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
use crate::types::{SerialConfig, UsbId};
use crate::serial::find_port_async;
use std::str;

use bossa;

use tokio::time::timeout;

static PROGRESS_STATE: Mutex<ProgressState> = Mutex::new(ProgressState::new());
static LOG_SENDER: Mutex<Option<tokio::sync::mpsc::UnboundedSender<UploadEvent>>> = Mutex::new(None);

macro_rules! log_send {
    ($output:expr, $level:ident, $($arg:tt)+) => {{
        let msg = format!($($arg)+);

        match LogMsgType::$level {
            LogMsgType::Error => log::error!("{}", msg),
            LogMsgType::Warn => log::warn!("{}", msg),
            LogMsgType::Info => log::info!("{}", msg),
            LogMsgType::Debug => log::debug!("{}", msg),
            LogMsgType::Trace => log::trace!("{}", msg),
            LogMsgType::BossaNative => eprint!("{}", msg),
        }

        log_minor_err($output.send(UploadEvent::log(LogMsgType::$level, msg)).await);
    }};
}

macro_rules! log_send_blocking {
    ($output:expr, $level:ident, $($arg:tt)+) => {{
        let msg = format!($($arg)+);

        match LogMsgType::$level {
            LogMsgType::Error => log::error!("{}", msg),
            LogMsgType::Warn => log::warn!("{}", msg),
            LogMsgType::Info => log::info!("{}", msg),
            LogMsgType::Debug => log::debug!("{}", msg),
            LogMsgType::Trace => log::trace!("{}", msg),
            LogMsgType::BossaNative => eprint!("{}", msg),
        }

        log_minor_err($output.send(UploadEvent::log(LogMsgType::$level, msg)));
    }};
}

#[derive(Debug, Clone)]
pub enum UploadEvent {
    Log(LogMsg),
    Error(String),
    ProgressBarUpdate(ProgressBar),
    Success,
}

impl UploadEvent {
    pub fn log(log_type: LogMsgType, message: impl Into<String>) -> Self {
        Self::Log(LogMsg{message: message.into(), log_type})
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct UploadConfig {
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UploadPhase {
    #[default]
    Initializing,
    Erasing,
    Writing,
    Verifying,
    Resetting,
}

impl Display for UploadPhase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadPhase::Initializing => write!(f, "Initializing"),
            UploadPhase::Erasing => write!(f, "Erasing Flash"),
            UploadPhase::Writing => write!(f, "Writing Firmware"),
            UploadPhase::Verifying => write!(f, "Verifying Firmware"),
            UploadPhase::Resetting => write!(f, "Resetting Device"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressBar{
    pub phase: UploadPhase,
    pub current: u32,
    pub total: u32,
}

impl ProgressBar{
    pub fn loading_bar_string(&self) -> String{
        const PROGRESS_BAR_WIDTH: usize = 32;
        let mut progress = self.current as f64 / self.total as f64;
        if progress < 0.0 { progress = 0.0; }
        if progress > 1.0 { progress = 1.0; }
        let percent = progress * 100.0;
        let progress_bar_len = (progress * PROGRESS_BAR_WIDTH as f64).round() as usize;

        let mut progress_bar: [u8; PROGRESS_BAR_WIDTH] = [b' '; PROGRESS_BAR_WIDTH];
        progress_bar[..progress_bar_len].fill(b'=');
        let progress_str = str::from_utf8(&progress_bar).unwrap();

        format!("[{progress_str}] {percent:>6.2}% ({}/{})", self.current, self.total)
    }
    
    
}

#[derive(Debug, Clone, Copy)]
struct ProgressState {
    phase: UploadPhase,
    current: u32,
    total: u32,
}

impl ProgressState {
    const fn new() -> Self {
        Self {
            phase: UploadPhase::Initializing,
            current: 0,
            total: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogMsg {
    pub message: String,
    pub log_type: LogMsgType,
}

impl Display for LogMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if matches!(self.log_type, LogMsgType::BossaNative) {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{:?}: {}", self.log_type, self.message)
        }
    }
}

impl Into<String> for LogMsg {
    fn into(self) -> String {
        format!("{}", self)
    }
}

#[derive(Debug, Clone)]
pub enum LogMsgType{
    BossaNative,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub const TEKNIC_BOOTLOADER_OFFSET_ADDRESS: u32 = 0x4000;

pub fn listen(config: UploadConfig) -> Subscription<UploadEvent> {
    Subscription::run_with(config.clone(), {
        move |config| subscription(config.clone())
    })
}

pub fn subscription(config: UploadConfig) -> impl Stream<Item =UploadEvent> {
    stream::channel(100, move |mut output| async move {
        let res = execute_upload_sequence(&mut output, &config).await;
        match res{
            Ok(_) => {
                log::debug!("upload sequence completed successfully");
                log_minor_err(output.send(UploadEvent::Success).await);
            }
            Err(e) => {
                log::error!("upload sequence failed: {:?}", e);
                log_minor_err(output.send(UploadEvent::Error(e.to_string())).await);
            }
        }
    })
}

async fn execute_upload_sequence(output: &mut mpsc::Sender<UploadEvent>, config: &UploadConfig) -> Result<()> {
    log_send!(output, Info, "Starting firmware upload sequence...");
    if !config.file_path.exists() {
        anyhow::bail!("Firmware file not found: {:?}", config.file_path);
    }

    let bootloader_port_info = if let Ok(cc_serial_port_info) = find_serial_port(UsbId::CLEARCORE_SERIAL).await{
        log_send!(output, Info, "cc serial port found");
        let cc_bootloader_port_info = touch_to_bootloader(output, &cc_serial_port_info).await
            .context("Failed to touch cc serial port to bootloader")?;
        log_send!(output, Info, "cc touched to bootloader, waiting 1 second");
        tokio::time::sleep(Duration::from_secs(1)).await;
        cc_bootloader_port_info
    } else {
        if let Ok(cc_bootloader_port_info) = find_serial_port(UsbId::CLEARCORE_BOOTLOADER).await {
            log_send!(output, Warn, "Clearcore already in bootloader mode, skipping touching to bootloader");
            cc_bootloader_port_info
        } else {
            log_send!(output, Error, "Failed to find cc serial port");
            anyhow::bail!("Failed to find cc serial port");
        }
    };


    let firmware_path = std::path::absolute(config.file_path.clone())
        .context("Failed to get absolute firmware path. Likely the path is invalid or file does not exist.")?;
    let firmware_path_str = firmware_path.to_str()
        .context("Failed to convert firmware path to string. Likely the path contains invalid characters.")?;

    log_send!(output, Info, "Firmware path: {}", firmware_path_str);

    match upload_firmware(output, &bootloader_port_info.port_name, firmware_path_str).await {
        Ok(_) => log_send!(output, Info, "Firmware uploaded successfully"),
        Err(e) => anyhow::bail!("Failed to upload firmware: {}", e),
    }

    let _serial_port_info = timeout(
        Duration::from_secs(10), wait_for_serial_port(UsbId::CLEARCORE_SERIAL)
    ).await??;

    log_send!(output, Info, "cc serial port found after uploading firmware. Reset success");

    Ok(())
}

async fn upload_firmware(output: &mut mpsc::Sender<UploadEvent>, port_name: &str, firmware_path: &str) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let port_name = port_name.to_string();
    let firmware_path = firmware_path.to_string();

    {
        let mut state = PROGRESS_STATE.lock().unwrap();
        *state = ProgressState::new();
    }

    let task = tokio::task::spawn_blocking(move || {
        upload_firmware_blocking(tx, &port_name, &firmware_path)
    });

    let mut ticker = tokio::time::interval(Duration::from_millis(16));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                 if let Ok(state) = PROGRESS_STATE.lock() {
                     let _ = output.try_send(UploadEvent::ProgressBarUpdate(ProgressBar {
                         phase: state.phase,
                         current: state.current,
                         total: state.total
                     }));
                 }
            }
            msg = rx.recv() => {
                match msg {
                    Some(event) => {
                        log_minor_err(output.send(event).await);
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }

    task.await.context("Firmware upload task panicked")??;

    Ok(())
}

struct GlobalSenderGuard;

impl GlobalSenderGuard {
    fn new(sender: tokio::sync::mpsc::UnboundedSender<UploadEvent>) -> Self {
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

fn set_phase(phase: UploadPhase) {
    if let Ok(mut state) = PROGRESS_STATE.lock() {
        state.phase = phase;
        state.current = 0;
        state.total = 100;
    }
}

fn upload_firmware_blocking(output_sender: tokio::sync::mpsc::UnboundedSender<UploadEvent>, port_name: &str, firmware_path: &str) -> Result<()> {
    let _guard = GlobalSenderGuard::new(output_sender.clone());

    let mut port_factory = bossa::lib::new_port_factory();
    let_cxx_string!(port_name_cxx = port_name);
    let serial_port = port_factory.pin_mut().create_port(&port_name_cxx, true);
    if serial_port.is_null() {
        anyhow::bail!("Failed to create serial port: {port_name}");
    }
    log_send_blocking!(output_sender, Debug, "Serial port created successfully");

    let mut samba = bossa::lib::new_samba();
    if samba.is_null() {
        anyhow::bail!("Failed to create Samba");
    }
    log_send_blocking!(output_sender, Debug, "Samba created successfully");

    if !samba.pin_mut().connect(serial_port, 115200) {
        anyhow::bail!("Failed to connect to device via Samba");
    }
    log_send_blocking!(output_sender, Info, "Connected to device via Samba");

    let mut device = bossa::lib::new_device(samba.pin_mut());
    device.pin_mut().create();
    log_send_blocking!(output_sender, Debug, "Device created successfully");

    let log_observer = bossa::ObserverCallback(log_observer_fn);
    let prog_observer_callback = bossa::ProgressCallback(prog_observer_fn);
    let mut observer = unsafe { bossa::lib::new_bossa_observer_with_progress(log_observer, prog_observer_callback) };
    if observer.is_null() {
        anyhow::bail!("Failed to create observer");
    }
    log_send_blocking!(output_sender, Debug, "Observer created successfully");

    let mut flasher = bossa::lib::new_flasher(
        samba.pin_mut(),
        device.pin_mut(),
        observer.pin_mut()
    );
    log_send_blocking!(output_sender, Debug, "Flasher created successfully");

    let mut flasher_info = bossa::lib::new_flasher_info();
    flasher.pin_mut().info(flasher_info.pin_mut());

    let mut info = bossa::lib::FlasherInfoRs::default();
    bossa::lib::flasherinfo2flasherinfors(&flasher_info.pin_mut(), &mut info);
    log_send_blocking!(output_sender, Info, "Device info: {}", info.info());

    log_send_blocking!(output_sender, Info, "Erasing flash...");
    set_phase(UploadPhase::Erasing);
    flasher.pin_mut().erase(TEKNIC_BOOTLOADER_OFFSET_ADDRESS);

    log_send_blocking!(output_sender, Info, "Writing firmware from {}", firmware_path);
    set_phase(UploadPhase::Writing);
    let_cxx_string!(firmware_path_cxx = firmware_path);
    unsafe {
        flasher.pin_mut().write(firmware_path_cxx.as_c_str().as_ptr(), TEKNIC_BOOTLOADER_OFFSET_ADDRESS);
    }

    log_send_blocking!(output_sender, Info, "Verifying firmware...");
    set_phase(UploadPhase::Verifying);
    let mut page_errors = 0u32;
    let mut total_errors = 0u32;
    let verify_ok = unsafe {
        flasher.pin_mut().verify(
            firmware_path_cxx.as_c_str().as_ptr(),
            &mut page_errors,
            &mut total_errors,
            TEKNIC_BOOTLOADER_OFFSET_ADDRESS
        )
    };

    if !verify_ok || total_errors > 0 {
        anyhow::bail!("Verification failed: {} page errors, {} total errors", page_errors, total_errors);
    }
    log_send_blocking!(output_sender, Info, "Firmware verification successful");

    log_send_blocking!(output_sender, Info, "Resetting device...");
    set_phase(UploadPhase::Resetting);
    flasher.pin_mut().reset();
    log_send_blocking!(output_sender, Info, "Reset complete");
    Ok(())
}

extern "C" fn log_observer_fn(message: &CxxString) {
    if let Ok(str_slice) = message.to_str() {
        let msg = str_slice.trim().to_string();
        if let Ok(guard) = LOG_SENDER.lock() {
            if let Some(sender) = guard.as_ref() {
                let _ = sender.send(UploadEvent::log(LogMsgType::BossaNative, msg));
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


async fn touch_to_bootloader(output: &mut mpsc::Sender<UploadEvent>, cc_serial_port_info: &SerialPortInfo) -> Result<SerialPortInfo> {
    let port_name = cc_serial_port_info.port_name.clone();
    match tokio_serial::new(&port_name, 1200).open_native_async(){
        Ok(cc_serial_port) => {
            log_send!(output, Info, "cc serial port opened");

            drop(cc_serial_port);

            log_send!(output, Info, "cc serial port closed");
        },
        Err(e) => {
            log_send!(output, Error, "Failed to open cc serial port at 1200bps: {} \n\
            This might not be a problem on windows. Error: {}", &port_name, e);
        }
    }



    timeout(Duration::from_secs(5), wait_for_serial_port_disconnect(UsbId::CLEARCORE_SERIAL))
        .await.context("Timed out waiting for cc serial port disconnect")?
        .context("unexpected error while waiting for serial port to disconnect after touching")?;

    log_send!(output, Info, "cc serial disconnected");

    let bootloader_port = timeout(
        Duration::from_secs(10), wait_for_serial_port(UsbId::CLEARCORE_BOOTLOADER)
    ).await.context("Timed out waiting for cc bootloader port")?
        .context("unexpected error while waiting for bootloader port to appear after touching")?;

    log::info!("cc bootloader found");

    Ok(bootloader_port)
}

pub async fn wait_for_serial_port(usb_id: UsbId) -> Result<SerialPortInfo> {
    loop {
        let ports = tokio_serial::available_ports()?;
        if let Some(port) = ports.iter().find(|&port| { is_specified_port(port, usb_id) }) {
            return Ok(port.clone());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}


pub async fn wait_for_serial_port_disconnect(usb_id: UsbId) -> Result<()> {
    loop {
        let ports = tokio_serial::available_ports()?;
        if ports.iter().any(|port| { is_specified_port(&port, usb_id) }) {
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        }
        return Ok(());
    }
}

pub fn is_specified_port(port_info: &SerialPortInfo, target_usb_id: UsbId) -> bool {
    match &port_info.port_type {
        SerialPortType::UsbPort(usb_info) => {
            let usb_id = UsbId::from(usb_info);
            usb_id == target_usb_id
        },
        _ => false
    }
}

fn log_minor_err<E: Display>(res: Result<(), E>) {
    if let Err(err) = res {
        log::warn!("Upload stream warning: {}", err);
    }
}

async fn touch_port(port_name: &str) -> Result<()> {
    let mut port = tokio_serial::new(port_name, 1200).open()?;
    port.write_data_terminal_ready(false)?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    drop(port);
    Ok(())
}

pub async fn find_serial_port(usb_id: UsbId) -> Result<SerialPortInfo> {
    let ports = tokio_serial::available_ports()?;
    match ports.iter().find(|&port| { is_specified_port(port, usb_id) }) {
        Some(port) => Ok(port.clone()),
        None => {
            log::warn!("port {usb_id:?} not found. All ports: {ports:?}");
            Err(anyhow::anyhow!("port {usb_id:?} not found"))
        }
    }
}

async fn wait_for_port(id: &UsbId, timeout: Duration) -> Result<String> {
    let start = Instant::now();
    loop {
        if let Ok(name) = find_port_async(id).await {
            return Ok(name);
        }
        if start.elapsed() > timeout {
            return Err(anyhow!("Timeout finding port {:?}", id));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}