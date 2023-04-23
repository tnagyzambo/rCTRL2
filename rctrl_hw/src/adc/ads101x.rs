use super::super::sensor::Sensor;
use anyhow::Result;
use cfg_if::cfg_if;
use i2cdev::core::*;
#[cfg(target_os = "linux")]
use i2cdev::linux::LinuxI2CDevice;
#[cfg(not(any(target_os = "linux")))]
use i2cdev::mock::MockI2CDevice;

// Register map of ADS101X devices
const CONVERSION_REG: u8 = 0x00;
const CONFIG_REG: u8 = 0x01;
const LO_THRESH_REG: u8 = 0x02;
const HI_THRESH_REG: u8 = 0x03;

// Bit offsets for the different sections of the u16 config
const OS_OFFSET: u8 = 15;
const MUX_OFFSET: u8 = 12;
const PGA_OFFSET: u8 = 9;
const MODE_OFFSET: u8 = 8;
const DATA_RATE_OFFSET: u8 = 5;
const COMP_MODE_OFFSET: u8 = 4;
const COMP_POLARITY_OFFSET: u8 = 3;
const COMP_LATCH_OFFSET: u8 = 2;
const COMP_QUEUE_OFFSET: u8 = 0;

/// Operational status or single-shot conversion start.
///
/// This bit determines the operational status of the device. OS can only be written
/// when in power-down state and has no effect when a conversion is ongoing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Os {
    /// When writing: No effect
    ///
    /// When reading: Device is currently performing a conversion
    Off = 0,
    /// When writing: Start a single conversion (when in power-down state)
    ///
    /// When reading: Device is not currently performing a conversion
    On = 1,
}

impl Default for Os {
    fn default() -> Self {
        Os::On
    }
}

impl From<u16> for Os {
    fn from(word: u16) -> Self {
        match (word & 0x8000) >> OS_OFFSET {
            0 => Os::Off,
            1 => Os::On,
            _ => unreachable!(),
        }
    }
}

/// Input multiplexer configuration (ADS1015 only)
///
/// These bits configure the input multiplexer. These bits serve no function on the
/// ADS1013 and ADS1014.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mux {
    /// AINP = AIN0 and AINN = AIN1 (default)
    Ain0Ain1 = 0,
    /// AINP = AIN0 and AINN = AIN3
    Ain0Ain3 = 1,
    /// AINP = AIN1 and AINN = AIN3
    Ain1Ain3 = 2,
    /// AINP = AIN2 and AINN = AIN3
    Ain2Ain3 = 3,
    /// AINP = AIN0 and AINN = GND
    Ain0Gnd = 4,
    /// AINP = AIN1 and AINN = GND
    Ain1Gnd = 5,
    /// AINP = AIN2 and AINN = GND
    Ain2Gnd = 6,
    /// AINP = AIN3 and AINN = GND
    Ain3Gnd = 7,
}

impl Default for Mux {
    fn default() -> Self {
        Self::Ain0Ain1
    }
}

impl From<u16> for Mux {
    fn from(word: u16) -> Self {
        match (word & 0x7000) >> MUX_OFFSET {
            0 => Mux::Ain0Ain1,
            1 => Mux::Ain0Ain3,
            2 => Mux::Ain1Ain3,
            3 => Mux::Ain2Ain3,
            4 => Mux::Ain0Gnd,
            5 => Mux::Ain1Gnd,
            6 => Mux::Ain2Gnd,
            7 => Mux::Ain3Gnd,
            _ => unreachable!(),
        }
    }
}

/// Programmable gain amplifier configuration
///
/// These bits set the FSR of the programmable gain amplifier. These bits serve no
/// function on the ADS1013.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pga {
    /// FSR = ±6.144 V
    Fsr6_144V = 0,
    /// FSR = ±4.096 V
    Fsr4_096V = 1,
    /// FSR = ±2.048 V (default)
    Fsr2_048V = 2,
    /// FSR = ±1.024 V
    Fsr1_024V = 3,
    /// FSR = ±0.512 V
    Fsr0_512V = 4,
    /// FSR = ±0.256 V
    Fsr0_256V = 5,
}

impl Default for Pga {
    fn default() -> Self {
        Self::Fsr2_048V
    }
}

impl From<u16> for Pga {
    fn from(word: u16) -> Self {
        match (word & 0x0E00) >> PGA_OFFSET {
            0 => Pga::Fsr6_144V,
            1 => Pga::Fsr4_096V,
            2 => Pga::Fsr2_048V,
            3 => Pga::Fsr1_024V,
            4 => Pga::Fsr0_512V,
            5 => Pga::Fsr0_256V,
            6 => Pga::Fsr0_256V,
            7 => Pga::Fsr0_256V,
            _ => unreachable!(),
        }
    }
}

impl Pga {
    /// Return the LSB size of the current `Pga` configuration in volts.
    fn as_lsb(self) -> f64 {
        match self {
            Self::Fsr6_144V => 3E-3,
            Self::Fsr4_096V => 2E-3,
            Self::Fsr2_048V => 1E-3,
            Self::Fsr1_024V => 0.5E-3,
            Self::Fsr0_512V => 0.25E-3,
            Self::Fsr0_256V => 0.125E-3,
        }
    }
}

/// Device operating mode
///
/// This bit controls the operating mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Continuous-conversion mode
    Continuous,
    /// Single-shot mode or power-down state (default)
    SingleShot,
}

impl Default for Mode {
    fn default() -> Self {
        Self::SingleShot
    }
}

impl From<u16> for Mode {
    fn from(word: u16) -> Self {
        match (word & 0x0100) >> MODE_OFFSET {
            0 => Mode::Continuous,
            1 => Mode::SingleShot,
            _ => unreachable!(),
        }
    }
}

/// Data rate
///
/// These bits control the data rate setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataRate {
    /// 128 SPS
    Sps128 = 0,
    /// 250 SPS
    Sps250 = 1,
    /// 490 SPS
    Sps490 = 2,
    /// 920 SPS
    Sps920 = 3,
    /// 1600 SPS (default)
    Sps1600 = 4,
    /// 2400 SPS
    Sps2400 = 5,
    /// 3300 SPS
    Sps3300 = 6,
}

impl Default for DataRate {
    fn default() -> Self {
        Self::Sps1600
    }
}

impl From<u16> for DataRate {
    fn from(word: u16) -> Self {
        match (word & 0x00E0) >> DATA_RATE_OFFSET {
            0 => DataRate::Sps128,
            1 => DataRate::Sps250,
            2 => DataRate::Sps490,
            3 => DataRate::Sps920,
            4 => DataRate::Sps1600,
            5 => DataRate::Sps2400,
            6 => DataRate::Sps3300,
            7 => DataRate::Sps3300,
            _ => unreachable!(),
        }
    }
}

/// Comparator mode (ADS1014 and ADS1015 only)
///
/// This bit configures the comparator operating mode. This bit serves no function on
/// the ADS1013.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompMode {
    /// Traditional comparator (default)
    Traditional = 0,
    /// Window comparator
    Window = 1,
}

impl Default for CompMode {
    fn default() -> Self {
        Self::Traditional
    }
}

impl From<u16> for CompMode {
    fn from(word: u16) -> Self {
        match (word & 0x0010) >> COMP_MODE_OFFSET {
            0 => CompMode::Traditional,
            1 => CompMode::Window,
            _ => unreachable!(),
        }
    }
}

/// Comparator polarity (ADS1014 and ADS1015 only)
///
/// This bit controls the polarity of the ALERT/RDY pin. This bit serves no function on
/// the ADS1013.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompPolarity {
    /// Active low (default)
    ActiveLow = 0,
    /// Active high
    ActiveHigh = 1,
}

impl Default for CompPolarity {
    fn default() -> Self {
        Self::ActiveLow
    }
}

impl From<u16> for CompPolarity {
    fn from(word: u16) -> Self {
        match (word & 0x0008) >> COMP_POLARITY_OFFSET {
            0 => CompPolarity::ActiveLow,
            1 => CompPolarity::ActiveHigh,
            _ => unreachable!(),
        }
    }
}

/// Latching comparator (ADS1014 and ADS1015 only)
///
/// This bit controls whether the ALERT/RDY pin latches after being asserted or
/// clears after conversions are within the margin of the upper and lower threshold
/// values. This bit serves no function on the ADS1013.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompLatch {
    /// Nonlatching comparator. The ALERT/RDY pin does not latch when asserted (default).
    Nonlatching = 0,
    /// Latching comparator. The asserted ALERT/RDY pin remains latched until
    /// conversion data are read by the master or an appropriate SMBus alert response
    /// is sent by the master. The device responds with its address, and it is the lowest
    /// address currently asserting the ALERT/RDY bus line.
    Latching = 1,
}

impl Default for CompLatch {
    fn default() -> Self {
        Self::Nonlatching
    }
}

impl From<u16> for CompLatch {
    fn from(word: u16) -> Self {
        match (word & 0x0004) >> COMP_LATCH_OFFSET {
            0 => CompLatch::Nonlatching,
            1 => CompLatch::Latching,
            _ => unreachable!(),
        }
    }
}

/// Comparator queue and disable (ADS1014 and ADS1015 only)
///
/// These bits perform two functions. When set to 11, the comparator is disabled and
/// the ALERT/RDY pin is set to a high-impedance state. When set to any other
/// value, the ALERT/RDY pin and the comparator function are enabled, and the set
/// value determines the number of successive conversions exceeding the upper or
/// lower threshold required before asserting the ALERT/RDY pin. These bits serve
/// no function on the ADS1013.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompQueue {
    /// Assert after one conversion
    OneConversion = 0,
    /// Assert after two conversions
    TwoConversion = 1,
    /// Assert after four conversions
    FourConversion = 2,
    /// Disable comparator and set ALERT/RDY pin to high-impedance (default)
    Disable = 3,
}

impl Default for CompQueue {
    fn default() -> Self {
        Self::Disable
    }
}

impl From<u16> for CompQueue {
    fn from(word: u16) -> Self {
        match (word & 0x0003) >> COMP_QUEUE_OFFSET {
            0 => CompQueue::OneConversion,
            1 => CompQueue::TwoConversion,
            2 => CompQueue::FourConversion,
            3 => CompQueue::Disable,
            _ => unreachable!(),
        }
    }
}

/// The 16-bit Config register is used to control the operating mode, input selection, data rate, full-scale range, and
/// comparator modes.
///
/// This design goal of this struct is to have each configurtaion strongly typed
/// but also ergonomic to use. All fields are private with public methods to set config options.
///
/// For detailed information see Section 8.6.3 of the datasheet.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Config {
    os: Os,
    mux: Mux,
    pga: Pga,
    mode: Mode,
    data_rate: DataRate,
    comp_mode: CompMode,
    comp_polarity: CompPolarity,
    comp_latch: CompLatch,
    comp_queue: CompQueue,
}

impl From<u16> for Config {
    fn from(word: u16) -> Self {
        Self {
            os: Os::from(word),
            mux: Mux::from(word),
            pga: Pga::from(word),
            mode: Mode::from(word),
            data_rate: DataRate::from(word),
            comp_mode: CompMode::from(word),
            comp_polarity: CompPolarity::from(word),
            comp_latch: CompLatch::from(word),
            comp_queue: CompQueue::from(word),
        }
    }
}

impl Config {
    pub fn with_os(mut self, os: Os) -> Self {
        self.os = os;
        self
    }

    pub fn with_mux(mut self, mux: Mux) -> Self {
        self.mux = mux;
        self
    }

    pub fn with_pga(mut self, pga: Pga) -> Self {
        self.pga = pga;
        self
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_data_rate(mut self, data_rate: DataRate) -> Self {
        self.data_rate = data_rate;
        self
    }

    pub fn with_comp_mode(mut self, comp_mode: CompMode) -> Self {
        self.comp_mode = comp_mode;
        self
    }

    pub fn with_comp_polarity(mut self, comp_polarity: CompPolarity) -> Self {
        self.comp_polarity = comp_polarity;
        self
    }

    pub fn with_comp_latch(mut self, comp_latch: CompLatch) -> Self {
        self.comp_latch = comp_latch;
        self
    }

    pub fn with_comp_queue(mut self, comp_queue: CompQueue) -> Self {
        self.comp_queue = comp_queue;
        self
    }
}

impl From<Config> for u16 {
    fn from(config: Config) -> Self {
        let os = (config.os as u16) << OS_OFFSET;
        let mux = (config.mux as u16) << MUX_OFFSET;
        let pga = (config.pga as u16) << PGA_OFFSET;
        let mode = (config.mode as u16) << MODE_OFFSET;
        let data_rate = (config.data_rate as u16) << DATA_RATE_OFFSET;
        let comp_mode = (config.comp_mode as u16) << COMP_MODE_OFFSET;
        let comp_polarity = (config.comp_polarity as u16) << COMP_POLARITY_OFFSET;
        let comp_latch = (config.comp_latch as u16) << COMP_LATCH_OFFSET;
        let comp_queue = (config.comp_queue as u16) << COMP_QUEUE_OFFSET;

        os | mux | pga | mode | data_rate | comp_mode | comp_polarity | comp_latch | comp_queue
    }
}

pub struct ADS101x {
    /// Platform specific implementation of i2c device
    #[cfg(target_os = "linux")]
    dev: LinuxI2CDevice,
    #[cfg(not(any(target_os = "linux")))]
    dev: MockI2CDevice,
    /// Current configuration of ADS101x device
    config: Config,
}

impl ADS101x {
    /// Creates a new `ADS101x` device.
    ///
    /// Uses the platofrm specific implementation for the compile target.
    /// Defaults to a mock I2C device for unimplemented targets.
    /// Will return an error if the config of the created `ADS101x` device cannot be read.
    ///
    /// # Arguments
    /// * `path` - Linux path to I2C deivce.
    /// * `addr` - I2C address of `ADS101x` device.
    #[allow(unused_variables)]
    pub fn new(path: &str, addr: u16) -> Result<Self> {
        cfg_if! {
            if #[cfg(target_os = "linux")] {
                Self::new_linux(path, addr)
            } else {
                Self::new_mock()
            }
        }
    }

    /// Creates a new `ADS101x` device for Linux targets.
    ///
    /// Will return an error if the config of the created `ADS101x` device cannot be read.
    ///
    /// # Arguments
    /// * `path` - Linux path to I2C deivce.
    /// * `addr` - I2C address of `ADS101x` device.
    #[cfg(target_os = "linux")]
    fn new_linux(path: &str, addr: u16) -> Result<Self> {
        let dev = LinuxI2CDevice::new(path, addr)?;
        let config = Config::from(dev.smbus_read_word_data(CONFIG_REG)?);

        Ok(Self { dev, config })
    }

    /// Creates a mock `ADS101x` for unimplemented targets.
    ///
    /// Will return an error if the config of the created `ADS101x` device cannot be read.
    #[cfg(not(any(target_os = "linux")))]
    fn new_mock() -> Result<Self> {
        let mut dev = MockI2CDevice::new();

        // Create register map of ADS101x with default config
        dev.regmap
            .write_regs(CONVERSION_REG as usize, &[0x00, 0x00]);
        dev.regmap.write_regs(CONFIG_REG as usize, &[0x85, 0x83]);
        dev.regmap.write_regs(LO_THRESH_REG as usize, &[0x80, 0x00]);
        dev.regmap.write_regs(HI_THRESH_REG as usize, &[0xFF, 0xF8]);

        let config = Config::from(dev.smbus_read_word_data(CONFIG_REG)?);

        Ok(Self { dev, config })
    }

    /// Configure `ADS101x` device.
    ///
    /// Will return an error if the config is not read back from the `ADS101x` device correctly after being set.
    ///
    /// # Arguments
    /// * `config` - `Config` to be sent as u16 to the `ADS101x` device.
    pub fn config(&mut self, config: Config) -> Result<()> {
        self.dev.smbus_write_word_data(CONFIG_REG, config.into())?;
        self.config = Config::from(self.dev.smbus_read_word_data(CONFIG_REG)?);

        if self.config != config {
            // TODO: Create proper error
            //Err("failed to set config")
        }

        Ok(())
    }

    /// Read the current voltage being read by the `ADS101x`.
    fn read_raw(&mut self) -> Result<f64> {
        // Raw value is read in two's compliment format
        let mut raw = self.dev.smbus_read_word_data(CONVERSION_REG)?;
        let msb: u16 = raw & 0xFF;
        let lsb: u16 = raw & 0xFF00;

        // Switch msb and lsb positions and shift left to get 12 bit value
        raw = (msb << 8 | lsb) >> 4;

        // Check if negative and flip bits as per two's compliment
        if (raw & 0x8000) != 0 {
            raw = 0xF000 | raw;
        }

        // Multiply by pga setting
        let voltage: f64 = (raw as i16 as f64) * self.config.pga.as_lsb();

        Ok(voltage)
    }

    /// Read the `ADS101x` device and apply a sensor transformation.
    ///
    /// Transform the read voltage into a sensor reading by passing in a sensor conversion function
    ///
    /// # Arguments
    /// * `sensor` - Any sensor that implements the `Sensor` trait
    pub fn read<T: Sensor>(&mut self, sensor: &T) -> Result<<T as Sensor>::Output> {
        let voltage = self.read_raw()?;

        Ok(sensor.conversion(voltage))
    }

    // TODO: Create functions for reading ADS1015 channels.
    // Potentially create dedicated ADS1013, ADS1014 and ADS1015 structs
}
