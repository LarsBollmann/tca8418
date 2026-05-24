#![doc = include_str!("../README.md")]

//! ## Examples
//! The main usage is to create a [Tca8418] object using a i2c device.
//! You can then configure it and read [events](KeyEvent) from the FIFO queue.
//!
//! ### Mixed Keypad and GPIO
//!  
//! Pins not assigned to the keypad matrix can be used as GPIO inputs or outputs.
//!  
//! ```rust,no_run
//! use tca8418::{Tca8418, PinMask};
//! # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
//! // R0-R3 and C0-C3 as keypad matrix
//! let matrix_pins = PinMask::rows(0x0F) | PinMask::cols(0x0F);
//! keypad.configure_keypad(matrix_pins).unwrap();
//!  
//! // C4 as GPIO output
//! keypad.set_gpio_direction(PinMask::C4).unwrap();
//!  
//! // Set C4 high
//! keypad.write_gpio(PinMask::C4).unwrap();
//!  
//! // Read-modify-write: toggle C4 without affecting other pins
//! let current = keypad.read_gpio().unwrap();
//! keypad.write_gpio(current ^ PinMask::C4).unwrap();
//! # Ok(())
//! # }
//! ```
//!  
//! ### Async with Embassy
//!  
//! The sync driver works in async contexts. Use your HAL's async GPIO to await the interrupt pin instead of busy-polling.
//!  
//! ```rust,ignore
//! use tca8418::{Tca8418, PinMask, InterruptFlags};
//!
//! #[embassy_executor::task]
//! async fn keyboard_task(
//!     mut keypad: Tca8418<I2c<'static>>,
//!     mut int_pin: ExtiInput<'static>,
//! ) {
//!     let pins = PinMask::rows(0x7F) | PinMask::cols(0xFF);
//!     keypad.configure_keypad(pins).unwrap();
//!
//!     // Key event interrupts need to be enabled to listen to events from the keypad matrix
//!     keypad.enable_key_event_interrupt(true).unwrap();
//!
//!     // If pins are configured as GPI pins, the GPI interrupt needs to be enabled as well
//!     // keypad.enable_gpi_interrupt(true).unwrap();
//!  
//!     loop {
//!         int_pin.wait_for_falling_edge().await;
//!         for event in keypad.events()? {
//!             if let Some(key) = event.pressed_keypad() {
//!                 // handle keypress
//!             }
//!         }
//!         keypad.clear_interrupts(InterruptFlags::K_INT).unwrap();
//!     }
//! }
//! ```
//!  
//! ## Pin Masks
//!  
//! The [`PinMask`] type is used throughout the API to specify a mask for the row and column pins. You can construct masks using constants or helper functions:
//!  
//! ```rust
//! use tca8418::PinMask;
//!  
//! // Individual pins
//! let mask = PinMask::R0 | PinMask::R1 | PinMask::C0 | PinMask::C1;
//!  
//! // Row/column helpers
//! let mask = PinMask::rows(0x0F) | PinMask::cols(0x03FF);
//!  
//! // Predefined constants
//! let all = PinMask::ALL;
//! let no_pins = PinMask::NONE;
//! let all_rows = PinMask::ALL_ROWS;
//!  
//! // Modify existing masks
//! let with_c4 = mask.with(PinMask::C4);
//! let without_r0 = mask.without(PinMask::R0);
//!  
//! // Query
//! if mask.contains(PinMask::R0) { /* R0 is set */ }
//! if mask.intersects(PinMask::ALL_ROWS) { /* at least one row is set */ }
//! ```

#![warn(missing_docs)]
#![no_std]

pub mod events;
mod pins;
pub mod registers;

#[cfg(not(feature = "async"))]
use embedded_hal::i2c::I2c;
#[cfg(feature = "async")]
use embedded_hal_async::i2c::I2c;
pub use pins::PinMask;
pub use registers::{Config, Register};

#[doc(inline)]
pub use registers::InterruptFlags;

#[doc(inline)]
pub use events::{Key, KeyEvent};

#[doc(hidden)]
pub use events::{GpiKey, KeypadMatrixKey};

/// Default I²C address for TCA8418 (7-bit: 0x34)
pub const DEFAULT_ADDRESS: u8 = 0x34;

#[doc(hidden)]
pub struct EventIter {
    events: [Option<KeyEvent>; 10],
    index: u8,
    count: u8,
}

impl Iterator for EventIter {
    type Item = KeyEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let event = self.events[self.index as usize];
        self.index += 1;
        event
    }
}

// ============================================================================
// Error Type
// ============================================================================

/// Driver errors
#[derive(Debug)]
pub enum Error<E> {
    /// I²C bus error
    I2c(E),
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::I2c(e)
    }
}

#[cfg(feature = "defmt")]
impl<E: defmt::Format> defmt::Format for Error<E> {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::I2c(e) => defmt::write!(f, "I2c({})", e),
        }
    }
}

// ============================================================================
// Driver
// ============================================================================

/// TCA8418 keypad scan IC driver.
///
/// Wraps an I²C bus and provides methods for configuring the keypad matrix,
/// reading key events, and controlling GPIO pins.
///
/// # Creating a Driver
///
/// Use [`Tca8418::new`] with the default I²C address (0x34), or
/// [`Tca8418::with_address`] if you want to configure a different adress.
///
/// ```rust,no_run
/// # use embedded_hal::i2c::I2c;
/// use tca8418::Tca8418;
/// # fn run<I2C: I2c>(i2c: I2C) {
/// let mut keypad = Tca8418::new(i2c);
/// # }
/// ```
///
/// # Basic Usage
///
/// Configure which row/column pins form the keypad matrix, then poll or
/// use interrupts to read events:
/// See [KeyEvent] for more information about the returned event type.
///
/// ```rust,no_run
/// # use embedded_hal::i2c::I2c;
/// use tca8418::{Tca8418, PinMask};
/// # fn run<I2C: I2c>(i2c: I2C) {
/// let mut keypad = Tca8418::new(i2c);
///
/// // Assign rows 0–2 and columns 0–3 to the keypad matrix
/// let mask = PinMask::rows(0b0000_0111) | PinMask::cols(0b0000_0000_1111);
/// keypad.configure_keypad(mask).unwrap();
///
/// // Read a single event from the FIFO
/// if let Some(event) = keypad.read_event().unwrap() {
///     // Do something...
/// }
/// # }
/// ```
///
/// # Draining the FIFO
///
/// Use [events()](`Tca8418::events`) to create an iterator that iterates over all pending events, or
/// [read_all_events()](`Tca8418::read_all_events`)  to collect them into a fixed-size array.
///
/// # Releasing the Bus
///
/// Call [release()](`Tca8418::release`) to consume the driver and get the I²C
/// peripheral back.
pub struct Tca8418<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C, E> Tca8418<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Create a new TCA8418 driver with the default I²C address (0x34).
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: DEFAULT_ADDRESS,
        }
    }

    /// Create a new TCA8418 driver with a custom I²C address.
    pub fn with_address(i2c: I2C, addr: u8) -> Self {
        Self { i2c, addr }
    }

    /// Release the I²C bus, consuming the driver.
    pub fn release(self) -> I2C {
        self.i2c
    }

    // Read events

    /// Get the number of events currently in the FIFO (0–10).
    #[cfg(not(feature = "async"))]
    pub fn event_count(&mut self) -> Result<u8, Error<E>> {
        let val = self.read_register(Register::KeyLckEc)?;
        Ok(val & 0x0F)
    }

    /// Get the number of events currently in the FIFO (0–10).
    #[cfg(feature = "async")]
    pub async fn event_count(&mut self) -> Result<u8, Error<E>> {
        let val = self.read_register(Register::KeyLckEc).await?;
        Ok(val & 0x0F)
    }

    /// Read a single key event from the FIFO.
    ///
    /// Returns `None` if the FIFO is empty.
    #[cfg(not(feature = "async"))]
    pub fn read_event(&mut self) -> Result<Option<KeyEvent>, Error<E>> {
        let raw = self.read_register(Register::KeyEventA)?;
        Ok(KeyEvent::from_raw(raw))
    }

    /// Read a single key event from the FIFO.
    ///
    /// Returns `None` if the FIFO is empty.
    #[cfg(feature = "async")]
    pub async fn read_event(&mut self) -> Result<Option<KeyEvent>, Error<E>> {
        let raw = self.read_register(Register::KeyEventA).await?;
        Ok(KeyEvent::from_raw(raw))
    }

    /// Drain all pending events from the FIFO into a fixed-size array
    /// and return an iterator over them.
    ///
    /// This reads the event count once, drains that many events,
    /// and returns an iterator that requires no further I²C access.
    #[cfg(not(feature = "async"))]
    pub fn events(&mut self) -> Result<EventIter, Error<E>> {
        let count = self.event_count()?;
        let mut events = [None; 10];
        for i in 0..count.min(10) {
            events[i as usize] = self.read_event()?;
        }
        Ok(EventIter {
            events,
            index: 0,
            count: count.min(10),
        })
    }

    /// Drain all pending events from the FIFO into a fixed-size array
    /// and return an iterator over them.
    ///
    /// This reads the event count once, drains that many events,
    /// and returns an iterator that requires no further I²C access.
    #[cfg(feature = "async")]
    pub async fn events(&mut self) -> Result<EventIter, Error<E>> {
        let count = self.event_count().await?;
        let mut events = [None; 10];
        for i in 0..count.min(10) {
            events[i as usize] = self.read_event().await?;
        }
        Ok(EventIter {
            events,
            index: 0,
            count: count.min(10),
        })
    }
    /// Drain all pending events from the FIFO.
    ///
    /// Returns a `heapless`-style fixed-size array of up to 10 events.
    /// The returned tuple is `(events_array, count)`.
    #[cfg(not(feature = "async"))]
    pub fn read_all_events(&mut self) -> Result<([Option<KeyEvent>; 10], u8), Error<E>> {
        let count = self.event_count()?;
        let mut events = [None; 10];
        for i in 0..count.min(10) {
            events[i as usize] = self.read_event()?;
        }
        Ok((events, count))
    }

    /// Drain all pending events from the FIFO.
    ///
    /// Returns a `heapless`-style fixed-size array of up to 10 events.
    /// The returned tuple is `(events_array, count)`.
    #[cfg(feature = "async")]
    pub async fn read_all_events(&mut self) -> Result<([Option<KeyEvent>; 10], u8), Error<E>> {
        let count = self.event_count().await?;
        let mut events = [None; 10];
        for i in 0..count.min(10) {
            events[i as usize] = self.read_event().await?;
        }
        Ok((events, count))
    }

    // ========================================================================
    // Low-level register access
    // ========================================================================

    /// Write a single byte to a register.
    #[cfg(feature = "async")]
    pub async fn write_register(&mut self, reg: Register, value: u8) -> Result<(), Error<E>> {
        self.i2c.write(self.addr, &[reg as u8, value]).await?;
        Ok(())
    }

    /// Write a single byte to a register.
    #[cfg(not(feature = "async"))]
    pub fn write_register(&mut self, reg: Register, value: u8) -> Result<(), Error<E>> {
        self.i2c.write(self.addr, &[reg as u8, value])?;
        Ok(())
    }

    /// Read a single byte from a register.
    #[cfg(feature = "async")]
    pub async fn read_register(&mut self, reg: Register) -> Result<u8, Error<E>> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.addr, &[reg as u8], &mut buf)
            .await?;
        Ok(buf[0])
    }

    /// Read a single byte from a register.
    #[cfg(not(feature = "async"))]
    pub fn read_register(&mut self, reg: Register) -> Result<u8, Error<E>> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.addr, &[reg as u8], &mut buf)?;
        Ok(buf[0])
    }

    #[cfg(not(feature = "async"))]
    pub(crate) fn write_multiple_registers(
        &mut self,
        reg_row: Register,
        reg_col_low: Register,
        reg_col_high: Register,
        pins: PinMask,
    ) -> Result<(), Error<E>> {
        self.write_register(reg_row, pins.row_bits())?;
        self.write_register(reg_col_low, pins.col_low_bits())?;
        self.write_register(reg_col_high, pins.col_high_bits())?;
        Ok(())
    }

    #[cfg(feature = "async")]
    pub(crate) async fn write_multiple_registers(
        &mut self,
        reg_row: Register,
        reg_col_low: Register,
        reg_col_high: Register,
        pins: PinMask,
    ) -> Result<(), Error<E>> {
        self.write_register(reg_row, pins.row_bits()).await?;
        self.write_register(reg_col_low, pins.col_low_bits())
            .await?;
        self.write_register(reg_col_high, pins.col_high_bits())
            .await?;
        Ok(())
    }

    #[cfg(not(feature = "async"))]
    pub(crate) fn read_multiple_registers(
        &mut self,
        reg_row: Register,
        reg_col_low: Register,
        reg_col_high: Register,
    ) -> Result<PinMask, Error<E>> {
        let row_bits = self.read_register(reg_row)?;
        let col_low_bits = self.read_register(reg_col_low)?;
        let col_high_bits = self.read_register(reg_col_high)?;
        Ok(PinMask::rows(row_bits)
            | PinMask::cols_low(col_low_bits)
            | PinMask::cols_high(col_high_bits))
    }

    #[cfg(feature = "async")]
    pub(crate) async fn read_multiple_registers(
        &mut self,
        reg_row: Register,
        reg_col_low: Register,
        reg_col_high: Register,
    ) -> Result<PinMask, Error<E>> {
        let row_bits = self.read_register(reg_row).await?;
        let col_low_bits = self.read_register(reg_col_low).await?;
        let col_high_bits = self.read_register(reg_col_high).await?;
        Ok(PinMask::rows(row_bits)
            | PinMask::cols_low(col_low_bits)
            | PinMask::cols_high(col_high_bits))
    }

    /// Modify a register using a read-modify-write cycle.
    #[cfg(not(feature = "async"))]
    pub fn modify_register<F>(&mut self, reg: Register, f: F) -> Result<(), Error<E>>
    where
        F: FnOnce(u8) -> u8,
    {
        let val = self.read_register(reg)?;
        self.write_register(reg, f(val))
    }

    /// Modify a register using a read-modify-write cycle.
    #[cfg(feature = "async")]
    pub async fn modify_register<F>(&mut self, reg: Register, f: F) -> Result<(), Error<E>>
    where
        F: FnOnce(u8) -> u8,
    {
        let val = self.read_register(reg).await?;
        self.write_register(reg, f(val)).await
    }

    // ========================================================================
    // Initialization & configuration
    // ========================================================================

    /// Configure which pins participate in the keypad matrix or which ones are used as GPIO.
    ///
    /// - `pins`: A bitmask of pins to configure as keypad pins. Use [`PinMask::rows`] and [`PinMask::cols`] to create the mask.
    ///
    /// Pins set to 1 are keypad pins; pins set to 0 are GPIO.
    #[cfg(not(feature = "async"))]
    pub fn configure_keypad(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::KpGpio1,
            Register::KpGpio2,
            Register::KpGpio3,
            pins,
        )?;
        Ok(())
    }

    /// Configure which pins participate in the keypad matrix or which ones are used as GPIO.
    ///
    /// - `pins`: A bitmask of pins to configure as keypad pins. Use [`PinMask::rows`] and [`PinMask::cols`] to create the mask.
    ///
    /// Pins set to 1 are keypad pins; pins set to 0 are GPIO.
    #[cfg(feature = "async")]
    pub async fn configure_keypad(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::KpGpio1,
            Register::KpGpio2,
            Register::KpGpio3,
            pins,
        )
        .await?;
        Ok(())
    }

    /// Enable or disable key event interrupts on the INT pin.
    #[cfg(not(feature = "async"))]
    pub fn enable_key_event_interrupt(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::KE_IEN
            } else {
                v & !Config::KE_IEN
            }
        })
    }

    /// Enable or disable key event interrupts on the INT pin.
    #[cfg(feature = "async")]
    pub async fn enable_key_event_interrupt(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::KE_IEN
            } else {
                v & !Config::KE_IEN
            }
        })
        .await
    }

    /// Enable or disable GPI (general purpose input) interrupts on the INT pin.
    #[cfg(not(feature = "async"))]
    pub fn enable_gpi_interrupt(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::GPI_IEN
            } else {
                v & !Config::GPI_IEN
            }
        })
    }

    /// Enable or disable GPI (general purpose input) interrupts on the INT pin.
    #[cfg(feature = "async")]
    pub async fn enable_gpi_interrupt(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::GPI_IEN
            } else {
                v & !Config::GPI_IEN
            }
        })
        .await
    }

    /// Enable or disable overflow interrupts on the INT pin.
    ///
    /// Also sets the OVR_FLOW_M to 1 when enabled and to 0 while disabled.
    /// Both need to be enabled for the interrupt output to be pulled low when overflow occurs.
    /// **Note:** See the TCA8418 datasheet errata section for overflow behavior issues.
    #[cfg(not(feature = "async"))]
    pub fn enable_overflow_interrupt(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::OVR_FLOW_IEN | Config::OVR_FLOW_M
            } else {
                v & !Config::OVR_FLOW_IEN & !Config::OVR_FLOW_M
            }
        })
    }

    /// Enable or disable overflow interrupts on the INT pin.
    ///
    /// Also sets the OVR_FLOW_M to 1 when enabled and to 0 while disabled.
    /// Both need to be enabled for the interrupt output to be pulled low when overflow occurs.
    /// **Note:** See the TCA8418 datasheet errata section for overflow behavior issues.
    #[cfg(feature = "async")]
    pub async fn enable_overflow_interrupt(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::OVR_FLOW_IEN | Config::OVR_FLOW_M
            } else {
                v & !Config::OVR_FLOW_IEN & !Config::OVR_FLOW_M
            }
        })
        .await
    }

    /// Set overflow mode.
    /// - `false`: FIFO stops accepting events when full
    /// - `true`: FIFO wraps, oldest events are overwritten
    ///
    /// Needs to be enabled for overflow interrupts to work.
    /// **Note:** See the TCA8418 datasheet errata section for overflow behavior issues.
    #[cfg(not(feature = "async"))]
    pub fn set_overflow_mode_wrap(&mut self, wrap: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if wrap {
                v | Config::OVR_FLOW_M
            } else {
                v & !Config::OVR_FLOW_M
            }
        })
    }
    /// Set overflow mode.
    /// - `false`: FIFO stops accepting events when full
    /// - `true`: FIFO wraps, oldest events are overwritten
    ///
    /// Needs to be enabled for overflow interrupts to work.
    /// **Note:** See the TCA8418 datasheet errata section for overflow behavior issues.
    #[cfg(feature = "async")]
    pub async fn set_overflow_mode_wrap(&mut self, wrap: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if wrap {
                v | Config::OVR_FLOW_M
            } else {
                v & !Config::OVR_FLOW_M
            }
        })
        .await
    }

    /// Set the INT deassertion behavior.
    /// When enabled, clearing the interrupt causes a 50ms deassertion
    /// before re-asserting if events are still pending.
    #[cfg(not(feature = "async"))]
    pub fn set_int_retrigger(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::INT_CFG
            } else {
                v & !Config::INT_CFG
            }
        })
    }

    /// Set the INT deassertion behavior.
    /// When enabled, clearing the interrupt causes a 50ms deassertion
    /// before re-asserting if events are still pending.
    #[cfg(feature = "async")]
    pub async fn set_int_retrigger(&mut self, enable: bool) -> Result<(), Error<E>> {
        self.modify_register(Register::Cfg, |v| {
            if enable {
                v | Config::INT_CFG
            } else {
                v & !Config::INT_CFG
            }
        })
        .await
    }

    /// Write the full CFG register directly.
    #[cfg(not(feature = "async"))]
    pub fn set_config_raw(&mut self, value: u8) -> Result<(), Error<E>> {
        self.write_register(Register::Cfg, value)
    }

    /// Write the full CFG register directly.
    #[cfg(feature = "async")]
    pub async fn set_config_raw(&mut self, value: u8) -> Result<(), Error<E>> {
        self.write_register(Register::Cfg, value).await
    }

    /// Read the full CFG register directly
    #[cfg(not(feature = "async"))]
    pub fn read_config_raw(&mut self) -> Result<u8, Error<E>> {
        self.read_register(Register::Cfg)
    }

    /// Read the full CFG register directly
    #[cfg(feature = "async")]
    pub async fn read_config_raw(&mut self) -> Result<u8, Error<E>> {
        self.read_register(Register::Cfg).await
    }
    // Interrupt handling

    /// Read the interrupt status register.
    #[cfg(not(feature = "async"))]
    pub fn interrupt_status(&mut self) -> Result<InterruptFlags, Error<E>> {
        Ok(InterruptFlags(self.read_register(Register::IntStat)?))
    }

    /// Read the interrupt status register.
    #[cfg(feature = "async")]
    pub async fn interrupt_status(&mut self) -> Result<InterruptFlags, Error<E>> {
        Ok(InterruptFlags(self.read_register(Register::IntStat).await?))
    }

    /// Check if there is a pending key event interrupt
    #[cfg(not(feature = "async"))]
    pub fn has_pending_key_event(&mut self) -> Result<bool, Error<E>> {
        let status = self.interrupt_status()?;
        Ok(status.contains(InterruptFlags::K_INT))
    }

    /// Check if there is a pending key event interrupt
    #[cfg(feature = "async")]
    pub async fn has_pending_key_event(&mut self) -> Result<bool, Error<E>> {
        let status = self.interrupt_status().await?;
        Ok(status.contains(InterruptFlags::K_INT))
    }

    /// Clear specific interrupt flags by writing 1 to the corresponding bits.
    #[cfg(not(feature = "async"))]
    pub fn clear_interrupts(&mut self, interrupts: InterruptFlags) -> Result<(), Error<E>> {
        self.write_register(Register::IntStat, interrupts.bits())
    }

    /// Clear specific interrupt flags by writing 1 to the corresponding bits.
    #[cfg(feature = "async")]
    pub async fn clear_interrupts(&mut self, interrupts: InterruptFlags) -> Result<(), Error<E>> {
        self.write_register(Register::IntStat, interrupts.bits())
            .await
    }

    /// Clear all interrupt flags.
    #[cfg(not(feature = "async"))]
    pub fn clear_all_interrupts(&mut self) -> Result<(), Error<E>> {
        self.clear_interrupts(InterruptFlags::ALL)
    }

    /// Clear all interrupt flags.
    #[cfg(feature = "async")]
    pub async fn clear_all_interrupts(&mut self) -> Result<(), Error<E>> {
        self.clear_interrupts(InterruptFlags::ALL).await
    }

    // GPIO Configuration

    /// Set GPIO direction according to the provided pin mask.
    /// Bit = 0: input, Bit = 1: output.
    /// Only affects pins NOT configured as keypad pins.
    #[cfg(not(feature = "async"))]
    pub fn set_gpio_direction(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioDir1,
            Register::GpioDir2,
            Register::GpioDir3,
            pins,
        )
    }

    /// Set GPIO direction according to the provided pin mask.
    /// Bit = 0: input, Bit = 1: output.
    /// Only affects pins NOT configured as keypad pins.
    #[cfg(feature = "async")]
    pub async fn set_gpio_direction(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioDir1,
            Register::GpioDir2,
            Register::GpioDir3,
            pins,
        )
        .await
    }

    /// Write GPIO output values according to the provided pin mask.
    /// Bit = 0: output low, Bit = 1: output high.
    /// Only affects pins configured as GPIO output.
    ///
    /// ```rust,no_run
    /// # use tca8418::{Tca8418, PinMask};
    /// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
    /// // Only set C9 high, every other pin will be set low
    /// keypad.write_gpio(PinMask::C9)?;
    ///
    /// // Set C3 high, everything else stays as-is
    /// let current = keypad.read_gpio()?;
    /// keypad.write_gpio(current.with(PinMask::C3))?;
    ///
    /// // Set C3 low
    /// keypad.write_gpio(current.without(PinMask::C3))?;
    ///
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(feature = "async"))]
    pub fn write_gpio(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioDatOut1,
            Register::GpioDatOut2,
            Register::GpioDatOut3,
            pins,
        )
    }

    /// Write GPIO output values according to the provided pin mask.
    /// Bit = 0: output low, Bit = 1: output high.
    /// Only affects pins configured as GPIO output.
    ///
    /// ```rust,no_run
    /// # use tca8418::{Tca8418, PinMask};
    /// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
    /// // Only set C9 high, every other pin will be set low
    /// keypad.write_gpio(PinMask::C9)?;
    ///
    /// // Set C3 high, everything else stays as-is
    /// let current = keypad.read_gpio()?;
    /// keypad.write_gpio(current.with(PinMask::C3))?;
    ///
    /// // Set C3 low
    /// keypad.write_gpio(current.without(PinMask::C3))?;
    ///
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "async")]
    pub async fn write_gpio(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioDatOut1,
            Register::GpioDatOut2,
            Register::GpioDatOut3,
            pins,
        )
        .await
    }

    /// Read GPIO data status for all pins, returning a `PinMask` with the current states of all row and column pins.
    /// If debouncing is enabled, these registers return their default values until a change of state occurs at an input.
    /// Initial pin states can be read by disabling debouncing.
    ///
    /// ```rust,no_run
    /// # use tca8418::{Tca8418, PinMask};
    /// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
    /// let gpio_status = keypad.read_gpio()?;
    /// if gpio_status.contains(PinMask::R0) {
    ///     // R0 is high
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(feature = "async"))]
    pub async fn read_gpio(&mut self) -> Result<PinMask, Error<E>> {
        self.read_multiple_registers(
            Register::GpioDatStat1,
            Register::GpioDatStat2,
            Register::GpioDatStat3,
        )
    }

    /// Read GPIO data status for all pins, returning a `PinMask` with the current states of all row and column pins.
    /// If debouncing is enabled, these registers return their default values until a change of state occurs at an input.
    /// Initial pin states can be read by disabling debouncing.
    ///
    /// ```rust,no_run
    /// # use tca8418::{Tca8418, PinMask};
    /// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
    /// let gpio_status = keypad.read_gpio()?;
    /// if gpio_status.contains(PinMask::R0) {
    ///     // R0 is high
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "async")]
    pub async fn read_gpio(&mut self) -> Result<PinMask, Error<E>> {
        self.read_multiple_registers(
            Register::GpioDatStat1,
            Register::GpioDatStat2,
            Register::GpioDatStat3,
        )
        .await
    }
    // Pull-up resistor control

    /// Disable internal pull-up resistors according to the provided pin mask.
    /// Bit = 0 (default) means pull-up enabled for that pin, Bit = 1 means pull-up disabled.
    #[cfg(not(feature = "async"))]
    pub fn disable_pullups(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioPullDis1,
            Register::GpioPullDis2,
            Register::GpioPullDis3,
            pins,
        )
    }

    /// Disable internal pull-up resistors according to the provided pin mask.
    /// Bit = 0 (default) means pull-up enabled for that pin, Bit = 1 means pull-up disabled.
    #[cfg(feature = "async")]
    pub async fn disable_pullups(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioPullDis1,
            Register::GpioPullDis2,
            Register::GpioPullDis3,
            pins,
        )
        .await
    }
    // Debounce control

    /// Disable debounce according to the provided pin mask.
    /// Bit = 1: debounce disabled for that pin.
    #[cfg(not(feature = "async"))]
    pub fn disable_debounce(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::DebounceDis1,
            Register::DebounceDis2,
            Register::DebounceDis3,
            pins,
        )
    }

    /// Disable debounce according to the provided pin mask.
    /// Bit = 1: debounce disabled for that pin.
    #[cfg(feature = "async")]
    pub async fn disable_debounce(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::DebounceDis1,
            Register::DebounceDis2,
            Register::DebounceDis3,
            pins,
        )
        .await
    }
    // GPI Event mode

    /// Set GPI event mode according to the provided pin mask.
    /// This only applies to pins configured as GPIO pins.
    /// Bit = 0: events tracked in status register (default), Bit = 1: events go into FIFO.
    /// Must be set to 0 to be able to use GPI pins for a unlock sequence in locked mode.
    #[cfg(not(feature = "async"))]
    pub fn set_gpi_event_mode(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(Register::GpiEm1, Register::GpiEm2, Register::GpiEm3, pins)
    }

    /// Set GPI event mode according to the provided pin mask.
    /// This only applies to pins configured as GPIO pins.
    /// Bit = 0: events tracked in status register (default), Bit = 1: events go into FIFO.
    /// Must be set to 0 to be able to use GPI pins for a unlock sequence in locked mode.
    #[cfg(feature = "async")]
    pub async fn set_gpi_event_mode(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(Register::GpiEm1, Register::GpiEm2, Register::GpiEm3, pins)
            .await
    }

    /// Enable GPIO interrupts according to the provided pin mask.
    /// Bit = 0: no interrupt generated, Bit = 1: interrupt generated on event.
    #[cfg(not(feature = "async"))]
    pub fn enable_gpio_interrupt(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioIntEn1,
            Register::GpioIntEn2,
            Register::GpioIntEn3,
            pins,
        )
    }

    /// Enable GPIO interrupts according to the provided pin mask.
    /// Bit = 0: no interrupt generated, Bit = 1: interrupt generated on event.
    #[cfg(feature = "async")]
    pub async fn enable_gpio_interrupt(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioIntEn1,
            Register::GpioIntEn2,
            Register::GpioIntEn3,
            pins,
        )
        .await
    }

    /// Set GPIO interrupt detection level according to the provided pin mask.
    /// This only affects pins configured as GPIO inputs with interrupts enabled.
    /// Bit = 0: low level / falling edge triggers, Bit = 1: high level / rising edge.
    #[cfg(not(feature = "async"))]
    pub fn set_gpio_int_level(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioIntLvl1,
            Register::GpioIntLvl2,
            Register::GpioIntLvl3,
            pins,
        )
    }

    /// Set GPIO interrupt detection level according to the provided pin mask.
    /// This only affects pins configured as GPIO inputs with interrupts enabled.
    /// Bit = 0: low level / falling edge triggers, Bit = 1: high level / rising edge.
    #[cfg(feature = "async")]
    pub async fn set_gpio_int_level(&mut self, pins: PinMask) -> Result<(), Error<E>> {
        self.write_multiple_registers(
            Register::GpioIntLvl1,
            Register::GpioIntLvl2,
            Register::GpioIntLvl3,
            pins,
        )
        .await
    }
    // Keypad lock

    /// Check if the keypad is currently locked.
    #[cfg(not(feature = "async"))]
    pub fn is_locked(&mut self) -> Result<bool, Error<E>> {
        let val = self.read_register(Register::KeyLckEc)?;
        Ok(val & 0x40 != 0)
    }

    /// Check if the keypad is currently locked.
    #[cfg(feature = "async")]
    pub async fn is_locked(&mut self) -> Result<bool, Error<E>> {
        let val = self.read_register(Register::KeyLckEc).await?;
        Ok(val & 0x40 != 0)
    }

    /// Set the keypad interrupt mask timer (0–31 seconds). 0 disables.
    /// This controls how often interrupts are generated while the keypad is locked.
    #[cfg(not(feature = "async"))]
    pub fn set_interrupt_mask_timer(&mut self, seconds: u8) -> Result<(), Error<E>> {
        let clamped = seconds.min(31);
        self.modify_register(Register::KpLckTimer, |v| (v & 0x07) | (clamped << 3))
    }

    /// Set the keypad interrupt mask timer (0–31 seconds). 0 disables.
    /// This controls how often interrupts are generated while the keypad is locked.
    #[cfg(feature = "async")]
    pub async fn set_interrupt_mask_timer(&mut self, seconds: u8) -> Result<(), Error<E>> {
        let clamped = seconds.min(31);
        self.modify_register(Register::KpLckTimer, |v| (v & 0x07) | (clamped << 3))
            .await
    }

    /// Set the unlock key sequence timer (0–7 seconds).
    /// This is the maximum time allowed between pressing unlock key 1 and unlock key 2.
    #[cfg(not(feature = "async"))]
    pub fn set_unlock_timer(&mut self, seconds: u8) -> Result<(), Error<E>> {
        let clamped = seconds.min(7);
        self.modify_register(Register::KpLckTimer, |v| (v & 0xF8) | clamped)
    }

    /// Set the unlock key sequence timer (0–7 seconds).
    /// This is the maximum time allowed between pressing unlock key 1 and unlock key 2.
    #[cfg(feature = "async")]
    pub async fn set_unlock_timer(&mut self, seconds: u8) -> Result<(), Error<E>> {
        let clamped = seconds.min(7);
        self.modify_register(Register::KpLckTimer, |v| (v & 0xF8) | clamped)
            .await
    }

    /// Set the unlock key combination (two key numbers).
    ///
    /// ```rust,no_run
    /// use tca8418::{Key,Tca8418};
    /// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
    ///
    /// // Set the keys at positions 0,1 and 3,3 in the keypad matrix as the unlock sequence
    /// keypad.set_unlock_keys(Key::from_row_col(0, 1).unwrap(), Key::from_row_col(3, 3).unwrap()).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(feature = "async"))]
    pub fn set_unlock_keys(&mut self, key1: Key, key2: Key) -> Result<(), Error<E>> {
        self.write_register(Register::Unlock1, key1.get_key_number() & 0x7F)?;
        self.write_register(Register::Unlock2, key2.get_key_number() & 0x7F)?;
        Ok(())
    }

    /// Set the unlock key combination (two key numbers).
    ///
    /// ```rust,no_run
    /// use tca8418::{Key,Tca8418};
    /// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
    ///
    /// // Set the keys at positions 0,1 and 3,3 in the keypad matrix as the unlock sequence
    /// keypad.set_unlock_keys(Key::from_row_col(0, 1).unwrap(), Key::from_row_col(3, 3).unwrap()).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "async")]
    pub async fn set_unlock_keys(&mut self, key1: Key, key2: Key) -> Result<(), Error<E>> {
        self.write_register(Register::Unlock1, key1.get_key_number() & 0x7F)
            .await?;
        self.write_register(Register::Unlock2, key2.get_key_number() & 0x7F)
            .await?;
        Ok(())
    }

    /// Lock the keypad, preventing key event interrupts and FIFO recording.
    /// Configure unlock keys and timers before calling this.
    #[cfg(not(feature = "async"))]
    pub fn lock(&mut self) -> Result<(), Error<E>> {
        self.modify_register(Register::KeyLckEc, |v| v | 0x40)
    }

    /// Lock the keypad, preventing key event interrupts and FIFO recording.
    /// Configure unlock keys and timers before calling this.
    #[cfg(feature = "async")]
    pub async fn lock(&mut self) -> Result<(), Error<E>> {
        self.modify_register(Register::KeyLckEc, |v| v | 0x40).await
    }

    /// Manually unlock the keypad.
    #[cfg(not(feature = "async"))]
    pub fn unlock(&mut self) -> Result<(), Error<E>> {
        self.modify_register(Register::KeyLckEc, |v| v & !0x40)
    }

    /// Manually unlock the keypad.
    #[cfg(feature = "async")]
    pub async fn unlock(&mut self) -> Result<(), Error<E>> {
        self.modify_register(Register::KeyLckEc, |v| v & !0x40)
            .await
    }
}
