# tca8418
[![Crates.io](https://img.shields.io/crates/v/tca8418)](https://crates.io/crates/tca8418)
[![docs.rs](https://img.shields.io/docsrs/tca8418)](https://docs.rs/tca8418)
[![CI](https://github.com/larsbollmann/tca8418/actions/workflows/release.yml/badge.svg)](https://github.com/larsbollmann/tca8418/actions)
[![License](https://img.shields.io/crates/l/tca8418)](https://github.com/larsbollmann/tca8418/blob/main/LICENSE)

Platform-agnostic Rust driver for the [Texas Instruments TCA8418](https://www.ti.com/product/TCA8418) I²C keypad scan IC, built on [`embedded-hal`](https://docs.rs/embedded-hal) traits.
 
The TCA8418 supports up to 80 keys (8 rows × 10 columns), with a 10-event FIFO, configurable GPIO pins, hardware debounce, and interrupt output. It operates from 1.65V to 3.6V and communicates over I²C at up to 1 MHz.
 
- [Datasheet (PDF)](https://www.ti.com/lit/ds/symlink/tca8418.pdf)
- [API Documentation](https://docs.rs/tca8418)
 
## Features
 
- `no_std` compatible — works on any microcontroller
- Keypad matrix configuration with flexible row/column selection
- 10-event FIFO with iterator-based draining
- GPIO input/output on unused pins
- Interrupt status reading and clearing
- Keypad lock/unlock with configurable timers
- Pull-up resistor and debounce control per pin
 
## Quick Start
This is a simple example using active polling.
See the docs for more examples.
 
```rust,ignore
use tca8418::{Tca8418, PinMask, InterruptFlags};
 
// Create the driver
let mut keypad = Tca8418::new(i2c);
 
// Configure rows 0–6 and columns 0–8 as the keypad matrix
let pins = PinMask::rows(0b0111_1111) | PinMask::cols(0b0001_1111_1111);
keypad.configure_keypad(pins).unwrap();
 
// Poll for events
loop {
    for event in keypad.events().flatten() {
        if let Some(key) = event.pressed_keypad() {
            // key.row, key.col
        }
    }
}
```
