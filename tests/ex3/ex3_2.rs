use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EFrequencyItem, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder,
    },
};

const C4_FLOAT: f32 = 261.63;
const C5_FLOAT: f32 = C4_FLOAT * 2f32;

fn square_fragments(period: f32, frequency: f32, order_factor: u32) -> Option<Vec<WaveSoundSetting>> {
    if period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    // 基本音を入れる。
    setting
        .frequency(EFrequencyItem::Constant {
            frequency: frequency as f64,
        })
        .length_sec(period)
        .intensity(0.6f64);
    results.push(setting.build().unwrap());

    // 倍音を入れる。
    for i in 2..order_factor {
        let order = (2 * i) - 1;
        let overtone_frequency = frequency * (order as f32);
        let intensity = 0.6f64 * (order as f64).recip();
        results.push(
            setting
                .frequency(EFrequencyItem::Constant {
                    frequency: overtone_frequency as f64,
                })
                .intensity(intensity)
                .length_sec(period)
                .build()
                .unwrap(),
        );
    }

    Some(results)
}

#[test]
fn write_fromc4toc5() {
    const WRITE_FILE_PATH: &'static str = "assets/ex3/ex3_2.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = square_fragments(1f32, C5_FLOAT, 50).unwrap();
    let sound = WaveSound::from_settings(&fmt_setting, &sound_settings);
    let container = WaveContainer::from_wavesound(&sound).unwrap();

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
