// std and main are not available for bare metal software
#![no_std]
#![no_main]

mod address;
mod buffer;
mod command;
mod constants;
mod opcode;
mod response_packet;
mod serprog;
mod spi;

use buffer::Buffer;
use cortex_m::asm::delay;
use cortex_m_rt::entry; // The runtime
use embedded_alloc::Heap;
use serprog::SerProg;
use stm32f1xx_hal::{
    pac,
    prelude::*,
    usb::{Peripheral, UsbBus},
};
use usb_device::prelude::{UsbDeviceBuilder, UsbVidPid};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

#[allow(unused_imports, clippy::single_component_path_imports)]
use panic_halt; // When a panic occurs, stop the microcontroller

#[global_allocator]
static HEAP: Heap = Heap::empty();

mod prelude {
    pub(crate) use crate::address::*;
    pub(crate) use crate::constants::*;
    pub(crate) use crate::opcode::*;
    pub(crate) use crate::response_packet::*;
    pub(crate) use anyhow::{anyhow, bail, Result};
}

#[entry]
fn main() -> ! {
    // Get handles to the hardware objects. These functions can only be called
    // once, so that the borrowchecker can ensure you don't reconfigure
    // something by accident.
    let dp = pac::Peripherals::take().unwrap();

    // GPIO pins on the STM32F1 must be driven by the APB2 peripheral clock.
    // This must be enabled first. The HAL provides some abstractions for
    // us: First get a handle to the RCC peripheral:
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    // Configure the clock
    let clocks = rcc
        .cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .pclk1(24.MHz())
        .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut gpioa = dp.GPIOA.split();
    let mut gpiob = dp.GPIOB.split();

    // Pull down PA12 (D+ pin) to send a RESET condition to the USB bus
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low();
    delay(clocks.sysclk().to_Hz() / 100);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };

    let usb_bus = UsbBus::new(usb);

    // VID: ST Microelectronics
    // PID: STM32
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x0483, 0x5740))
        .manufacturer("nankeen.me")
        .product("STM32 serprog")
        .serial_number("CAFEBABE")
        .device_class(USB_CLASS_CDC)
        .build();

    let serial = SerialPort::new(&usb_bus);

    // Setup SPI
    let (sck, miso, mosi) = (
        gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh),
        gpiob.pb14.into_pull_down_input(&mut gpiob.crh),
        gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh),
    );

    // Setup DMA
    let dma1 = dp.DMA1.split();

    let spi = spi::SpiManager::new((sck, miso, mosi), dp.SPI2, clocks, (dma1.4, dma1.5));
    let mut serprog = SerProg::new(spi, serial, usb_dev);
    let mut response_buffer = [0u8; response_packet::ResponsePacket::MAX_SIZE];
    let mut ser_buf = Buffer::new([0u8; command::Command::MAX_SIZE]);

    // Loop to handle commands
    loop {
        match serprog.process_command(&mut ser_buf) {
            Ok(resp) => {
                let n = resp.to_bytes(&mut response_buffer).unwrap();
                serprog.send_response(&response_buffer[..n])
            }
            Err(_) => ser_buf.clear(),
        }
    }
}
