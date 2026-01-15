use embedded_hal::{
    blocking::spi::{Transfer, Write},
    digital::v2::OutputPin,
    spi::{Mode, Phase, Polarity},
};
use stm32f1xx_hal::{
    afio::MAPR,
    gpio::gpioa::{CRL, PA4, PA5, PA6, PA7},
    gpio::{Alternate, Floating, Input, Output, PushPull},
    pac::SPI1,
    rcc::{Clocks, APB2},
    spi::{Spi, Spi1NoRemap},
    time::{Hertz, U32Ext},
};

const SPI_MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

type SpiPins = (
    PA5<Alternate<PushPull>>, // sck
    PA6<Input<Floating>>,     // miso
    PA7<Alternate<PushPull>>, // mosi
);

struct SpiDisabled {
    cs: PA4<Input<Floating>>,
    sck: PA5<Input<Floating>>,
    miso: PA6<Input<Floating>>,
    mosi: PA7<Input<Floating>>,
    spi: SPI1,
}

struct SpiEnabled {
    cs: PA4<Output<PushPull>>,
    spi: Spi<SPI1, Spi1NoRemap, SpiPins, u8>,
}

pub(crate) struct SpiManager {
    /*
    cs:   Option<PA4<Input<Floating>>>,
    sck:  Option<PA5<Input<Floating>>>,
    miso: Option<PA6<Input<Floating>>>,
    mosi: Option<PA7<Input<Floating>>>,
    spi: Option<SPI1>,
    spi_hal: Option<Spi<SPI1, Spi1NoRemap, SpiPins, u8>>,
    */
    disabled: Option<SpiDisabled>,
    enabled: Option<SpiEnabled>,
    clocks: Clocks,
    pub(crate) last_freq: Hertz,
}

impl SpiManager {
    pub(crate) fn new(
        cs: PA4<Input<Floating>>,
        sck: PA5<Input<Floating>>,
        miso: PA6<Input<Floating>>,
        mosi: PA7<Input<Floating>>,
        spi: SPI1,
        clocks: Clocks,
    ) -> Self {
        Self {
            enabled: None,
            disabled: Some(SpiDisabled {
                cs,
                sck,
                miso,
                mosi,
                spi,
            }),
            clocks,
            last_freq: 1_000_000_u32.hz(),
        }
    }

    pub(crate) fn disable(&mut self, crl: &mut CRL) {
        if let Some(SpiEnabled { cs, spi }) = self.enabled.take() {
            let (spi, (sck, miso, mosi)) = spi.release();
            self.disabled = Some(SpiDisabled {
                cs: cs.into_floating_input(crl),
                sck: sck.into_floating_input(crl),
                miso: miso.into_floating_input(crl),
                mosi: mosi.into_floating_input(crl),
                spi,
            });
        }
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.disabled.is_some()
    }

    pub(crate) fn enable<F>(&mut self, freq: F, mapr: &mut MAPR, crl: &mut CRL, apb: &mut APB2)
    where
        F: Into<Hertz>,
    {
        if let Some(SpiDisabled {
            cs,
            sck,
            miso,
            mosi,
            spi,
        }) = self.disabled.take()
        {
            let pins = (
                sck.into_alternate_push_pull(crl),
                miso,
                mosi.into_alternate_push_pull(crl),
            );
            let spi = Spi::spi1(spi, pins, mapr, SPI_MODE, freq, self.clocks, apb);
            self.enabled = Some(SpiEnabled {
                cs: cs.into_push_pull_output(crl),
                spi,
            });
        }
    }

    /// Configures the SPI frequency if self is enabled, else it will be equivalent to enable()
    pub(crate) fn configure<F>(&mut self, freq: F, mapr: &mut MAPR, crl: &mut CRL, apb: &mut APB2)
    where
        F: Into<Hertz>,
    {
        let freq = freq.into();
        self.last_freq = freq;

        if self.enabled.is_some() {
            // If already enabled, reconfigure by disable then enable
            self.disable(crl);
            self.enable(freq, mapr, crl, apb);
        } else {
            // If disabled, just enable with new frequency
            self.enable(freq, mapr, crl, apb);
        }
    }

    /// Performs an SPI transfer with CS control: write then read
    pub(crate) fn transfer_with_cs(
        &mut self,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<(), ()> {
        if let Some(SpiEnabled { cs, spi }) = &mut self.enabled {
            // Assert CS low
            cs.set_low().map_err(|_| ())?;

            // Write phase: send write_data
            if !write_data.is_empty() {
                if let Err(_) = spi.write(write_data) {
                    // Deassert CS on error
                    let _ = cs.set_high();
                    return Err(());
                }
            }

            // Read phase: transfer dummy 0xFF bytes into read_buffer
            if !read_buffer.is_empty() {
                // Fill read buffer with dummy data (0xFF)
                for byte in read_buffer.iter_mut() {
                    *byte = 0xff;
                }

                // Perform transfer (sends 0xFF, receives actual data)
                if let Err(_) = spi.transfer(read_buffer) {
                    // Deassert CS on error
                    let _ = cs.set_high();
                    return Err(());
                }
            }

            // Deassert CS high
            cs.set_high().map_err(|_| ())?;

            Ok(())
        } else {
            // SPI not enabled
            Err(())
        }
    }
}
