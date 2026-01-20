use anyhow::{Context, Result};
use serialport::{SerialPort, SerialPortInfo};
use std::io::Read;
use std::time::Duration;
use iced::futures::SinkExt;
use iced::Subscription;
use iced::stream;
use crate::types::SerialConfig;




#[derive(Debug, Clone)]
pub enum SerialEvent {
    Data(String),
    Error(String),
}

/// Synchronously connect to the ClearCore serial port
pub fn connect_to_clearcore(config: &SerialConfig) -> Result<Box<dyn SerialPort>> {
    let port_name = find_clearcore_port(config)?;
    let port = open_serial_port(&port_name, config)?;
    Ok(port)
}

/// Connect to and listen to the ClearCore serial port
pub fn listen() -> Subscription<SerialEvent> {
    Subscription::run(connect_and_listen)
}

fn connect_and_listen() -> impl iced::futures::Stream<Item = SerialEvent> {
    stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<SerialEvent>| async move {
        use iced::futures::SinkExt;

        // Connect to the serial port
        let config = SerialConfig::default();
        let mut port = match connect_to_clearcore(&config) {
            Ok(p) => p,
            Err(e) => {
                let _ = output.send(SerialEvent::Error(e.to_string())).await;
                return;
            }
        };

        // Run blocking I/O in spawn_blocking
        let _ = tokio::task::spawn_blocking(move || {
            let mut buffer = [0u8; 4096];
            let mut line_buffer = String::new();

            loop {
                match port.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        // Convert bytes to string and accumulate
                        if let Ok(text) = String::from_utf8(buffer[..n].to_vec()) {
                            line_buffer.push_str(&text);

                            // Send complete lines
                            while let Some(newline_pos) = line_buffer.find('\n') {
                                let line = line_buffer[..=newline_pos].to_string();
                                // Use blocking send within the blocking task
                                if iced::futures::executor::block_on(output.send(SerialEvent::Data(line))).is_err() {
                                    return; // Channel closed
                                }
                                line_buffer = line_buffer[newline_pos + 1..].to_string();
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        // Continue on timeout
                    }
                    Err(e) => {
                        let _ = iced::futures::executor::block_on(output.send(SerialEvent::Error(e.to_string())));
                        break;
                    }
                }
            }
        }).await;
    })
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
