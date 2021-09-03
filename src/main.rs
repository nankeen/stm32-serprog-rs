// std and main are not available for bare metal software
#![no_std]
#![no_main]

mod data_utils;
mod serprog;

use cortex_m::asm::delay;
use cortex_m_rt::entry; // The runtime
use data_utils::OpCode;
use embedded_hal::digital::v2::OutputPin;
use serprog::SerProg;
use stm32f1xx_hal::usb::{Peripheral, UsbBus};
use stm32f1xx_hal::{pac, prelude::*}; // STM32F1 specific functions
use usb_device::prelude::{UsbDeviceBuilder, UsbVidPid};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

#[allow(unused_imports)]
use panic_halt; // When a panic occurs, stop the microcontroller

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
    let mut rcc = dp.RCC.constrain();

    // Configure the clock
    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

    // Pull down PA12 (D+ pin) to send a RESET condition to the USB bus
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low().unwrap();
    delay(clocks.sysclk().0 / 100);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };

    let usb_bus = UsbBus::new(usb);

    let serial = SerialPort::new(&usb_bus);

    // VID: ST Microelectronics
    // PID: STM32
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x0483, 0x5740))
        .manufacturer("nankeen.me")
        .product("STM32 serprog")
        .serial_number("CAFEBABE")
        .device_class(USB_CLASS_CDC)
        .build();

    let mut serprog = SerProg::new(serial, usb_dev);
    let mut response_buffer = [0u8; data_utils::ResponsePacket::MAX_SIZE];

    // Loop to handle commands
    loop {
        // Read opcode from USB serial
        if let Some(cmd) = OpCode::from_u8(serprog.read_u8()) {
            // Pass it to the command handler
            if let Ok(res) = serprog.handle_command(cmd) {
                // Serialize and respond
                if let Ok(n) = res.to_bytes(&mut response_buffer) {
                    serprog.send_response(&response_buffer[..n]);
                }
            }
        }
    }
}
