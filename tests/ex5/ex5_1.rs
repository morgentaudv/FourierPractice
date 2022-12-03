use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EIntensityControlItem, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder,
    },
};

const C4_FLOAT: f32 = 261.63;
const C5_FLOAT: f32 = C4_FLOAT * 2f32;

fn triangle_fragments(
    start_time: f32,
    period: f32,
    frequency: f32,
    order_factor: u32,
) -> Option<Vec<WaveSoundSetting>> {
    if start_time < 0f32 || period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    // 基本音を入れる。
    const BASE_INTENSITY: f64 = 0.5;
    setting
        .frequency(frequency)
        .start_sec(start_time)
        .length_sec(period)
        .intensity_control_items(vec![EIntensityControlItem::Fade {
            start_time: 0.0,
            length: period as f64,
            start_factor: 0.5,
            end_factor: 0.0,
        }])
        .intensity(BASE_INTENSITY);
    results.push(setting.build().unwrap());

    // 倍音を入れる。
    for i in 2..order_factor {
        let order = (2 * i) - 1;
        let overtone_frequency = frequency * (order as f32);
        let intensity = BASE_INTENSITY * (order as f64).powi(2).recip() * {
            if i % 2 == 0 {
                -1.0
            } else {
                1.0
            }
        };
        results.push(setting.frequency(overtone_frequency).intensity(intensity).build().unwrap());
    }

    Some(results)
}

#[test]
fn ex5_1_test() {
    const WRITE_FILE_PATH: &'static str = "assets/ex5/ex5_1.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = triangle_fragments(0f32, 5f32, C5_FLOAT, 50).unwrap();
    let sound = WaveSound::from_settings(&fmt_setting, &sound_settings);
    let container = WaveContainer::from_wavesound(&sound).unwrap();

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create file.");
        let mut writer = io::BufWriter::new(dest_file);
        container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}