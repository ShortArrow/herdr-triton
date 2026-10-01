//! Hardware probe for RP2040-Keyboard-3: shows which key and which LED
//! sit where.
//!
//! - Idle: WS2812 chain index 0 is red, 1 is green, 2 is blue.
//! - GP12 held: all three white. GP13 held: all yellow. GP14 held: all magenta.
//! - GP25 red LED blinks as a heartbeat.
//! - Each key edge is also written to CDC serial as `GP12 down` / `GP12 up`.
//!
//! USB is CDC serial plus `drooling::PicotoolReset`, so later flashes need
//! no BOOT button.

#![no_std]
#![warn(clippy::undocumented_unsafe_blocks)]
#![no_main]

use core::fmt::Write as _;

use cortex_m_rt::entry;
use embedded_hal::digital::{InputPin, OutputPin};
use panic_halt as _;
use rp2040_hal::{self as hal, clocks::Clock, fugit::ExtU32, pac, pio::PIOExt};
use smart_leds::{SmartLedsWrite, RGB8};
use usb_device::{class_prelude::UsbBusAllocator, device::UsbRev, prelude::*, LangID};
use usbd_serial::SerialPort;
use ws2812_pio::Ws2812Direct;

use drooling::PicotoolReset;

#[link_section = ".boot2"]
#[no_mangle]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
const LEVEL: u8 = 32;
const IDLE: [RGB8; 3] = [
    RGB8::new(LEVEL, 0, 0),
    RGB8::new(0, LEVEL, 0),
    RGB8::new(0, 0, LEVEL),
];
const HELD: [(u8, RGB8); 3] = [
    (12, RGB8::new(LEVEL, LEVEL, LEVEL)),
    (13, RGB8::new(LEVEL, LEVEL, 0)),
    (14, RGB8::new(LEVEL, 0, LEVEL)),
];

/// Picks the frame for the given held-key flags: the first held key's
/// colour on all LEDs, or the idle index colours when nothing is held.
fn frame(held: [bool; 3]) -> [RGB8; 3] {
    match held.iter().position(|h| *h) {
        Some(i) => [HELD[i].1; 3],
        None => IDLE,
    }
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
    // SAFETY: `main` runs once and never returns, so this is the only
    // access to USB_BUS, and the reference it hands out lives for good.
    let usb_bus = unsafe {
        USB_BUS = Some(UsbBusAllocator::new(hal::usb::UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        )));
        #[allow(static_mut_refs)]
        USB_BUS.as_ref().unwrap()
    };
    let mut serial = SerialPort::new(usb_bus);
    let mut picotool = PicotoolReset::new(usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x2e8a, 0x000a))
        .strings(&[StringDescriptors::new(LangID::EN_US)
            .manufacturer("ShortArrow")
            .product("herdr-triton")
            .serial_number("io-probe")])
        .unwrap()
        .usb_rev(UsbRev::Usb210)
        .max_packet_size_0(64)
        .unwrap()
        .composite_with_iads()
        .build();

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut keys = [
        pins.gpio12.into_pull_up_input().into_dyn_pin(),
        pins.gpio13.into_pull_up_input().into_dyn_pin(),
        pins.gpio14.into_pull_up_input().into_dyn_pin(),
    ];
    let mut heartbeat = pins.gpio25.into_push_pull_output();

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut leds = Ws2812Direct::new(
        pins.gpio18.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
    );

    let mut held = [false; 3];
    leds.write(frame(held)).ok();
    let mut next_beat = timer.get_counter() + 500.millis();
    let mut beat_on = false;

    loop {
        usb_dev.poll(&mut [&mut serial, &mut picotool]);

        let mut changed = false;
        for (i, key) in keys.iter_mut().enumerate() {
            let now_held = key.is_low().unwrap_or(false);
            if now_held != held[i] {
                held[i] = now_held;
                changed = true;
                let mut line: heapless_line::Line = Default::default();
                let _ = writeln!(
                    line,
                    "GP{} {}\r",
                    HELD[i].0,
                    if now_held { "down" } else { "up" }
                );
                let _ = serial.write(line.as_bytes());
            }
        }
        if changed {
            leds.write(frame(held)).ok();
        }

        let now = timer.get_counter();
        if now >= next_beat {
            next_beat = now + 500.millis();
            beat_on = !beat_on;
            let _ = if beat_on {
                heartbeat.set_high()
            } else {
                heartbeat.set_low()
            };
        }
    }
}

mod heapless_line {
    /// A fixed 32-byte line buffer for formatting serial output without
    /// allocation; text beyond 32 bytes is dropped.
    #[derive(Default)]
    pub struct Line {
        buf: [u8; 32],
        len: usize,
    }

    impl Line {
        pub fn as_bytes(&self) -> &[u8] {
            &self.buf[..self.len]
        }
    }

    impl core::fmt::Write for Line {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let n = s.len().min(self.buf.len() - self.len);
            self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
            self.len += n;
            Ok(())
        }
    }
}
