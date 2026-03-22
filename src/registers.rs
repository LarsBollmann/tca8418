// ============================================================================
// Register Addresses and Bit Masks for the TCA8418
// ============================================================================

//! This module contains register adress and register bitmask definitions. Intended for internal use.

/// Register map for the TCA8418
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Register {
    /// Reserved
    Reserved = 0x00,
    /// Configuration register
    Cfg = 0x01,
    /// Interrupt status register
    IntStat = 0x02,
    /// Key lock and event counter register
    KeyLckEc = 0x03,
    /// Key event register A (FIFO read)
    KeyEventA = 0x04,
    /// Key event register B (FIFO read)
    KeyEventB = 0x05,
    /// Key event register C (FIFO read)
    KeyEventC = 0x06,
    /// Key event register D (FIFO read)
    KeyEventD = 0x07,
    /// Key event register E (FIFO read)
    KeyEventE = 0x08,
    /// Key event register F (FIFO read)
    KeyEventF = 0x09,
    /// Key event register G (FIFO read)
    KeyEventG = 0x0A,
    /// Key event register H (FIFO read)
    KeyEventH = 0x0B,
    /// Key event register I (FIFO read)
    KeyEventI = 0x0C,
    /// Key event register J (FIFO read)
    KeyEventJ = 0x0D,
    /// Key lock/timer register
    KpLckTimer = 0x0E,
    /// Unlock key 1
    Unlock1 = 0x0F,
    /// Unlock key 2
    Unlock2 = 0x10,
    /// GPIO interrupt status 1 (ROW0-ROW7)
    GpioIntStat1 = 0x11,
    /// GPIO interrupt status 2 (COL0-COL7)
    GpioIntStat2 = 0x12,
    /// GPIO interrupt status 3 (COL8-COL9)
    GpioIntStat3 = 0x13,
    /// GPIO data status 1 / GPI event mode 1 (ROW0-ROW7)
    GpioDatStat1 = 0x14,
    /// GPIO data status 2 / GPI event mode 2 (COL0-COL7)
    GpioDatStat2 = 0x15,
    /// GPIO data status 3 / GPI event mode 3 (COL8-COL9)
    GpioDatStat3 = 0x16,
    /// GPIO data output 1 (ROW0-ROW7)
    GpioDatOut1 = 0x17,
    /// GPIO data output 2 (COL0-COL7)
    GpioDatOut2 = 0x18,
    /// GPIO data output 3 (COL8-COL9)
    GpioDatOut3 = 0x19,
    /// GPIO interrupt enable 1 (ROW0-ROW7)
    GpioIntEn1 = 0x1A,
    /// GPIO interrupt enable 2 (COL0-COL7)
    GpioIntEn2 = 0x1B,
    /// GPIO interrupt enable 3 (COL8-COL9)
    GpioIntEn3 = 0x1C,
    /// Keypad/GPIO selection 1 (ROW0-ROW7)
    KpGpio1 = 0x1D,
    /// Keypad/GPIO selection 2 (COL0-COL7)
    KpGpio2 = 0x1E,
    /// Keypad/GPIO selection 3 (COL8-COL9)
    KpGpio3 = 0x1F,
    /// GPI event mode 1 (ROW0-ROW7)
    GpiEm1 = 0x20,
    /// GPI event mode 2 (COL0-COL7)
    GpiEm2 = 0x21,
    /// GPI event mode 3 (COL8-COL9)
    GpiEm3 = 0x22,
    /// GPIO direction 1 (ROW0-ROW7)
    GpioDir1 = 0x23,
    /// GPIO direction 2 (COL0-COL7)
    GpioDir2 = 0x24,
    /// GPIO direction 3 (COL8-COL9)
    GpioDir3 = 0x25,
    /// GPIO interrupt level 1 (ROW0-ROW7)
    GpioIntLvl1 = 0x26,
    /// GPIO interrupt level 2 (COL0-COL7)
    GpioIntLvl2 = 0x27,
    /// GPIO interrupt level 3 (COL8-COL9)
    GpioIntLvl3 = 0x28,
    /// Debounce disable 1 (ROW0-ROW7)
    DebounceDis1 = 0x29,
    /// Debounce disable 2 (COL0-COL7)
    DebounceDis2 = 0x2A,
    /// Debounce disable 3 (COL8-COL9)
    DebounceDis3 = 0x2B,
    /// GPIO pull-up disable 1 (ROW0-ROW7)
    GpioPullDis1 = 0x2C,
    /// GPIO pull-up disable 2 (COL0-COL7)
    GpioPullDis2 = 0x2D,
    /// GPIO pull-up disable 3 (COL8-COL9)
    GpioPullDis3 = 0x2E,
}

// ============================================================================
// Configuration Register Bits (0x01)
// ============================================================================

/// Configuration register bit masks
pub struct Config;

impl Config {
    /// Key interrupt enable (key events generate INT)
    pub const KE_IEN: u8 = 1 << 0;
    /// GPI interrupt enable (GPI changes generate INT)
    pub const GPI_IEN: u8 = 1 << 1;
    /// Key lock interrupt enable
    pub const K_LCK_IEN: u8 = 1 << 2;
    /// Overflow interrupt enable
    pub const OVR_FLOW_IEN: u8 = 1 << 3;
    /// INT deassertion configuration (50ms deassertion when set)
    pub const INT_CFG: u8 = 1 << 4;
    /// Overflow mode (0 = FIFO full stops, 1 = FIFO wraps)
    pub const OVR_FLOW_M: u8 = 1 << 5;
    /// GPI event mode (0 = direct from register, 1 = FIFO)
    pub const GPI_E_CFG: u8 = 1 << 6;
    /// Auto-increment for key events
    pub const AI: u8 = 1 << 7;
}

/// A struct that represents the bits of the interrupt status register (INT_STAT).
///
/// Create new ones using the constants. Supports bitwise OR, AND and NOT operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptFlags(pub(crate) u8);

impl InterruptFlags {
    /// A key event (press or release) is pending in the FIFO.
    pub const K_INT: Self = Self(1 << 0);
    /// A GPI (general purpose input) event has occurred.
    /// Only asserted for pins configured as GPI with interrupts enabled.    
    pub const GPI_INT: Self = Self(1 << 1);
    /// The keypad has been successfully unlocked via the unlock key sequence.
    pub const K_LCK_INT: Self = Self(1 << 2);
    /// The key event FIFO has overflowed.
    pub const OVR_FLOW_INT: Self = Self(1 << 3);
    /// Ctrl + ALT + Delete (Keys 1, 11 and 21).
    ///
    /// **Note:** See the TCA8418 datasheet errata section — the CAD
    /// interrupt may behave unexpectedly in some conditions.
    pub const CAD_INT: Self = Self(1 << 4);
    /// All interrupt flags
    pub const ALL: Self = Self(0x1F);
    /// No interrupt flags
    pub const EMPTY: Self = Self(0);

    /// Returns the raw `u8` value of the flags.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns `true` if all flags in `other` are set in `self`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns true if no Flag is set
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl core::ops::BitOr for InterruptFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for InterruptFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::Not for InterruptFlags {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0 & 0x1F)
    }
}
