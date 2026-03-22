/// This is a helper type for a single u32 bitmask representing a combination of row and column pins.
///
/// Bits 0-7 correspond to rows R0-R7, bits 8-15 correspond to columns C0-C7, and bits 16-17 correspond to columns C8-C9.
/// You can create one using either the individual pin constants (e.g. `Pins::R0 | Pins::R1 | Pins::C0`) or the helper functions (e.g. `Pins::rows(0x02) | Pins::cols(0x01)`).
///
/// A lot of the TCA8418's configuration are per row/column pins, so this struct makes it easier to work with those configurations without having to manage separate row and column bitmasks.
/// If you only ever want to use one combination of row and column pins you can create a constant once and reuse it throughout your code.
///
/// ## Examples
/// ```rust
/// # use tca8418::PinMask;
/// // Lets say you want to configure a bitmask where rows R0-R4 and columns C0-C3 are set. You could do this using the helper functions:
/// let mask = PinMask::rows(0b0001_1111) | PinMask::cols(0b0000_0000_0000_1111);
///
///
/// // Or you could do the same thing using the individual pin constants:
/// let mask = PinMask::R0 | PinMask::R1 | PinMask::R2 | PinMask::R3 | PinMask::R4 | PinMask::C0 | PinMask::C1 | PinMask::C2 | PinMask::C3;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinMask(pub u32);

impl PinMask {
    /// Create a Pins bitmask from a u8 defining the rows
    /// - `mask`: bit 0 = R0, bit 1 = R1, ..., bit 7 = R7
    /// ```rust
    /// # use tca8418::PinMask;
    /// // Create a Pins bitmask for rows R0, R2, and R4:
    /// let mask = PinMask::rows(0b0001_0101);
    /// ```
    pub const fn rows(mask: u8) -> Self {
        Self(mask as u32)
    }

    /// Create a Pins bitmask from a u16 defining the columns
    /// - `mask`: bit 0 = C0, bit 1 = C1, ..., bit 9 = C9 (only uses lower 10 bits)
    /// ```rust
    /// # use tca8418::PinMask;
    /// // Create a Pins bitmask for columns C2, C4 and C9:
    /// let mask = PinMask::cols(0b0000_0010_0001_0100);
    /// ```
    pub const fn cols(mask: u16) -> Self {
        Self((mask as u32) << 8)
    }

    pub(crate) fn cols_low(mask: u8) -> Self {
        Self((mask as u32) << 8)
    }

    pub(crate) fn cols_high(mask: u8) -> Self {
        Self((mask as u32) << 16)
    }

    /// Sets all bits in other in this mask
    pub const fn with(self, other: PinMask) -> Self {
        Self(self.0 | other.0)
    }

    /// Clears all bits in other in this mask
    pub const fn without(self, other: PinMask) -> Self {
        Self(self.0 & !other.0)
    }

    /// Check if all bits in `other` are set in this mask.
    pub const fn contains(&self, other: PinMask) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if any bit in `other` is set in this mask.
    pub const fn intersects(&self, other: PinMask) -> bool {
        (self.0 & other.0) != 0
    }

    /// Check if no bits are set.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Return raw bits
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Return the part of the bitmask that represents the rows
    pub const fn row_bits(&self) -> u8 {
        self.0 as u8
    }

    /// Return the part of the bitmask that represents the columns
    pub const fn col_bits(&self) -> u16 {
        (self.0 >> 8) as u16 & 0x03FF
    }

    pub(crate) fn col_low_bits(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub(crate) fn col_high_bits(&self) -> u8 {
        (self.0 >> 16) as u8 & 0x03
    }

    /// A mask with all rows and columns set
    pub const ALL: Self = Self(0x0003_FFFF);
    /// A mask with no rows or columns set
    pub const NONE: Self = Self(0);
    /// A mask with all rows set
    pub const ALL_ROWS: Self = Self(0xFF);
    /// A mask with all columns set
    pub const ALL_COLS: Self = Self(0x03FF << 8);

    /// Rows
    /// ROW0 Pin
    pub const R0: Self = Self(1 << 0);
    /// ROW1 Pin
    pub const R1: Self = Self(1 << 1);
    /// ROW2 Pin
    pub const R2: Self = Self(1 << 2);
    /// ROW3 Pin
    pub const R3: Self = Self(1 << 3);
    /// ROW4 Pin
    pub const R4: Self = Self(1 << 4);
    /// ROW5 Pin
    pub const R5: Self = Self(1 << 5);
    /// ROW6 Pin
    pub const R6: Self = Self(1 << 6);
    /// ROW7 Pin
    pub const R7: Self = Self(1 << 7);

    /// Columns
    /// COL0 Pin
    pub const C0: Self = Self(1 << 8);
    /// COL1 Pin
    pub const C1: Self = Self(1 << 9);
    /// COL2 Pin
    pub const C2: Self = Self(1 << 10);
    /// COL3 Pin
    pub const C3: Self = Self(1 << 11);
    /// COL4 Pin
    pub const C4: Self = Self(1 << 12);
    /// COL5 Pin
    pub const C5: Self = Self(1 << 13);
    /// COL6 Pin
    pub const C6: Self = Self(1 << 14);
    /// COL7 Pin
    pub const C7: Self = Self(1 << 15);
    /// COL8 Pin
    pub const C8: Self = Self(1 << 16);
    /// COL9 Pin
    pub const C9: Self = Self(1 << 17);
}

// Allow combining: Pins::rows(0x7F) | Pins::cols(0xFF)
impl core::ops::BitOr for PinMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// Allow combining with assignment: pins |= Pins::R0 | Pins::C0
impl core::ops::BitOrAssign for PinMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for PinMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitAndAssign for PinMask {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl core::ops::Not for PinMask {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0 & 0x0003_FFFF) // Only keep bits 0-17
    }
}

impl core::ops::BitXor for PinMask {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        Self((self.0 ^ rhs.0) & 0x0003_FFFF)
    }
}
