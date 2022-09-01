use std::{os::raw::c_int, thread::sleep, time::Duration};

use super::note::Notes;
use super::parser::{EventData, MidiFile};
use super::status::StatusType;

#[cfg(windows)]
use windows::Win32::Media::{
    Audio::{
        midiInClose, midiInOpen, midiInStart, midiInStop, midiOutClose, midiOutOpen, midiOutReset,
        midiOutShortMsg, CALLBACK_FUNCTION, CALLBACK_NULL, HMIDIIN, HMIDIOUT,
    },
    MM_MIM_DATA,
};

pub unsafe fn send_midi(device: HMIDIOUT, status: StatusType, channel: u32, low: u32, high: u32) {
    let dw_msg = status as u32 | channel | (high << 16) | (low << 8);
    midiOutShortMsg(device, dw_msg);
}

pub unsafe fn send_midi_single(device: HMIDIOUT, status: StatusType, channel: u32, low: u32) {
    let dw_msg = status as u32 | channel | (low << 8);
    midiOutShortMsg(device, dw_msg);
}

pub unsafe fn output() {
    let mut h_device = HMIDIOUT::default();
    midiOutOpen(&mut h_device, 0u32, 0, 0, CALLBACK_NULL);

    // send_midi_single(h_device, Status::ProgramChange, 0, 1);

    let mut midi = MidiFile::create();
    midi.parse("test.mid").unwrap();
    println!("A: {}!", midi.tempo);
    let mut prev_tick = 0;
    send_midi_single(h_device, StatusType::ProgramChange, 0, 0);
    for i in midi.tracks.iter() {
        for ev in i.events.iter() {
            if ev.delta_tick > 1000 {
                if let EventData::SysexData { .. } = &ev.data {
                    continue;
                }
            }
            sleep(Duration::from_millis(
                ((ev.delta_tick as f64) * (midi.tempo as f64) / midi.division as f64 / 1000.0).round()
                    as u64,
            ));
            prev_tick += ev.delta_tick;
            if let EventData::NoteOnOffData { key, velocity } = ev.data {
                let note = Notes::from(key as u32).unwrap();
                if ev.status.status_type == StatusType::NoteOn {
                    send_midi(
                        h_device,
                        StatusType::NoteOn,
                        0,
                        note.0.octave(note.1 as u32).unwrap(),
                        velocity as u32,
                    );
                } else {
                    send_midi(
                        h_device,
                        StatusType::NoteOff,
                        0,
                        note.0.octave(note.1 as u32).unwrap(),
                        0,
                    );
                }

                println!(
                    "Status: {:?}, DeltaTick: {}, Total: {}, Millis: {}, Note: {:?}",
                    ev.status.status_type,
                    ev.delta_tick,
                    prev_tick,
                    ((ev.delta_tick * midi.tempo) as f32 / midi.division as f32 / 1000.0).round()
                        as u64,
                    note.0
                );
            }
        }
    }

    midiOutReset(h_device);
    midiOutClose(h_device);
}

pub fn midi_in_proc(
    _h_device: HMIDIIN,
    w_msg: u32,
    _dw_instance: u32,
    dw_param1: u32,
    _dw_param2: u32,
) {
    if w_msg == MM_MIM_DATA {
        let status = dw_param1 & 0xff;
        let high = dw_param1 >> 8 & 0xff;
        let low = dw_param1 >> 16 & 0xff;
        println!("Status: {:X} - High: {:X} - Low: {:X}", status, high, low);
    }
}

extern "C" {
    fn _getch() -> c_int;
    fn _kbhit() -> c_int;
}

pub unsafe fn input() {
    let mut h_device = HMIDIIN::default();
    midiInOpen(
        &mut h_device,
        0u32,
        midi_in_proc as usize,
        0usize,
        CALLBACK_FUNCTION,
    );
    midiInStart(h_device);

    loop {
        // BREAK IF
        if _kbhit() == 0 {
            sleep(Duration::from_millis(100));
            continue;
        }
        let c = _getch();
        if c == 0x1B {
            break;
        };
        if c == 113 {
            break;
        };
    }

    midiInStop(h_device);
    midiInClose(h_device);
}
