use super::{Key, Keyboard, Macro, MouseAction, MouseEvent, Accord};
use anyhow::{anyhow, bail, Result};
use log::debug;
use anyhow::ensure;
use rusb::{Context, DeviceHandle};

pub struct Keyboard8850 {
    handle: DeviceHandle<Context>,
    endpoint: u8,
}

impl Keyboard for Keyboard8850 {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        let mut msg = vec![
            0x03,
            0xfe,
            key.to_key_id_8850(12)?,
            layer + 1,
            expansion.kind(),
            0,
            0,
            0,
            0,
            0,
        ];

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 18, "macro sequence is too long");

                // Allow single key modifier to be used in combo with other key(s)
                if presses.len() == 1 && presses[0].code.is_none(){
                    msg.push(0);
                } else {
                    msg.push(presses.len() as u8);
                }

                for Accord { modifiers, code } in presses.iter() {
                    msg.extend_from_slice(&[modifiers.as_u8(), code.map_or(0, |c| c.value())]);
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                msg.extend_from_slice(&[0, low, high, 0, 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), _)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                msg.extend_from_slice(&[0x01, 0, buttons.as_u8()]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0x1]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0xff]);
            }
        };

        self.send(&msg)?;

        // Finish key binding
        self.send(&[0x03, 0xfd, 0xfe, 0xff])?;

        Ok(())
    }

    fn set_led(&mut self, _n: u8) -> Result<()> {
        bail!(
            "If you have a device which supports backlight LEDs, please let us know at \\
               https://github.com/kriomant/ch57x-keyboard-tool/issues/60. We'll be glad to \\
               help you reverse-engineer it."
        )
    }

    fn get_handle(&self) -> &DeviceHandle<Context> {
        &self.handle
    }

    fn get_endpoint(&self) -> u8 {
        self.endpoint
    }

    fn preferred_endpoint() -> u8 {
        0x04
    }
}

impl Keyboard8850 {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint };

        keyboard.send(&[])?;

        Ok(keyboard)
    }
}

impl Key {
    // 8850 specific key ID calculation
    fn to_key_id_8850(self, base: u8) -> Result<u8> {
        match self {
            Key::Button(n) if n >= base => Err(anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 4 => Err(anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(base + 1 + 3 * n + (action as u8)),
        }
    }
}