use anyhow::{Context, Result};
use serialport::{SerialPort, SerialPortInfo};
use std::io::Read;
use std::time::Duration;
use iced::futures::SinkExt;
use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::Stream;
use iced::stream;
use crate::types::SerialConfig;




#[derive(Debug, Clone)]
pub enum SerialEvent {
    Connected(String),
    Data(Vec<u8>),
    Error(String),
    Disconnected,
}


pub fn listen(config: SerialConfig) -> Subscription<SerialEvent> {
    Subscription::run(iced::stream::channel(
        100,
        // We must tell the compiler exactly what type Iced is giving us here:
        move |mut output: iced::futures::channel::mpsc::Sender<SerialEvent>| async move {

            // 1. Create a BRIDGE channel (Tokio)
            // The blocking thread will send to THIS sender.
            let (sender, mut receiver) = tokio::sync::mpsc::channel(100);

            // 2. Spawn the Blocking I/O Thread
            // We move the 'sender' into the thread so it can talk back to us.
            std::thread::spawn(move || {
                loop {
                    // --- Connection Logic ---
                    let port_name = match find_clearcore_port(&config) {
                        Ok(name) => name,
                        Err(_) => {
                            // Retry wait
                            std::thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                    };

                    let mut port = match open_serial_port(&port_name, &config) {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = sender.blocking_send(SerialEvent::Error(e.to_string()));
                            std::thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                    };

                    if sender.blocking_send(SerialEvent::Connected(port_name)).is_err() { break; }

                    // --- Read Loop ---
                    let mut buffer = [0u8; 4096];
                    loop {
                        match port.read(&mut buffer) {
                            Ok(n) if n > 0 => {
                                if sender.blocking_send(SerialEvent::Data(buffer[..n].to_vec())).is_err() {
                                    break; // Channel closed (UI stopped listening)
                                }
                            }
                            Ok(_) => {}
                            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                                // Timeout is GOOD. It lets us check if the sender is closed.
                                if sender.is_closed() { break; }
                            }
                            Err(e) => {
                                let _ = sender.blocking_send(SerialEvent::Error(e.to_string()));
                                break;
                            }
                        }
                    }

                    let _ = sender.blocking_send(SerialEvent::Disconnected);

                    if sender.is_closed() { break; }
                    std::thread::sleep(Duration::from_secs(1));
                }
            });

            // 3. Bridge Loop (Async)
            // We receive from the thread (Tokio) and forward to Iced (Output)
            while let Some(event) = receiver.recv().await {
                // This .send() requires 'use iced::futures::SinkExt;'
                let _ = output.send(event).await;
            }
        },
    ))
}



/// Find a serial port matching the given VID/PID
pub fn find_clearcore_port(config: &SerialConfig) -> Result<String> {
    let ports = serialport::available_ports()
        .context("Failed to enumerate serial ports")?;

    for port in ports {
        if let SerialPortInfo {
            port_name,
            port_type: serialport::SerialPortType::UsbPort(usb_info),
        } = port
        {
            if usb_info.vid == config.vendor_id && usb_info.pid == config.product_id {
                log::info!("Found ClearCore at {}", port_name);
                return Ok(port_name);
            }
        }
    }

    anyhow::bail!(
        "ClearCore not found (VID: 0x{:04X}, PID: 0x{:04X})",
        config.vendor_id,
        config.product_id
    )
}



/// Open a serial port with the given configuration
pub fn open_serial_port(port_name: &str, config: &SerialConfig) -> Result<Box<dyn SerialPort>> {
    let port = serialport::new(port_name, config.baud_rate)
        .timeout(Duration::from_millis(100))
        .open()
        .context(format!("Failed to open serial port {}", port_name))?;

    log::info!("Opened serial port {} at {} baud", port_name, config.baud_rate);
    Ok(port)
}

/// Read available data from serial port
pub fn read_serial_data(port: &mut Box<dyn SerialPort>) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; 1024];
    match port.read(&mut buffer) {
        Ok(n) if n > 0 => {
            buffer.truncate(n);
            Ok(buffer)
        }
        Ok(_) => Ok(Vec::new()),
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(Vec::new()),
        Err(e) => Err(e).context("Failed to read from serial port"),
    }
}
