use embedded_hal::spi::{Mode, Phase, Polarity};
use stm32f1xx_hal::{
    afio::MAPR,
    gpio::{Alternate, Cr, Floating, Input, Output, PushPull, PA4, PA5, PA6, PA7},
    pac::SPI1,
    rcc::Clocks,
    spi::{Spi, Spi1NoRemap},
    time::Hertz,
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
    disabled: Option<SpiDisabled>,
    enabled: Option<SpiEnabled>,
    clocks: Clocks,
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
        }
    }

    pub fn disable(&mut self, crl: &mut Cr<'A', false>) {
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

    pub fn enable<F>(&mut self, freq: F, mapr: &mut MAPR, crl: &mut Cr<'A', false>)
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
            let spi = Spi::spi1(spi, pins, mapr, SPI_MODE, freq.into(), self.clocks);
            self.enabled = Some(SpiEnabled {
                cs: cs.into_push_pull_output(crl),
                spi,
            });
        }
    }

    /// Configures the SPI frequency if self is enabled, else it will be equivalent to enable()
    pub(crate) fn configure<F>(&mut self, _freq: F, _mapr: &mut MAPR, _crl: &mut Cr<'A', false>)
    where
        F: Into<Hertz>,
    {
        // TODO: Implement configure
        /*
        match self.enabled.take() {
            Some(SpiEnabled { cs, spi }) => {
            }
            None => {
                self.enabled = self.
            }
        }
        match self.spi_hal.take() {
            None => {
                self.enable(freq, mapr, crl, apb);
            }
            Some(spi_hal) => {
                let (spi1, pins) = spi_hal.release();
                self.spi_hal = Some(Spi::spi1(
                    spi1,
                    pins,
                    mapr,
                    SPI_MODE,
                    freq,
                    self.clocks,
                    apb,
                ));
            }
        }
            match self {
                Self::SpiDisabled { .. } => {
                    //
                },
                Self::SpiEnabled { spi, cs, clocks } => {
                    let (spi1, pins) = spi.release();
                    core::mem::replace(self, Self::SpiEnabled {
                        cs: *cs, spi: Spi::spi1(spi1, pins, mapr, SPI_MODE, freq, *clocks, apb), clocks: *clocks
                    });
                    /*

                    */
                }
            }
        */
    }
}
