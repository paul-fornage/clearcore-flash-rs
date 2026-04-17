use iced::futures::{SinkExt, Stream};
use iced::{stream, Subscription};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_serial::{SerialPort, SerialPortBuilderExt};
use iced::futures::channel::mpsc;
use crate::serial::{find_port_async, log_minor_err};
use crate::types::{LogMsg, SerialConfig, UsbId};
use crate::ui::monitor_screen::MonitorConnectionState;

#[derive(Debug, Clone)]
pub enum SerialMonitorEvent {
    StateChange(MonitorConnectionState),
    Data(LogMsg),
    Ready(tokio::sync::mpsc::Sender<String>), // This is pretty bad. Don't like this approach
}



/// Entry point for the UI to request the serial subscription.
/// Subscription::run uses the function pointer `connect_and_listen` as an identity key.
/// As long as this Subscription is returned by your app, the stream continues running.
pub fn listen() -> Subscription<SerialMonitorEvent> {
    Subscription::run(connect_and_listen)
}

/// The actual stream logic. This is called ONCE when the subscription starts.
fn connect_and_listen() -> impl Stream<Item = SerialMonitorEvent> {
    stream::channel(100, |mut output: mpsc::Sender<SerialMonitorEvent>| async move {
        log::info!("Starting serial connection monitor");
        let config = SerialConfig::SERIAL_MONITOR;

        async fn state_change(output: &mut mpsc::Sender<SerialMonitorEvent>,
                              new_state: MonitorConnectionState
        ){
            log_minor_err(output.send(SerialMonitorEvent::StateChange(
                new_state
            )).await);
        }

        state_change(&mut output, MonitorConnectionState::Searching).await;

        // This loop handles the connection lifecycle (Auto-reconnect)
        loop {
            // Create a fresh input channel for each connection attempt.
            // The sender is forwarded to the UI so it can write to the serial port.
            let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<String>(32);
            log_minor_err(output.send(SerialMonitorEvent::Ready(cmd_tx)).await);

            // 1. Find the port asynchronously (retries internally if needed, or we retry here)
            let port_info = match find_port_async(config.usb_id).await {
                Ok(info) => info,
                Err(e) => {
                    match find_port_async(UsbId::CLEARCORE_BOOTLOADER).await {
                        Ok(info) => {
                            let name = info.port_name;
                            log::error!("Failed to find ClearCore serial port: {e:?}, but found bootloader!: {name}");
                            state_change(&mut output, MonitorConnectionState::Error(
                                format!("Failed to find ClearCore serial port, \
                                    but found the bootloader at {name}. Power cycle clearcore to \
                                    exit bootloader mode")
                            )).await;
                        },
                        Err(e) => {
                            log::error!("Failed to find ClearCore serial port: {e:?}");
                            state_change(&mut output, MonitorConnectionState::Error(
                                format!("Failed to find ClearCore serial port: {e:?}")
                            )).await;
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    continue;
                }
            };
            let port_name = port_info.port_name;
            log::info!("ClearCore serial port found: {}", &port_name);

            // 2. Report we are connecting
            state_change(&mut output, MonitorConnectionState::Connecting(port_name.clone())).await;

            // 3. Attempt to open the port
            let port_result = tokio_serial::new(&port_name, config.baud_rate)
                .timeout(Duration::from_millis(2000))
                .open_native_async();

            match port_result {
                Ok(mut port) => {
                    log::info!("Opened serial port. name: {:?}", port.name());
                    if let Err(e) = port.write_data_terminal_ready(true) {
                        log::warn!("Failed to set DTR: {}", e);
                    }
                    if let Err(e) = port.write_request_to_send(true) {
                        log::warn!("Failed to set RTS: {}", e);
                    }
                    // 4. Connected!
                    state_change(&mut output,
                                 MonitorConnectionState::Connected(port_name.clone())).await;

                    // 5. Split port into read/write halves and enter the read/write loop
                    let (read_half, mut write_half) = tokio::io::split(port);
                    let mut reader = BufReader::new(read_half);
                    let mut line = String::new();
                    // Use Option so we can fall back to read-only if the sender is dropped
                    let mut cmd_rx_opt = Some(cmd_rx);

                    'read_loop: loop {
                        line.clear();

                        let read_result = if let Some(ref mut rx) = cmd_rx_opt {
                            tokio::select! {
                                result = reader.read_line(&mut line) => result,
                                cmd = rx.recv() => {
                                    match cmd {
                                        Some(text) => {
                                            let bytes = format!("{text}\n").into_bytes();
                                            if let Err(e) = write_half.write_all(&bytes).await {
                                                log::error!("Failed to write to serial port: {e:?}");
                                            }
                                        }
                                        None => {
                                            // Sender was dropped; switch to read-only mode
                                            cmd_rx_opt = None;
                                        }
                                    }
                                    continue 'read_loop;
                                }
                            }
                        } else {
                            reader.read_line(&mut line).await
                        };

                        match read_result {
                            Ok(0) => {
                                log::warn!("EOF: Device disconnected");
                                state_change(&mut output, MonitorConnectionState::Error(
                                                 "Device disconnected".to_string())).await;
                                break; // Break read loop, go back to search loop
                            }
                            Ok(_) => {
                                let line_trimmed = line.trim();
                                log::trace!("Received serial data: {}", line_trimmed);
                                match output.send(SerialMonitorEvent::Data(LogMsg::new_cc(line_trimmed))).await {
                                    Ok(_) => {}
                                    Err(e) => if e.is_disconnected() {
                                        log::warn!("Serial output disconnected, dropping data: {}", line_trimmed);
                                        return; // Listener cancelled (UI changed screens), stop everything.
                                    } else {
                                        log::error!("unexpected Serial output buffer error: {e:?}\n\
                                            dropping data: {}", line_trimmed);
                                    }

                                }
                            }
                            Err(e) => {
                                log::error!("IO error: {:?}", e);
                                state_change(&mut output,
                                             MonitorConnectionState::Error(e.to_string())).await;
                                break; // Break read loop
                            }
                        }
                    }
                }
                Err(e) => {
                    // Connection failed immediately
                    log::warn!("Failed to open serial port: {e:?}");
                    state_change(&mut output,
                                 MonitorConnectionState::Error(e.to_string())).await;
                }
            }

            // If we are here, the connection was lost or failed.
            // Wait a bit before searching again to avoid tight loops.
            tokio::time::sleep(Duration::from_secs(1)).await;
            state_change(&mut output,
                         MonitorConnectionState::Searching).await;
        }
    })
}
