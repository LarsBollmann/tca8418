//! This module includes types that define the different supported key types and events of the TCA8418.

/// This struct represents a key in the keypad matrix.
///
/// The row and column indices refer to the physical row and column pins on the TCA8418.
/// The key_number field is the raw key number as reported by the TCA8418 (1-80)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeypadMatrixKey {
    /// Row index (0–7)
    pub row: u8,
    /// Column index (0–9)
    pub col: u8,
}

impl KeypadMatrixKey {
    /// Create a `KeyboardKey` from a raw key number (1–80).
    /// Key number maps to matrix position: `key = row * 10 + col + 1`
    pub(crate) fn from_key_number(key_number: u8) -> Option<Self> {
        if key_number == 0 || key_number > 80 {
            return None;
        }
        let k = key_number - 1;
        let row = k / 10;
        let col = k % 10;
        Some(Self { row, col })
    }

    /// Create a key representing an element in the keypad matrix from row and col index
    pub fn from_row_col(row: u8, col: u8) -> Option<Self> {
        if row < 8 && col < 10 {
            Some(Self { row, col })
        } else {
            None
        }
    }

    /// Return the raw key number associated with this key
    pub fn get_key_number(&self) -> u8 {
        self.row * 10 + self.col + 1
    }
}

/// This struct represents an input on a row or col pin configured as an GPI (not part of the keypad matrix).
///
/// The index field represents which pin triggered the event. For row pins, index 0-7 corresponds to R0-R7. For column pins, index 0-9 corresponds to C0-C9.
/// This type is wrapped in the `Key::RowGpi` and `Key::ColGpi` variants to represent GPIO events on row and column pins respectively.
/// The key_number field is the raw key number as reported by the TCA8418, which is 97-104 for row GPIO pins and 105-114 for column GPIO pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GpiKey {
    /// Pin index (0–7 for rows, 0–9 for columns)
    pub index: u8,
}

/// This enum represents any key that can generate an event in the TCA8418 FIFO.
///
/// The TCA8418 distinguishes between keys configured as part of the keypad matrix and GPIO pins configured as inputs (GPI).
/// - `Key::KeypadMatrixKey` represents a key in the keypad matrix, identified by its row and column indices.
/// - `Key::RowGpi` represents a row GPIO pin configured as an input, identified by its row index (0-7).
/// - `Key::ColGpi` represents a column GPIO pin configured as an input, identified by its column index (0-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Key {
    /// A key in the keypad matrix
    KeypadMatrix(KeypadMatrixKey),
    /// A Row GPIO pin configured as an input (not part of the keypad matrix)
    RowGpi(GpiKey),
    /// A Column GPIO pin configured as an input (not part of the keypad matrix)
    ColGpi(GpiKey),
}

impl Key {
    /// Create a `Key` from a raw key number (1–80 for keypad matrix, 97–104 for row GPIO, 105–114 for column GPIO).
    pub fn from_key_number(key_number: u8) -> Option<Self> {
        if (1..=80).contains(&key_number) {
            Some(Key::KeypadMatrix(KeypadMatrixKey::from_key_number(
                key_number,
            )?))
        } else if (97..=104).contains(&key_number) {
            Some(Key::RowGpi(GpiKey {
                index: key_number - 97,
            }))
        } else if (105..=114).contains(&key_number) {
            Some(Key::ColGpi(GpiKey {
                index: key_number - 105,
            }))
        } else {
            None
        }
    }

    /// Create a key representing an element in the keypad matrix from row and col index
    pub fn from_row_col(row: u8, col: u8) -> Option<Self> {
        Some(Key::KeypadMatrix(KeypadMatrixKey::from_row_col(row, col)?))
    }

    /// Create a row gpi key from row index (0-7)
    pub fn row_gpi(row: u8) -> Option<Self> {
        if row > 7 {
            return None;
        }
        Some(Key::RowGpi(GpiKey { index: row }))
    }

    /// Create a column gpi key from column index (0-9)
    pub fn col_gpi(col: u8) -> Option<Self> {
        if col > 9 {
            return None;
        }
        Some(Key::ColGpi(GpiKey { index: col }))
    }

    /// Return the raw key number (1–80 for keypad matrix, 97–104 for row GPIO, 105–114 for column GPIO) associated with this key.
    pub fn get_key_number(&self) -> u8 {
        match self {
            Key::KeypadMatrix(k) => k.get_key_number(),
            Key::RowGpi(g) => g.index + 97,
            Key::ColGpi(g) => g.index + 105,
        }
    }
}

// ============================================================================
// Key Event
// ============================================================================

/// This type represents a single key event read from the TCA8418's key event FIFO.
///
/// The `key_number` field is the raw key number as reported by the TCA8418.
/// The `pressed` field indicates whether the event is a key press (`true`) or a key release (`false`).
/// The `key` field can be used to get the [Key] enum variant, which distinguishes between keypad matrix keys and GPIO events.
///
/// As convenience methods, [`pressed_keypad()`](KeyEvent::pressed_keypad) and [`released_keypad()`](KeyEvent::released_keypad)
/// return `Some(`[`KeypadMatrixKey`]`)` if the event corresponds to a keypad matrix key press or release respectively,
/// and `None` otherwise (e.g. for GPIO events).
///
/// # Examples
///
/// ```rust,no_run
/// # use tca8418::{Tca8418, PinMask, InterruptFlags, Key};
/// # fn run<E: core::fmt::Debug>(keypad: &mut Tca8418<impl embedded_hal::i2c::I2c<Error = E>>) -> Result<(), tca8418::Error<E>> {
/// // Handle only keypad presses, most common case
/// for event in keypad.events()? {
///     if let Some(key) = event.pressed_keypad() {
///         let _ = (key.row, key.col);
///     }
/// }
///
/// // Handle different events separately
/// for event in keypad.events()? {
///     // Is event a press or release?
///     let pressed = event.pressed;
///     
///     match event.key {
///         // Event is from a key in the configured keypad matrix
///         Key::KeypadMatrix(k) => { /* k.row, k.col */}
///         // Event is from a row pin configured as GPI
///         Key::RowGpi(g)       => { /* g.index */ }
///         // Event is from a column pin configured as GPI
///         Key::ColGpi(g)       => { /* g.index */}
///     }
/// }
/// # Ok(())
/// # }
///
/// ```
///
/// If you want to use your own event type instead, you can create a struct that implements `core::convert::From<KeyEvent>` or  `core::convert::TryFrom<KeyEvent>`
/// and map the [KeyEvent] to your own type.
///
/// # Key Number Ranges
///
/// | Range | Type |
/// |---------|------|
/// | 1–80 | Keypad matrix key (row × 10 + col + 1) |
/// | 81–96 | Reserved (unused) |
/// | 97–104 | Row GPI event (R0–R7) |
/// | 105–114 | Column GPI event (C0–C9) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeyEvent {
    /// Key number as reported by the TCA8418
    pub key_number: u8,
    /// Parsed Key
    pub key: Key,
    /// `true` if the key was pressed, `false` if released
    pub pressed: bool,
}

impl KeyEvent {
    /// Parse a raw event byte from the KEY_EVENT register.
    ///
    /// Bit 7 = press (1) / release (0), bits 6:0 = key number (1–80 for keypad events, 97–114 for GPI events)
    /// Key number maps to matrix position: `key = row * 10 + col + 1`
    pub(crate) fn from_raw(raw: u8) -> Option<Self> {
        let key_number = raw & 0x7F;
        if key_number == 0 {
            return None; // FIFO is empty
        }
        let pressed = (raw & 0x80) != 0;
        let key = Key::from_key_number(key_number)?;
        Some(KeyEvent {
            key_number,
            key,
            pressed,
        })
    }

    /// Returns Some([KeypadMatrixKey]) if the event was a key press and the pressed key is part of the keypad matrix
    pub fn pressed_keypad(&self) -> Option<KeypadMatrixKey> {
        if self.pressed {
            KeypadMatrixKey::from_key_number(self.key_number)
        } else {
            None
        }
    }

    /// Returns Some([KeypadMatrixKey]) if the event was a key release and the pressed key is part of the keypad matrix
    pub fn released_keypad(&self) -> Option<KeypadMatrixKey> {
        if !self.pressed {
            KeypadMatrixKey::from_key_number(self.key_number)
        } else {
            None
        }
    }
}
