//! herdr-triton keypad firmware: wires the `keypad` core to the
//! RP2040-Keyboard-3's keys, WS2812 LEDs and USB.
//!
//! Keys: left GP14, middle GP13, right GP12 (active low). LEDs: WS2812 chain
//! on GP18, L1..L3 left to right. USB: CDC serial carrying `protocol`
//! frames, plus `drooling::PicotoolReset` for button-free flashing.

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use keypad::{led_order, Keypad, Millis};
use panic_halt as _;
use protocol::{encode, Decoder, DeviceMessage, HostMessage, MAX_FRAME_LEN};
use rp2040_hal::{self as hal, clocks::Clock, pac, pio::PIOExt};
use smart_leds::{SmartLedsWrite, RGB8};
use usb_device::{class_prelude::UsbBusAllocator, device::UsbRev, prelude::*, LangID};
use usbd_serial::SerialPort;
use ws2812_pio::Ws2812Direct;

use drooling::PicotoolReset;

#[link_section = ".boot2"]
#[no_mangle]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
/// How often the LEDs are redrawn, for the animations.
const REDRAW: Millis = 10;

#[entry]
fn main() -> ! {
    let serial_number = unique_serial_number();

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
    let now = || timer.get_counter().ticks() / 1000;

    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
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
            .serial_number(serial_number)])
        .unwrap()
        .usb_rev(UsbRev::Usb210)
        .max_packet_size_0(64)
        .unwrap()
        .composite_with_iads()
        .build();

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);
    let mut keys = [
        pins.gpio14.into_pull_up_input().into_dyn_pin(),
        pins.gpio13.into_pull_up_input().into_dyn_pin(),
        pins.gpio12.into_pull_up_input().into_dyn_pin(),
    ];
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut leds = Ws2812Direct::new(
        pins.gpio18.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
    );

    let mut keypad = Keypad::new();
    let mut decoder = Decoder::<HostMessage>::new();
    let mut next_redraw: Millis = 0;

    loop {
        usb_dev.poll(&mut [&mut serial, &mut picotool]);
        keypad.set_dtr(serial.dtr());

        let mut rx = [0u8; 64];
        if let Ok(n) = serial.read(&mut rx) {
            for &byte in &rx[..n] {
                if let Some(Ok(msg)) = decoder.push(byte) {
                    if let Some(reply) = keypad.receive(msg, now()) {
                        send(&mut serial, &reply);
                    }
                }
            }
        }

        let pressed = keys.each_mut().map(|k| k.is_low().unwrap_or(false));
        for msg in keypad.scan(pressed, now()).into_iter().flatten() {
            send(&mut serial, &msg);
        }

        let t = now();
        if t >= next_redraw {
            next_redraw = t + REDRAW;
            let pixels = keypad.pixels(t).map(|rgb| {
                let o = led_order(rgb);
                RGB8::new(o.r, o.g, o.b)
            });
            leds.write(pixels).ok();
        }
    }
}

/// Writes one frame without blocking; a frame that does not fit is dropped
/// and the host's decoder resynchronises at the next terminator.
fn send(serial: &mut SerialPort<hal::usb::UsbBus>, msg: &DeviceMessage) {
    let mut buf = [0u8; MAX_FRAME_LEN];
    let _ = serial.write(encode(msg, &mut buf));
}

/// `TRITON-` and the flash chip's 64-bit unique id in hex, read before
/// anything else touches flash. The host finds the keypad by this prefix;
/// Windows reports the serial number but not the product string.
fn unique_serial_number() -> &'static str {
    const PREFIX: &[u8] = b"TRITON-";
    static mut TEXT: [u8; PREFIX.len() + 16] = [0; PREFIX.len() + 16];
    let mut id = [0u8; 8];
    cortex_m::interrupt::free(|_| unsafe { rp2040_flash::flash::flash_unique_id(&mut id, true) });
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    unsafe {
        #[allow(static_mut_refs)]
        let text = &mut TEXT;
        text[..PREFIX.len()].copy_from_slice(PREFIX);
        for (i, b) in id.iter().enumerate() {
            text[PREFIX.len() + 2 * i] = HEX[(b >> 4) as usize];
            text[PREFIX.len() + 2 * i + 1] = HEX[(b & 0xf) as usize];
        }
        core::str::from_utf8_unchecked(text)
    }
}
