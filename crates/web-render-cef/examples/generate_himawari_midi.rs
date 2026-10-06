//! Original SMF fixture matching himawari_receiver's named-track offsets.
use std::path::PathBuf;

const BAR: u32 = 1920;
const END: u32 = BAR * 8;
type Event = (u32, Vec<u8>);

fn variable(mut value: u32, bytes: &mut Vec<u8>) {
    let mut buffer = [0; 4];
    let mut index = 3;
    buffer[index] = (value & 127) as u8;
    while {
        value >>= 7;
        value != 0
    } {
        index -= 1;
        buffer[index] = (value & 127) as u8 | 128;
    }
    bytes.extend_from_slice(&buffer[index..]);
}

fn track(name: &str, mut events: Vec<Event>) -> Vec<u8> {
    // A marker is a named MIDI track, not a MIDI marker meta-event.
    let mut data = vec![0, 255, 3, name.len() as u8];
    data.extend_from_slice(name.as_bytes());
    events.sort_by_key(|(tick, _)| *tick);
    let mut previous = 0;
    for (tick, event) in events {
        variable(tick - previous, &mut data);
        data.extend(event);
        previous = tick;
    }
    variable(END - previous, &mut data);
    data.extend_from_slice(&[255, 47, 0]);
    let mut chunk = b"MTrk".to_vec();
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    chunk.extend(data);
    chunk
}

fn note(events: &mut Vec<Event>, tick: u32, length: u32, channel: u8, pitch: u8) {
    events.push((tick, vec![0x90 | channel, pitch, 100]));
    events.push((tick + length, vec![0x80 | channel, pitch, 0]));
}

fn bend(events: &mut Vec<Event>, tick: u32, channel: u8, offset: i16) {
    let value = (8192 + offset) as u16;
    events.push((
        tick,
        vec![0xe0 | channel, (value & 127) as u8, (value >> 7) as u8],
    ));
}

fn main() -> anyhow::Result<()> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("generate_himawari_midi <output.mid>"))?,
    );
    let mut tracks = vec![track(
        "Conductor",
        vec![
            (0, vec![255, 81, 3, 7, 161, 32]),       // 120 BPM
            (0, vec![255, 88, 4, 4, 2, 24, 8]),      // 4/4
            (BAR * 4, vec![255, 81, 3, 6, 26, 128]), // 150 BPM
        ],
    )];
    for (index, name) in [
        "drums",
        "sub-cymbal-a",
        "sub-cymbal-b",
        "drums-unused",
        "cymbal-a",
        "cymbal-b",
        "main-drums-a",
        "main-drums-b",
    ]
    .into_iter()
    .enumerate()
    {
        let mut events = vec![];
        for bar in 0..8 {
            let start = bar * BAR;
            match index {
                1 => note(&mut events, start + 240, 60, 9, 38),
                2 => note(&mut events, start + 720, 60, 9, 49),
                4 => note(&mut events, start, 60, 9, 49),
                5 => note(&mut events, start + 960, 60, 9, 57),
                6 => {
                    for (tick, pitch) in [(0, 36), (480, 37), (960, 36), (1440, 37)] {
                        note(&mut events, start + tick, 120, 9, pitch);
                    }
                }
                7 => {
                    for (tick, pitch) in [
                        (0, 38),
                        (240, 39),
                        (480, 41),
                        (720, 38),
                        (960, 39),
                        (1200, 41),
                        (1440, 38),
                        (1680, 39),
                    ] {
                        note(&mut events, start + tick, 60, 9, pitch);
                    }
                }
                _ => {}
            }
        }
        tracks.push(track(name, events));
    }
    tracks.push(track("synth", vec![]));
    tracks.push(track("synth-unused", vec![]));
    for (channel, name) in [(0, "synth-a"), (1, "synth-b")] {
        let mut events = vec![];
        for bar in 0..8 {
            let start = bar * BAR;
            for (offset, pitch) in [(0, 72), (960, 76)] {
                note(
                    &mut events,
                    start + offset,
                    960,
                    channel,
                    pitch - channel * 7,
                );
                for (tick, value) in [(0, 0), (240, 4096), (480, -4096), (720, 0)] {
                    bend(&mut events, start + offset + tick, channel, value);
                }
            }
        }
        tracks.push(track(name, events));
    }
    tracks.push(track("Kaiwai Phrase", vec![]));
    let mut events = vec![];
    for bar in 0..8 {
        // Leave 0.5 seconds at each early bar's end for keep/fade/empty checks.
        for (offset, pitch) in [(0, 84), (480, 88), (960, 91)] {
            note(&mut events, bar * BAR + offset, 480, 2, pitch);
        }
    }
    tracks.push(track("phrase-notes", events));
    let mut smf = b"MThd\0\0\0\x06\0\x01".to_vec();
    smf.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
    smf.extend_from_slice(&480u16.to_be_bytes());
    for bytes in tracks {
        smf.extend(bytes);
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, &smf)?;
    println!(
        "Generated {} bytes, 15 raw tracks, PPQ 480, 8 bars, 120 -> 150 BPM: {}",
        smf.len(),
        output.display()
    );
    Ok(())
}
