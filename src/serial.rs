use anyhow::{Context, Result};
use serialport::{SerialPort, SerialPortInfo};
use std::io::Read;
use std::time::Duration;

use crate::types::SerialConfig;

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
