//! Учебный драйвер датчика температуры LM75.
//!
//! Драйвер написан на `embedded-hal` 1.0 и не зависит от конкретной платы.
//! Ему подходит любая шина, которая реализует трейт [`embedded_hal::i2c::I2c`].
//!
//! Подробный разбор каждой строки - в главе 4 руководства.

#![no_std]
#![deny(missing_docs)]

use embedded_hal::i2c::I2c;

/// Номера регистров LM75 (раздел "Pointer register" в datasheet).
mod register {
    pub const TEMPERATURE: u8 = 0x00;
    pub const CONFIGURATION: u8 = 0x01;
}

/// Бит 0 регистра конфигурации. Единица переводит датчик в режим энергосбережения.
const SHUTDOWN_BIT: u8 = 0b0000_0001;

/// Вес младшего бита температурного значения, в градусах Цельсия.
const DEGREES_PER_STEP: f32 = 0.125;

/// Адрес датчика на шине I2C.
///
/// Семь бит адреса: старшие четыре фиксированы (`1001`), младшие три
/// задаются выводами A2, A1 и A0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address(u8);

impl Address {
    /// Собирает адрес по состоянию выводов A2, A1, A0.
    ///
    /// `true` означает, что вывод подключён к питанию, `false` - к земле.
    pub const fn from_pins(a2: bool, a1: bool, a0: bool) -> Self {
        Address(0b100_1000 | ((a2 as u8) << 2) | ((a1 as u8) << 1) | (a0 as u8))
    }

    /// Семибитный адрес в том виде, в котором его принимает шина.
    pub const fn bits(self) -> u8 {
        self.0
    }
}

impl Default for Address {
    /// Все три вывода на земле, адрес `0x48`.
    fn default() -> Self {
        Self::from_pins(false, false, false)
    }
}

/// Ошибки драйвера.
#[derive(Debug)]
pub enum Error<E> {
    /// Ошибка шины I2C. Тип `E` задаёт реализация шины.
    I2c(E),
}

/// Драйвер датчика LM75.
#[derive(Debug)]
pub struct Lm75<I2C> {
    i2c: I2C,
    address: Address,
}

impl<I2C: I2c> Lm75<I2C> {
    /// Создаёт драйвер. Обращений к шине при этом не происходит.
    pub fn new(i2c: I2C, address: Address) -> Self {
        Lm75 { i2c, address }
    }

    /// Разбирает драйвер и возвращает шину, чтобы её мог использовать другой код.
    pub fn destroy(self) -> I2C {
        self.i2c
    }

    /// Читает температуру в "сырых" единицах по 0,125 °C.
    ///
    /// Регистр температуры хранит 11-битное число со знаком в старших битах
    /// 16-битного слова. Пять младших бит не используются, поэтому значение
    /// сдвигается вправо. Сдвиг знаковый, и отрицательные температуры
    /// сохраняют знак.
    pub fn read_temperature_raw(&mut self) -> Result<i16, Error<I2C::Error>> {
        let mut buffer = [0u8; 2];
        self.i2c
            .write_read(self.address.bits(), &[register::TEMPERATURE], &mut buffer)
            .map_err(Error::I2c)?;
        Ok(i16::from_be_bytes(buffer) >> 5)
    }

    /// Читает температуру в градусах Цельсия.
    pub fn read_temperature(&mut self) -> Result<f32, Error<I2C::Error>> {
        let raw = self.read_temperature_raw()?;
        Ok(f32::from(raw) * DEGREES_PER_STEP)
    }

    /// Переводит датчик в режим энергосбережения.
    ///
    /// Остальные биты регистра конфигурации сохраняются.
    pub fn shutdown(&mut self) -> Result<(), Error<I2C::Error>> {
        self.modify_configuration(|config| config | SHUTDOWN_BIT)
    }

    /// Выводит датчик из режима энергосбережения.
    ///
    /// Остальные биты регистра конфигурации сохраняются.
    pub fn wake(&mut self) -> Result<(), Error<I2C::Error>> {
        self.modify_configuration(|config| config & !SHUTDOWN_BIT)
    }

    /// Читает регистр конфигурации, меняет значение и записывает обратно.
    fn modify_configuration(
        &mut self,
        change: impl FnOnce(u8) -> u8,
    ) -> Result<(), Error<I2C::Error>> {
        let mut current = [0u8; 1];
        self.i2c
            .write_read(
                self.address.bits(),
                &[register::CONFIGURATION],
                &mut current,
            )
            .map_err(Error::I2c)?;

        let updated = change(current[0]);
        self.i2c
            .write(self.address.bits(), &[register::CONFIGURATION, updated])
            .map_err(Error::I2c)
    }
}
