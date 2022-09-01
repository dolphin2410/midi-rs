#![allow(dead_code)]
use std::error::Error;

#[derive(Debug, Clone, Copy)]
pub enum Notes {
    C = 12,
    CSharp = 13,
    D = 14,
    DSharp = 15,
    E = 16,
    F = 17,
    FSharp = 18,
    G = 19,
    GSharp = 20,
    A = 21,
    ASharp = 22,
    B = 23,
}

impl Notes {
    pub fn octave(self, n: u32) -> Result<u32, Box<dyn Error>> {
        let octaved = self as u32 + 12 * n;
        if octaved < 21 {
            Err("Invalid Octave. Can't go lower than 21 (A0)".into())
        } else {
            Ok(octaved)
        }
    }

    pub fn from(n: u32) -> Option<(Self, u8)> {
        let modulo = n % 12;
        let octave_raw = ((n - modulo) / 12 - 1) as i8;
        if octave_raw < 0 {
            return None;
        }
        let octave = octave_raw as u8;
        match modulo {
            0 => Some((Self::C, octave)),
            1 => Some((Self::CSharp, octave)),
            2 => Some((Self::D, octave)),
            3 => Some((Self::DSharp, octave)),
            4 => Some((Self::E, octave)),
            5 => Some((Self::F, octave)),
            6 => Some((Self::FSharp, octave)),
            7 => Some((Self::G, octave)),
            8 => Some((Self::GSharp, octave)),
            9 => Some((Self::A, octave)),
            10 => Some((Self::ASharp, octave)),
            11 => Some((Self::B, octave)),
            _ => None,
        }
    }
}
