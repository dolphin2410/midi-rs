use std::error::Error;

use bytes::{Buf, BytesMut};

use crate::parser::{read_str, read_value, EventData, MetaData, MidiFile, MidiTrack, SysExMeta};

#[derive(PartialEq, Debug)]
pub enum StatusType {
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAftertouch = 0xa0,
    CtrlChange = 0xb0,
    ProgramChange = 0xc0,
    ChannelAftertouch = 0xd0,
    PitchBendChange = 0xe0,
    SystemMsg = 0xf0,
}

pub struct Status {
    pub status_type: StatusType,
    pub raw_status: u8,
}

impl Status {
    pub fn from_byte(byte: u8) -> Result<Self, Box<dyn Error>> {
        match byte & 0xf0 {
            0x80 => Ok(Self {
                status_type: StatusType::NoteOff,
                raw_status: byte,
            }),
            0x90 => Ok(Self {
                status_type: StatusType::NoteOn,
                raw_status: byte,
            }),
            0xa0 => Ok(Self {
                status_type: StatusType::PolyphonicAftertouch,
                raw_status: byte,
            }),
            0xb0 => Ok(Self {
                status_type: StatusType::CtrlChange,
                raw_status: byte,
            }),
            0xc0 => Ok(Self {
                status_type: StatusType::ProgramChange,
                raw_status: byte,
            }),
            0xd0 => Ok(Self {
                status_type: StatusType::ChannelAftertouch,
                raw_status: byte,
            }),
            0xe0 => Ok(Self {
                status_type: StatusType::PitchBendChange,
                raw_status: byte,
            }),
            0xf0 => Ok(Self {
                status_type: StatusType::SystemMsg,
                raw_status: byte,
            }),
            _ => Err(format!("Invalid Status Byte: {}", byte).into()),
        }
    }

    pub fn parse_data(
        &self,
        file: &mut MidiFile,
        track: &mut MidiTrack,
        bytes: &mut BytesMut,
    ) -> EventData {
        file.prev_status = self.raw_status;

        match self.status_type {
            StatusType::NoteOn | StatusType::NoteOff | StatusType::PolyphonicAftertouch => {
                let key = bytes.get_u8();
                let velocity = bytes.get_u8();
                EventData::NoteOnOffData { key, velocity }
            }
            StatusType::CtrlChange => {
                let control_id = bytes.get_u8();
                let control_value = bytes.get_u8();
                EventData::ControlData {
                    control_id,
                    control_value,
                }
            }
            StatusType::ProgramChange => {
                let program_id = bytes.get_u8();
                EventData::ProgramChangeData { program_id }
            }
            StatusType::ChannelAftertouch => {
                let channel_pressure = bytes.get_u8();
                EventData::ChannelData { channel_pressure }
            }
            StatusType::PitchBendChange => {
                let least_bytes = bytes.get_u8();
                let most_bytes = bytes.get_u8();
                EventData::PitchBendData {
                    least_bytes,
                    most_bytes,
                }
            }
            StatusType::SystemMsg => {
                file.prev_status = 0;
                if self.raw_status == 0xFF {
                    let ty = bytes.get_u8();
                    let len = read_value(bytes);
                    match SysExMeta::from(ty).unwrap() {
                        SysExMeta::MetaSequence => {
                            return EventData::SysexData {
                                meta: MetaData::DoubleU8(bytes.get_u8(), bytes.get_u8()),
                            }
                        }

                        SysExMeta::MetaChannelPrefix => {
                            return EventData::SysexData {
                                meta: MetaData::SingleU8(bytes.get_u8()),
                            }
                        }

                        SysExMeta::MetaLyrics
                        | SysExMeta::MetaCuePoint
                        | SysExMeta::MetaMarker
                        | SysExMeta::MetaSequencerSpecific
                        | SysExMeta::MetaCopyright
                        | SysExMeta::MetaText => {
                            return EventData::SysexData {
                                meta: MetaData::SingleString(*read_str(bytes, len as usize)),
                            }
                        }

                        SysExMeta::MetaTrackName => {
                            track.name = *read_str(bytes, len as usize);
                            return EventData::SysexData {
                                meta: MetaData::SingleString(track.name.clone()),
                            };
                        }

                        SysExMeta::MetaInstrumentName => {
                            track.instrument = *read_str(bytes, len as usize);
                            return EventData::SysexData {
                                meta: MetaData::SingleString(track.instrument.clone()),
                            };
                        }

                        SysExMeta::MetaEndOfTrack => {
                            track.end_of_track = true;
                            return EventData::SysexData {
                                meta: MetaData::None,
                            };
                        }

                        SysExMeta::MetaSetTempo => {
                            if file.tempo == 0 {
                                let first = bytes.get_u8();
                                let second = bytes.get_u8();
                                let third = bytes.get_u8();
                                file.tempo |= (first as u32) << 16;
                                file.tempo |= (second as u32) << 8;
                                file.tempo |= (third as u32) << 0;
                                file.bpm = 60000000 / file.tempo;
                                return EventData::SysexData {
                                    meta: MetaData::TripleU8(first, second, third),
                                };
                            }
                            return EventData::SysexData {
                                meta: MetaData::None,
                            };
                        }

                        SysExMeta::MetaSMPTEOffset => {
                            return EventData::SysexData {
                                meta: MetaData::QuintripleU8(
                                    bytes.get_u8(),
                                    bytes.get_u8(),
                                    bytes.get_u8(),
                                    bytes.get_u8(),
                                    bytes.get_u8(),
                                ),
                            };
                        }

                        SysExMeta::MetaTimeSignature => {
                            return EventData::SysexData {
                                meta: MetaData::QuadU8(
                                    bytes.get_u8(),
                                    2 << bytes.get_u8(),
                                    bytes.get_u8(),
                                    bytes.get_u8(),
                                ),
                            };
                        }

                        SysExMeta::MetaKeySignature => {
                            return EventData::SysexData {
                                meta: MetaData::DoubleU8(bytes.get_u8(), bytes.get_u8()),
                            };
                        }
                    }
                } else if self.raw_status == 0xF0 {
                    let len = read_value(bytes) as usize;
                    return EventData::SysexData {
                        meta: MetaData::SingleString(*read_str(bytes, len)),
                    };
                } else if self.raw_status == 0xf7 {
                    let len = read_value(bytes) as usize;
                    return EventData::SysexData {
                        meta: MetaData::SingleString(*read_str(bytes, len)),
                    };
                } else {
                    EventData::Error("Failed to parse data from system message".to_string())
                }
            }
        }
    }
}
