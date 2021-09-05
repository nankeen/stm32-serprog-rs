use embedded_hal::spi::{Mode, Phase, Polarity};
use stm32f1xx_hal::{
    afio::MAPR,
    gpio::gpioa::{CRL, PA4, PA5, PA6, PA7},
    gpio::{Alternate, Floating, Input, Output, PushPull},
    pac::SPI1,
    rcc::{Clocks, APB2},
    spi::{Spi, Spi1NoRemap},
    time::Hertz,
};

type SpiPins = (
    PA5<Alternate<PushPull>>, // sck
    PA6<Input<Floating>>,     // miso
    PA7<Alternate<PushPull>>, // mosi
);

pub(crate) enum SpiManager {
    SpiDisabled {
        cs: PA4<Input<Floating>>,
        sck: PA5<Input<Floating>>,
        miso: PA6<Input<Floating>>,
        mosi: PA7<Input<Floating>>,
        spi: SPI1,
        clocks: Clocks,
    },
    SpiEnabled {
        spi: Spi<SPI1, Spi1NoRemap, SpiPins, u8>,
        cs: PA4<Output<PushPull>>,
        clocks: Clocks,
    },
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
        Self::SpiDisabled {
            cs,
            sck,
            miso,
            mosi,
            spi,
            clocks,
        }
    }

    pub(crate) fn release(
        self,
        crl: &mut CRL,
    ) -> (
        SPI1,
        PA4<Input<Floating>>,
        PA5<Input<Floating>>,
        PA6<Input<Floating>>,
        PA7<Input<Floating>>,
    ) {
        match self {
            Self::SpiDisabled {
                spi,
                cs,
                sck,
                miso,
                mosi,
                ..
            } => (spi, cs, sck, miso, mosi),
            Self::SpiEnabled { .. } => self.disable(crl).release(crl),
        }
    }

    pub(crate) fn disable(self, crl: &mut CRL) -> Self {
        match self {
            Self::SpiDisabled { .. } => self,
            Self::SpiEnabled { spi, cs, clocks } => {
                let (spi, (sck, miso, mosi)) = spi.release();
                Self::SpiDisabled {
                    cs: cs.into_floating_input(crl),
                    sck: sck.into_floating_input(crl),
                    miso,
                    mosi: mosi.into_floating_input(crl),
                    spi,
                    clocks,
                }
            }
        }
    }

    /*
    pub(crate) fn spi_enable<F: Into<Hertz>, REMAP, PINS>(
        self,
        freq: F,
        mapr: &mut MAPR,
        crl: &mut CRL,
        apb: &mut APB2,
    ) -> SpiEnabled
    where
        F: Into<Hertz>,
    {
        let pins = (
            self.sck.into_alternate_push_pull(crl),
            self.miso,
            self.mosi.into_alternate_push_pull(crl),
        );

        let spi_mode = Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        };

        let spi = Spi::spi1(self.spi, pins, mapr, spi_mode, freq, self.clocks, apb);
        SpiEnabled {
            spi,
            clocks: self.clocks,
            cs: self.cs.into_push_pull_output(crl),
        }
    }
    */
}
