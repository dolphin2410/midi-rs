use std::{
    error::Error,
    fs::{self, File},
    io::prelude::*,
};

use bytes::{Buf, BytesMut};

use crate::status::Status;

pub enum SysExMeta {
    MetaSequence = 0x00,
    MetaText = 0x01,
    MetaCopyright = 0x02,
    MetaTrackName = 0x03,
    MetaInstrumentName = 0x04,
    MetaLyrics = 0x05,
    MetaMarker = 0x06,
    MetaCuePoint = 0x07,
    MetaChannelPrefix = 0x20,
    MetaEndOfTrack = 0x2F,
    MetaSetTempo = 0x51,
    MetaSMPTEOffset = 0x54,
    MetaTimeSignature = 0x58,
    MetaKeySignature = 0x59,
    MetaSequencerSpecific = 0x7F,
}

#[derive(Debug)]
pub enum MetaData {
    SingleU8(u8),
    DoubleU8(u8, u8),
    TripleU8(u8, u8, u8),
    QuadU8(u8, u8, u8, u8),
    QuintripleU8(u8, u8, u8, u8, u8),
    SingleString(String),
    None,
}

impl SysExMeta {
    pub fn from(d: u8) -> Option<Self> {
        match d {
            0x00 => Some(Self::MetaSequence),
            0x01 => Some(Self::MetaText),
            0x02 => Some(Self::MetaCopyright),
            0x03 => Some(Self::MetaTrackName),
            0x04 => Some(Self::MetaInstrumentName),
            0x05 => Some(Self::MetaLyrics),
            0x06 => Some(Self::MetaMarker),
            0x07 => Some(Self::MetaCuePoint),
            0x20 => Some(Self::MetaChannelPrefix),
            0x2F => Some(Self::MetaEndOfTrack),
            0x51 => Some(Self::MetaSetTempo),
            0x54 => Some(Self::MetaSMPTEOffset),
            0x58 => Some(Self::MetaTimeSignature),
            0x59 => Some(Self::MetaKeySignature),
            0x7F => Some(Self::MetaSequencerSpecific),
            _ => None,
        }
    }
}

pub enum EventData {
    NoteOnOffData { key: u8, velocity: u8 },
    ControlData { control_id: u8, control_value: u8 },
    ProgramChangeData { program_id: u8 },
    ChannelData { channel_pressure: u8 },
    PitchBendData { least_bytes: u8, most_bytes: u8 },
    SysexData { meta: MetaData },
    Error(String),
}

pub struct MidiEvent {
    pub status: Status,
    pub data: EventData,
    pub delta_tick: u32,
}

pub struct MidiTrack {
    pub name: String,
    pub instrument: String,
    pub events: Vec<MidiEvent>,
    pub end_of_track: bool,
}

pub fn read_str(bytes: &mut BytesMut, length: usize) -> Box<String> {
    let slice = (0..length)
        .into_iter()
        .map(|_| bytes.get_u8())
        .collect::<Vec<u8>>();
    let s = String::from_utf8_lossy(slice.as_slice());
    Box::new(String::from(s))
}

pub fn read_value(bytes: &mut BytesMut) -> u32 {
    let mut n_value = bytes.get_u8() as u32;
    let mut n_byte;

    if n_value & 0x80 != 0 {
        n_value = n_value & 0x7F;
        loop {
            n_byte = bytes.get_u8();
            n_value = (n_value << 7) | (n_byte as u32 & 0x7F);
            if n_byte & 0x80 == 0 {
                break;
            }
        }
    }

    n_value
}

pub struct MidiFile {
    pub tempo: u32,
    pub bpm: u32,
    pub tracks: Vec<MidiTrack>,
    pub division: u16,
    pub prev_status: u8,
}

impl MidiFile {
    pub fn create() -> Self {
        Self {
            tempo: 0,
            bpm: 0,
            tracks: vec![],
            division: 0,
            prev_status: 0,
        }
    }
    pub fn parse(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(filename).unwrap();
        let metadata = fs::metadata(filename).unwrap();
        let mut bytes = BytesMut::with_capacity(metadata.len() as usize);
        unsafe {
            bytes.set_len(metadata.len() as usize);
        }
        file.read(&mut bytes).unwrap();

        let _file_id = bytes.get_u32();
        let _header_len = bytes.get_u32();
        let _format = bytes.get_u16();
        let track_chunks = bytes.get_u16();
        let division = bytes.get_u16();
        self.division = division;

        let mut tracks: Vec<MidiTrack> = vec![];
        for _chunk in 0..track_chunks {
            let _n_track_id = bytes.get_u32();
            let _n_track_len = bytes.get_u32();

            let mut track = MidiTrack {
                events: vec![],
                name: String::new(),
                instrument: String::new(),
                end_of_track: false,
            };

            self.prev_status = 0u8;
            while bytes.remaining() != 0 && !track.end_of_track {
                let delta_tick = read_value(&mut bytes);
                let mut status = bytes.get_u8();
                let split = bytes.clone();

                if status < 0x80 {
                    status = self.prev_status;
                    bytes = split;
                }

                let status = Status::from_byte(status)?;
                let data = status.parse_data(self, &mut track, &mut bytes);

                let event = MidiEvent {
                    status,
                    data,
                    delta_tick,
                };
                track.events.push(event);
            }

            tracks.push(track);
        }

        self.tracks = tracks;
        Ok(())
    }
}
