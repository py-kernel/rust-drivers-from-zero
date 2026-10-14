use embedded_hal::i2c::ErrorKind;
use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as T};
use lm75_guide::{Address, Error, Lm75};

const ADDR: u8 = 0x48;
const TEMPERATURE: u8 = 0x00;
const CONFIGURATION: u8 = 0x01;

fn read_temperature_with(bytes: [u8; 2]) -> f32 {
    let expectations = [T::write_read(ADDR, vec![TEMPERATURE], bytes.to_vec())];
    let mut sensor = Lm75::new(I2cMock::new(&expectations), Address::default());

    let value = sensor.read_temperature().unwrap();

    sensor.destroy().done();
    value
}

#[test]
fn address_from_pins() {
    assert_eq!(Address::default().bits(), 0x48);
    assert_eq!(Address::from_pins(false, false, true).bits(), 0x49);
    assert_eq!(Address::from_pins(true, false, true).bits(), 0x4D);
    assert_eq!(Address::from_pins(true, true, true).bits(), 0x4F);
}

#[test]
fn positive_temperature() {
    // 0x1900 -> 200 шагов по 0,125 = 25,0 °C
    assert_eq!(read_temperature_with([0x19, 0x00]), 25.0);
}

#[test]
fn temperature_with_fraction() {
    // 0x0020 -> один шаг = 0,125 °C
    assert_eq!(read_temperature_with([0x00, 0x20]), 0.125);
}

#[test]
fn negative_temperature() {
    // 0xE700 -> -200 шагов = -25,0 °C
    assert_eq!(read_temperature_with([0xE7, 0x00]), -25.0);
}

#[test]
fn range_limits() {
    assert_eq!(read_temperature_with([0x7D, 0x00]), 125.0);
    assert_eq!(read_temperature_with([0xC9, 0x00]), -55.0);
}

#[test]
fn unused_low_bits_are_ignored() {
    // Пять младших бит не определены, значение не должно меняться.
    assert_eq!(read_temperature_with([0x19, 0x1F]), 25.0);
}

#[test]
fn raw_value_is_in_eighths() {
    let expectations = [T::write_read(ADDR, vec![TEMPERATURE], vec![0x19, 0x00])];
    let mut sensor = Lm75::new(I2cMock::new(&expectations), Address::default());

    assert_eq!(sensor.read_temperature_raw().unwrap(), 200);

    sensor.destroy().done();
}

#[test]
fn uses_configured_address() {
    let expectations = [T::write_read(0x4D, vec![TEMPERATURE], vec![0x19, 0x00])];
    let address = Address::from_pins(true, false, true);
    let mut sensor = Lm75::new(I2cMock::new(&expectations), address);

    sensor.read_temperature().unwrap();

    sensor.destroy().done();
}

#[test]
fn shutdown_keeps_other_bits() {
    let expectations = [
        T::write_read(ADDR, vec![CONFIGURATION], vec![0b0001_0000]),
        T::write(ADDR, vec![CONFIGURATION, 0b0001_0001]),
    ];
    let mut sensor = Lm75::new(I2cMock::new(&expectations), Address::default());

    sensor.shutdown().unwrap();

    sensor.destroy().done();
}

#[test]
fn wake_keeps_other_bits() {
    let expectations = [
        T::write_read(ADDR, vec![CONFIGURATION], vec![0b0001_0001]),
        T::write(ADDR, vec![CONFIGURATION, 0b0001_0000]),
    ];
    let mut sensor = Lm75::new(I2cMock::new(&expectations), Address::default());

    sensor.wake().unwrap();

    sensor.destroy().done();
}

#[test]
fn bus_error_is_passed_up() {
    let expectations =
        [T::write_read(ADDR, vec![TEMPERATURE], vec![0, 0]).with_error(ErrorKind::Other)];
    let mut sensor = Lm75::new(I2cMock::new(&expectations), Address::default());

    let result = sensor.read_temperature();

    assert!(matches!(result, Err(Error::I2c(ErrorKind::Other))));
    sensor.destroy().done();
}
