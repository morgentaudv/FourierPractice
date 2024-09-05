use crate::{
    math::frequency::EFrequency,
    wave::{sample::UniformedSample, sine::emitter::SineUnitSampleEmitter},
};

use super::{
    EProcessResult, EProcessState, ESineWaveEmitterType, EmitterRange, ProcessControlItem, ProcessOutputBuffer,
    ProcessProcessorInput, Setting, TInputNoneOutputBuffer, TProcess,
};

/// 正弦波を使って波形のバッファを作るための構造体
#[derive(Debug)]
pub struct SineWaveEmitterProcessData {
    setting: Setting,
    common: ProcessControlItem,
    emitter_type: ESineWaveEmitterType,
    /// `[0, 1]`まで
    intensity: f64,
    frequency: f64,
    range: EmitterRange,
    /// 処理後に出力情報が保存されるところ。
    output: Option<ProcessOutputBuffer>,
    /// 波形を出力するEmitter。
    emitter: Option<SineUnitSampleEmitter>,
}

impl SineWaveEmitterProcessData {
    /// ピンクノイズの生成
    pub fn new_pink(intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::PinkNoise,
            intensity: intensity,
            frequency: 0.0,
            range: range,
            setting: setting,
            output: None,
            emitter: None,
        }
    }

    /// ホワイトノイズの生成
    pub fn new_white(intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::WhiteNoise,
            intensity: intensity,
            frequency: 0.0,
            range: range,
            setting: setting,
            output: None,
            emitter: None,
        }
    }

    /// サイン波形の生成
    pub fn new_sine(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Sine,
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
            emitter: None,
        }
    }

    /// ノコギリ波形の生成
    pub fn new_saw(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Saw,
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
            emitter: None,
        }
    }

    /// 三角波形の生成
    pub fn new_triangle(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Triangle,
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
            emitter: None,
        }
    }

    /// 矩形波の生成
    pub fn new_square(
        frequency: EFrequency,
        duty_rate: f64,
        intensity: f64,
        range: EmitterRange,
        setting: Setting,
    ) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Square { duty_rate },
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
            emitter: None,
        }
    }
}

impl SineWaveEmitterProcessData {
    /// 初期化する
    fn initialize(&mut self) {
        let emitter = match self.emitter_type {
            ESineWaveEmitterType::PinkNoise => SineUnitSampleEmitter::new_pinknoise(self.intensity),
            ESineWaveEmitterType::WhiteNoise => SineUnitSampleEmitter::new_whitenoise(self.intensity),
            ESineWaveEmitterType::Sine => {
                SineUnitSampleEmitter::new_sine(self.frequency, 0.0, self.intensity, self.setting.sample_rate as usize)
            }
            ESineWaveEmitterType::Saw => SineUnitSampleEmitter::new_sawtooth(
                self.frequency,
                0.0,
                self.intensity,
                self.setting.sample_rate as usize,
            ),
            ESineWaveEmitterType::Triangle => SineUnitSampleEmitter::new_triangle(
                self.frequency,
                0.0,
                self.intensity,
                self.setting.sample_rate as usize,
            ),
            ESineWaveEmitterType::Square { duty_rate } => SineUnitSampleEmitter::new_square(
                self.frequency,
                duty_rate,
                0.0,
                self.intensity,
                self.setting.sample_rate as usize,
            ),
        };
        self.emitter = Some(emitter);
    }

    /// 初期化した情報から設定分のOutputを更新する。
    fn next_samples(&mut self, _input: &ProcessProcessorInput) -> Vec<UniformedSample> {
        assert!(self.emitter.is_some());

        // 設定のサンプル数ずつ吐き出す。
        // ただし今のと最終長さと比べて最終長さより長い分は0に埋める。
        let end_sample_index = {
            let ideal_add_time = self.setting.sample_count_frame as f64 / self.setting.sample_rate as f64;
            let ideal_next_time = self.common.elapsed_time + ideal_add_time;

            let mut add_time = ideal_add_time;
            if ideal_next_time > self.range.length {
                add_time = self.range.length - self.common.elapsed_time;
            }

            let samples = (add_time * self.setting.sample_rate as f64).ceil() as usize;
            assert!(samples <= self.setting.sample_count_frame);
            samples
        };

        let mut samples = self.emitter.as_mut().unwrap().next_samples(self.setting.sample_count_frame);
        if end_sample_index < samples.len() {
            // [end_sample_index, len())までに0に埋める。
            samples
                .iter_mut()
                .skip(end_sample_index)
                .for_each(|v| *v = UniformedSample::MIN);
        }
        samples
    }

    fn update_state_stopped(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        // 初期化する。
        self.initialize();
        assert!(self.emitter.is_some());

        // 初期化した情報から設定分のOutputを更新する。
        // outputのどこかに保持する。
        let buffer = self.next_samples(input);
        let elapsed_time = buffer.len() as f64 / self.setting.sample_rate as f64;
        self.output = Some(ProcessOutputBuffer {
            buffer,
            setting: self.setting.clone(),
            range: self.range,
        });

        // 時間更新
        self.common.elapsed_time += elapsed_time;
        self.common.process_timestamp += 1;

        // 状態確認
        if self.common.elapsed_time < self.range.length {
            self.common.state = EProcessState::Playing;
            return EProcessResult::Pending;
        } else {
            self.common.state = EProcessState::Finished;
            return EProcessResult::Finished;
        }
    }

    fn update_state_playing(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        // 初期化した情報から設定分のOutputを更新する。
        // outputのどこかに保持する。
        let buffer = self.next_samples(input);
        let elapsed_time = buffer.len() as f64 / self.setting.sample_rate as f64;
        self.output = Some(ProcessOutputBuffer {
            buffer,
            setting: self.setting.clone(),
            range: self.range,
        });

        // 時間更新
        self.common.elapsed_time += elapsed_time;
        self.common.process_timestamp += 1;

        // 状態確認
        if self.common.elapsed_time < self.range.length {
            self.common.state = EProcessState::Playing;
            return EProcessResult::Pending;
        } else {
            self.common.state = EProcessState::Finished;
            return EProcessResult::Finished;
        }
    }
}

impl TInputNoneOutputBuffer for SineWaveEmitterProcessData {
    fn get_output(&self) -> ProcessOutputBuffer {
        assert!(self.output.is_some());
        self.output.as_ref().unwrap().clone()
    }
}

impl TProcess for SineWaveEmitterProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        match self.common.state {
            EProcessState::Stopped => self.update_state_stopped(input),
            EProcessState::Playing => self.update_state_playing(input),
            EProcessState::Finished => {
                return EProcessResult::Finished;
            }
        }
    }

    /// 自分が処理可能なノードなのかを確認する。
    fn can_process(&self) -> bool {
        return true;
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
