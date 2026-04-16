use crate::serial::{get_or_touch_to_bootloader, log_minor_err, wait_for_serial_port};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use anyhow::{Context, Result};
use iced::futures::{SinkExt, Stream};
use iced::{stream, Subscription};
use iced::futures::channel::mpsc;
use cxx::{let_cxx_string, CxxString};
use crate::types::{LogMsg, LogMsgType, UsbId};
use crate::serial::TEKNIC_BOOTLOADER_OFFSET_ADDRESS;
use std::str;

use bossa;

use tokio::time::timeout;
use crate::{log_send, log_send_blocking};

static PROGRESS_STATE: Mutex<ProgressState> = Mutex::new(ProgressState::new());
static LOG_SENDER: Mutex<Option<tokio::sync::mpsc::UnboundedSender<UploadEvent>>> = Mutex::new(None);


#[derive(Debug, Clone)]
pub enum UploadEvent {
    Log(LogMsg),
    Error(String),
    ProgressBarUpdate(UploadProgressBar),
    Success,
}

impl UploadEvent {
    pub fn log(log_type: LogMsgType, message: impl Into<String>) -> Self {
        Self::Log(LogMsg::new(log_type, message))
    }
}

impl From<LogMsg> for UploadEvent {
    fn from(log: LogMsg) -> Self {
        UploadEvent::Log(log)
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
pub struct UploadProgressBar {
    pub phase: UploadPhase,
    pub current: u32,
    pub total: u32,
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

    let bootloader_port_info = get_or_touch_to_bootloader(output).await?;


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
                     let _ = output.try_send(UploadEvent::ProgressBarUpdate(UploadProgressBar {
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
