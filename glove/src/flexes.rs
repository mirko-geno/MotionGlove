use embassy_rp::adc::{self, Adc, Error as AdcError};

pub const THUMB: usize = 0;
pub const INDEX: usize = 1;
pub const MIDDLE: usize = 2;
pub type FingerReadings = [u16; 3];

pub struct FingerFlexes<'a> {
    adc_driver: Adc<'a, adc::Async>,
    thumb_flex: adc::Channel<'a>,
    index_flex: adc::Channel<'a>,
    middle_flex: adc::Channel<'a>,
}

impl<'a> FingerFlexes<'a> {
    pub fn new(adc_driver: Adc<'a, adc::Async>, thumb_flex: adc::Channel<'a>, index_flex: adc::Channel<'a>, middle_flex: adc::Channel<'a>,) -> Self {
        FingerFlexes {adc_driver, thumb_flex, index_flex, middle_flex}
    }

    pub async fn read(&mut self) -> Result<FingerReadings, AdcError> {
        let thumb   = self.adc_driver.read(&mut self.thumb_flex).await?;
        let index   = self.adc_driver.read(&mut self.index_flex).await?;
        let middle  = self.adc_driver.read(&mut self.middle_flex).await?;
        Ok([thumb, index, middle])
    }
}
