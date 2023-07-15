use core::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
};

use embedded_hal::spi::{Mode, Phase, Polarity};
use stm32f1xx_hal::{
    dma::dma1::{C4, C5},
    pac::SPI2,
    prelude::_stm32_hal_dma_ReadWriteDma,
    rcc::Clocks,
    spi::{Master, Pins, Remap, Spi, SpiRxTxDma},
    time::Hertz,
};

use crate::{buffer::Buffer, prelude::SpiError};

const SPI_MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

pub(crate) struct SpiDisabled<REMAP, PINS>
where
    REMAP: Remap<Periph = SPI2>,
    PINS: Pins<REMAP>,
{
    pins: PINS,
    // cs: PA4<Input<Floating>>,
    spi: SPI2,
    clocks: Clocks,
    dma_channels: (C4, C5),
    _remap: PhantomData<REMAP>,
}

pub(crate) struct SpiEnabled<REMAP, PINS>
where
    REMAP: Remap<Periph = SPI2>,
    PINS: Pins<REMAP>,
{
    // cs: PA4<Output<PushPull>>,
    spi_dma: SpiRxTxDma<SPI2, REMAP, PINS, Master, C4, C5>,
    clocks: Clocks,
    _remap: PhantomData<REMAP>,
}

pub(crate) enum SpiManager<REMAP, PINS>
where
    REMAP: Remap<Periph = SPI2>,
    PINS: Pins<REMAP>,
{
    Disabled(SpiDisabled<REMAP, PINS>),
    Enabled(SpiEnabled<REMAP, PINS>),
}

impl<REMAP, PINS> SpiManager<REMAP, PINS>
where
    REMAP: Remap<Periph = SPI2>,
    PINS: Pins<REMAP>,
{
    pub(crate) fn new(
        pins: PINS,
        // cs: PA4<Input<Floating>>,
        spi: SPI2,
        clocks: Clocks,
        channels: (C4, C5),
    ) -> Self {
        Self::Disabled(SpiDisabled {
            pins,
            spi,
            clocks,
            dma_channels: channels,
            _remap: PhantomData,
        })
    }

    pub fn disable(self) -> Self {
        match self {
            Self::Enabled(SpiEnabled {
                spi_dma, clocks, ..
            }) => {
                let (spi, c4, c5) = spi_dma.release();
                let (spi, pins) = spi.release();
                Self::Disabled(SpiDisabled {
                    pins,
                    spi,
                    clocks,
                    dma_channels: (c4, c5),
                    _remap: PhantomData,
                })
            }
            Self::Disabled(SpiDisabled { .. }) => self,
        }
    }

    pub fn enable<F>(self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        match self {
            Self::Enabled(SpiEnabled { .. }) => self,
            Self::Disabled(SpiDisabled {
                pins,
                spi,
                clocks,
                dma_channels,
                ..
            }) => {
                let spi = Spi::spi2(spi, pins, SPI_MODE, freq.into(), clocks);

                // Setup DMA
                let spi_dma = spi.with_rx_tx_dma(dma_channels.0, dma_channels.1);

                Self::Enabled(SpiEnabled {
                    spi_dma,
                    clocks,
                    _remap: PhantomData,
                })
            }
        }
    }

    /// Configures the SPI frequency if self is enabled, else it will be equivalent to enable()
    // pub(crate) fn configure<F>(&mut self, _freq: F, _mapr: &mut MAPR, _crl: &mut Cr<'A', false>)
    pub(crate) fn configure<F>(self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.disable().enable(freq)
    }

    pub fn read_write<RX, TX>(
        self,
        rx_buffer: Buffer<RX>,
        tx_buffer: Buffer<TX>,
    ) -> Result<(Buffer<RX>, Buffer<TX>, Self), SpiError>
    where
        RX: BorrowMut<[u8]>,
        TX: Borrow<[u8]>,
    {
        match self {
            Self::Enabled(SpiEnabled {
                spi_dma,
                clocks,
                _remap,
            }) => {
                let ((rx_buffer, tx_buffer), spi_dma) =
                    spi_dma.read_write(rx_buffer, tx_buffer).wait();
                Ok((
                    rx_buffer,
                    tx_buffer,
                    Self::Enabled(SpiEnabled {
                        spi_dma,
                        clocks,
                        _remap,
                    }),
                ))
            }
            _ => Err(SpiError::NotEnabled),
        }
    }
}
