use std::collections::HashMap;
use speedy2d::window::VirtualKeyCode;

lazy_static::lazy_static! {
    /// Real Keypad:
    /// 1 2 3 C
    /// 4 5 6 D
    /// 7 8 9 E
    /// A 0 B F
    /// Emulated Keypad on computer keyboard:
    /// 1 2 3 4
    /// Q W E R
    /// A S D F
    /// ` Z X C
    pub static ref KEY_MAP: HashMap<VirtualKeyCode, usize> = {
        let mut map = HashMap::new();
        map.insert(VirtualKeyCode::Key1, 1);
        map.insert(VirtualKeyCode::Key2, 2);
        map.insert(VirtualKeyCode::Key3, 3);
        map.insert(VirtualKeyCode::Key4, 0xC);
        map.insert(VirtualKeyCode::Q, 4);
        map.insert(VirtualKeyCode::W, 5);
        map.insert(VirtualKeyCode::E, 6);
        map.insert(VirtualKeyCode::R, 0xD);
        map.insert(VirtualKeyCode::A, 7);
        map.insert(VirtualKeyCode::S, 8);
        map.insert(VirtualKeyCode::D, 9);
        map.insert(VirtualKeyCode::F, 0xE);
        map.insert(VirtualKeyCode::Grave, 0xA);
        map.insert(VirtualKeyCode::Z, 0);
        map.insert(VirtualKeyCode::X, 0xB);
        map.insert(VirtualKeyCode::C, 0xF);
        map
    };
}

#[derive(Clone)]
pub struct Keypad {
    keys: [bool; 16],
}

impl Keypad {
    pub fn new() -> Self {
        Keypad {
            keys: [false; 16],
        }
    }

    pub fn get_new_key_release(&self, new: &Keypad) -> Option<usize> {
        for i in 0..self.keys.len() {
            if self.keys[i] && !new.keys[i] {
                return Some(i);
            }
        }
        None
    }

    pub fn key_down(&mut self, id: usize) {
        self.keys[id] = true
    }

    pub fn key_up(&mut self, id: usize) {
        self.keys[id] = false
    }

    pub fn is_key_pressed(&self, id: usize) -> bool {
        self.keys[id]
    }

}
