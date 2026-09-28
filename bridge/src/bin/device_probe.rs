//! Checks the keypad over serial: lists ports, or opens one, shows a frame
//! on the LEDs, and prints what the keypad sends.
//!
//! ```text
//! device_probe                list serial ports
//! device_probe <port> [secs]  run against one port (default 20 s)
//! ```
//!
//! Each key press is answered with a white flash on that key.

use std::io::Read;
use std::time::{Duration, Instant};

use protocol::{encode, Decoder, DeviceMessage, Edge, HostMessage, Led, Mode, Rgb, MAX_FRAME_LEN};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => list(),
        [port, rest @ ..] => {
            let secs = rest.first().and_then(|s| s.parse().ok()).unwrap_or(20);
            run(port, Duration::from_secs(secs));
        }
    }
}

fn list() {
    for p in serialport::available_ports().unwrap_or_default() {
        match p.port_type {
            serialport::SerialPortType::UsbPort(u) => println!(
                "{}  {:04x}:{:04x}  product={:?} manufacturer={:?} serial={:?}",
                p.port_name, u.vid, u.pid, u.product, u.manufacturer, u.serial_number
            ),
            other => println!("{}  {other:?}", p.port_name),
        }
    }
}

fn send(port: &mut dyn serialport::SerialPort, msg: &HostMessage) {
    let mut buf = [0u8; MAX_FRAME_LEN];
    port.write_all(encode(msg, &mut buf)).expect("write");
}

fn run(name: &str, how_long: Duration) {
    let mut port = serialport::new(name, 115_200)
        .timeout(Duration::from_millis(50))
        .dtr_on_open(true)
        .open()
        .expect("open");
    port.clear(serialport::ClearBuffer::Input).ok();

    let led = |r, g, b, mode| Led { rgb: Rgb { r, g, b }, mode };
    let frame = HostMessage::Frame([
        led(255, 140, 0, Mode::Breathe),
        led(0, 255, 0, Mode::Solid),
        led(0, 0, 255, Mode::Blink),
    ]);
    send(&mut *port, &frame);
    println!("sent frame: left amber breathe, middle green solid, right blue blink");

    let mut decoder = Decoder::<DeviceMessage>::new();
    let start = Instant::now();
    let mut last_frame = Instant::now();
    let mut buf = [0u8; 64];
    while start.elapsed() < how_long {
        if last_frame.elapsed() >= Duration::from_secs(1) {
            send(&mut *port, &frame);
            last_frame = Instant::now();
        }
        let n = match port.read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
            Err(e) => panic!("read: {e}"),
        };
        for &b in &buf[..n] {
            match decoder.push(b) {
                Some(Ok(msg)) => {
                    println!("{:>6} ms  {msg:?}", start.elapsed().as_millis());
                    if let DeviceMessage::Key { pos, edge: Edge::Down } = msg {
                        send(&mut *port, &HostMessage::Flash { pos, rgb: Rgb { r: 255, g: 255, b: 255 } });
                    }
                }
                Some(Err(e)) => println!("decode error: {e:?}"),
                None => {}
            }
        }
    }
    println!("done; the keypad falls back to dim white 3 s after the last frame");
}
