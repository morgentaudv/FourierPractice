use std::f64::consts::PI;

use derive_builder::Builder;
use itertools::Itertools;

use super::complex::Complex;
use super::container::WaveContainer;
use super::sample::UniformedSample;
use super::PI2;

/// 窓関数（Windowing Function）の種類の値を持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EWindowFunction {
    /// ハン窓関数を適用する。
    Hann,
}

impl EWindowFunction {
    /// 掛け算数値を計算する。もし範囲外なら、0だけを返す。
    pub fn get_factor(&self, length: f64, time: f64) -> f64 {
        // もし範囲外なら0を返す。
        if time < 0.0 || time > length {
            return 0f64;
        }

        let t = (time / length).clamp(0.0, 1.0);
        match self {
            EWindowFunction::Hann => {
                // 中央が一番高く、両端が0に収束する。
                (1f64 - (PI2 * t).cos()) * 0.5f64
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EAnalyzeMethod {
    #[default]
    DFT,
    FFT,
}

///
#[derive(Debug, Default, Clone, Copy, Builder)]
#[builder(default)]
pub struct FrequencyAnalyzer {
    pub time_start: f64,
    pub frequency_start: f64,
    pub frequency_length: f64,
    pub sample_counts: usize,
    pub window_function: Option<EWindowFunction>,
    pub analyze_method: EAnalyzeMethod,
}

impl FrequencyAnalyzer {
    /// 周波数特性を計算する。
    pub fn analyze_frequencies(&self, container: &WaveContainer) -> Option<Vec<SineFrequency>> {
        // まず入れられた情報から範囲に収められそうなのかを確認する。
        // sound_lengthはhalf-opened rangeなのかclosedなのかがいかがわしい模様。
        let wave_sound_length = container.sound_length() as f64;
        let recip_sample_per_sec = (container.samples_per_second() as f64).recip();
        let samples_time_length = recip_sample_per_sec * (self.sample_counts as f64);
        let samples_time_end = self.time_start + samples_time_length;

        if samples_time_end > wave_sound_length {
            return None;
        }

        // [time_start, time_start + sampels_time_end)の時間領域を
        // [frequency_start, frequency_start + frequency_length]まで分析する。
        if self.frequency_length <= 0.0 || self.frequency_start < 0.0 || self.sample_counts <= 0 {
            return None;
        }

        assert!(self.frequency_length == (self.sample_counts as f64));
        match self.analyze_method {
            EAnalyzeMethod::DFT => Some(analyze_as_dft(self, container)),
            EAnalyzeMethod::FFT => {
                if !self.sample_counts.is_power_of_two() {
                    None
                } else {
                    Some(analyze_as_fft(self, container))
                }
            }
        }
    }
}

impl FrequencyAnalyzer {
    ///
    fn get_window_fn_factor(&self, length: f64, time: f64) -> f64 {
        if let Some(window_fn) = self.window_function {
            window_fn.get_factor(length, time)
        } else {
            1f64
        }
    }
}

/// [`Discreted Fourier Transform`](https://en.wikipedia.org/wiki/Discrete_Fourier_transform)（離散フーリエ変換）を行って
/// 周波数特性を計算して返す。
fn analyze_as_dft(analyzer: &FrequencyAnalyzer, container: &WaveContainer) -> Vec<SineFrequency> {
    assert!(container.channel() == 1);

    let frequency_end = analyzer.frequency_start + analyzer.frequency_length;
    let freq_precision = analyzer.frequency_length * (analyzer.sample_counts as f64).recip();

    let mut results = vec![];
    let mut cursor_frequency = analyzer.frequency_start;

    let sample_start_index = container.calculate_sample_index_of_time(analyzer.time_start).expect("");
    let buffer = container.uniformed_sample_buffer();
    while cursor_frequency < frequency_end {
        let mut frequency = Complex::<f64>::default();

        for local_i in 0..analyzer.sample_counts {
            // アナログ波形に複素数の部分は存在しないので、Realパートだけ扱う。
            // coeff_input = exp(2pifn / N)
            let time_factor = (local_i as f64) / (analyzer.sample_counts as f64);
            let coeff_input = PI2 * cursor_frequency * time_factor;
            let coefficient = Complex::<f64>::from_exp(coeff_input * -1.0);

            let sample = {
                let sample_i = local_i + sample_start_index;
                let amplitude = buffer[sample_i].to_f64();
                let window_factor = analyzer.get_window_fn_factor(1.0, time_factor);
                amplitude * window_factor
            };
            frequency += sample * coefficient;
        }

        results.push(SineFrequency::from_complex_f64(cursor_frequency, frequency));

        // 周波数カーソルを進める。
        cursor_frequency += freq_precision;
    }

    results
}

/// [`Fast Fourier Transform`](https://en.wikipedia.org/wiki/Fast_Fourier_transform)（高速フーリエ変換）を行って
/// 周波数特性を計算して返す。
fn analyze_as_fft(analyzer: &FrequencyAnalyzer, container: &WaveContainer) -> Vec<SineFrequency> {
    assert!(container.channel() == 1);
    assert!(analyzer.sample_counts.is_power_of_two());

    // まず最後に求められる各Frequencyの情報をちゃんとした位置に入れるためのIndexルックアップテーブルを作る。
    // たとえば、index_count = 8のときに1番目のFrequency情報は4番目に入れるべきなど…
    let lookup_table = {
        // ビットリバーステクニックを使ってテーブルを作成。
        let mut results = vec![0];
        let mut addition_count = analyzer.sample_counts >> 1;
        while addition_count > 0 {
            results.append(&mut results.iter().map(|v| v + addition_count).collect_vec());
            addition_count >>= 1;
        }

        results
    };
    let sample_counts = analyzer.sample_counts;

    // まず最後レベルの信号を計算する。index_count分作る。
    let final_signals = {
        let sample_start_index = container.calculate_sample_index_of_time(analyzer.time_start).expect("");
        let samples_buffer = container.uniformed_sample_buffer();

        let mut prev_signals: Vec<Complex<f64>> = vec![];
        prev_signals.reserve(sample_counts);

        // 無限に伸びる周期波形をつくるよりは、すでに与えられた波形をもっと細かく刻んでサンプルしたほうが安定そう。
        for local_i in 0..sample_counts {
            // アナログ波形に複素数の部分は存在しないので、Realパートだけ扱う。
            let amplitude = {
                let sample_i = local_i + sample_start_index;
                let signal = samples_buffer[sample_i].to_f64();

                let time_factor = (local_i as f64) / (sample_counts as f64);
                let window_factor = analyzer.get_window_fn_factor(1.0, time_factor);
                signal * window_factor
            };

            // 負の数のAmplitudeも可能。
            prev_signals.push(Complex::<f64> {
                real: amplitude,
                imag: 0.0,
            });
        }

        //
        let mut next_signals: Vec<Complex<f64>> = vec![];
        next_signals.resize(analyzer.sample_counts, <Complex<f64> as Default>::default());

        let level = (sample_counts as f64).log2().ceil() as usize;
        for lv_i in 0..level {
            let index_period = sample_counts >> lv_i;
            let half_index = index_period >> 1;

            for period_i in (0..sample_counts).step_by(index_period) {
                for local_i in 0..half_index {
                    let lhs_i = period_i + local_i;
                    let rhs_i = period_i + local_i + half_index;
                    let prev_lhs_signal = prev_signals[lhs_i];
                    let prev_rhs_signal = prev_signals[rhs_i];
                    let coefficient =
                        Complex::<f64>::from_exp(PI2 * (local_i as f64) / (index_period as f64)).conjugate();

                    let new_lhs_signal = prev_lhs_signal + prev_rhs_signal;
                    let new_rhs_signal = coefficient * (prev_lhs_signal - prev_rhs_signal);
                    next_signals[lhs_i] = new_lhs_signal;
                    next_signals[rhs_i] = new_rhs_signal;
                }
            }

            // 次のレベルでprev→nextをするためにswapする。
            std::mem::swap(&mut prev_signals, &mut next_signals);
        }

        prev_signals
    };

    // 計算済みの`final_signals`はビットリバースのシグナルリストに１対１対応しているので
    // このままルックアップテーブルから結果シグナルに入れて[`SineFrequency`]に変換して返す。
    let mut results = vec![];
    results.resize(sample_counts, SineFrequency::default());

    let freq_precision = analyzer.frequency_length * (analyzer.sample_counts as f64).recip();
    for freq_i in 0..sample_counts {
        let target_i = lookup_table[freq_i];

        let frequency = analyzer.frequency_start + (freq_precision * (target_i as f64));
        let sine_freq = SineFrequency::from_complex_f64(frequency, final_signals[freq_i]);
        results[target_i] = sine_freq;
    }

    results
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ETransformMethod {
    #[default]
    IDFT,
    IFFT,
}

#[derive(Debug, Default, Clone, Copy, Builder)]
#[builder(default)]
pub struct FrequencyTransformer {
    pub transform_method: ETransformMethod,
}

impl FrequencyTransformer {
    pub fn transform_frequencies(
        &self,
        container: &WaveContainer,
        frequencies: &[SineFrequency],
    ) -> Option<Vec<UniformedSample>> {
        // まずそれぞれの方法が使えるかを確認する。
        // たとえば、IFFTは周波数特性のサイズが2のべき乗じゃないとできない。
        if frequencies.is_empty() {
            return None;
        }

        match self.transform_method {
            ETransformMethod::IDFT => Some(transform_as_idft(self, container, frequencies)),
            ETransformMethod::IFFT => Some(transform_as_ifft(self, container, frequencies)),
        }
    }
}

/// Inverse Discrete Fourier Transformを使って波形のサンプルリストに変換する。
fn transform_as_idft(
    _transformer: &FrequencyTransformer,
    container: &WaveContainer,
    frequencies: &[SineFrequency],
) -> Vec<UniformedSample> {
    // まず0からtime_lengthまでのサンプルだけを収集する。
    // time_lengthの間のサンプル数を全部求めて
    //
    // ただ、DFTでの時間計算が [0, 1]範囲となっていたので、IDFTも同じくする？
    // とりあえずf64のサンプルに変換する。

    // その前に、今はcontainerのチャンネルは1にする。

    assert!(container.samples_per_second() > 0);
    let samples_count = frequencies.len();

    let mut raw_samples = vec![];
    for time_i in 0..samples_count {
        let time_factor = (time_i as f64) / (samples_count as f64);

        // すべてのfrequency特性にイテレーションする。
        // a(k) * cos(2pik * time + phase)
        let summed: f64 = frequencies
            .iter()
            .map(|frequency| frequency.amplitude * ((PI2 * frequency.frequency * time_factor) + frequency.phase).cos())
            .sum();

        // 1 / N (sigma)
        //let raw_sample = summed / analyzer.time_length;
        let raw_sample = summed / (samples_count as f64);
        raw_samples.push(raw_sample);
    }

    //for raw_samples in &raw_samples { println!("{:?}", raw_samples); }

    raw_samples
        .into_iter()
        .map(|raw_sample| UniformedSample::from_f64(raw_sample))
        .collect_vec()
}

/// Inverse Fast Fourier Transformを使って波形のサンプルリストに変換する。
/// `frequencies`のサイズは必ず2のべき乗である必要がある。
fn transform_as_ifft(
    _transformer: &FrequencyTransformer,
    container: &WaveContainer,
    frequencies: &[SineFrequency],
) -> Vec<UniformedSample> {
    assert!(frequencies.len().is_power_of_two());
    assert!(container.samples_per_second() > 0);

    // freqs_count == samples_countにする。
    let samples_count = frequencies.len();

    // FFTから逆順で波形のAmplitudeを計算する。
    //
    // > まず最後に求められる各Frequencyの情報をちゃんとした位置に入れるためのIndexルックアップテーブルを作る。
    // > たとえば、index_count = 8のときに1番目のFrequency情報は4番目に入れるべきなど…
    //
    // FFTではそうだったが、IFFTではこの`lookup_table`からComplex情報を戻す。
    let lookup_table = {
        // ビットリバーステクニックを使ってテーブルを作成。
        let mut results = vec![0];
        let mut addition_count = samples_count >> 1;
        while addition_count > 0 {
            results.append(&mut results.iter().map(|v| v + addition_count).collect_vec());
            addition_count >>= 1;
        }

        results
    };
    assert!(lookup_table.len() == samples_count);

    // ループしながら展開。
    let final_signals = {
        let lastlv_samples = {
            let mut lastlv_samples = vec![];
            lastlv_samples.resize(samples_count, Complex::<f64>::default());

            for (write_i, search_i) in lookup_table.iter().enumerate() {
                lastlv_samples[write_i] = frequencies[*search_i].to_complex_f64();
            }
            lastlv_samples
        };

        let mut prev_signals = lastlv_samples;
        let mut next_signals: Vec<Complex<f64>> = vec![];
        next_signals.resize(samples_count, <Complex<f64> as Default>::default());

        // (level, 0]順で展開をする。
        let level = (samples_count as f64).log2().ceil() as usize;
        for level_i in (0..level).rev() {
            let index_period = samples_count >> level_i;
            let half_index = index_period >> 1;

            for period_i in (0..samples_count).step_by(index_period) {
                for local_i in 0..half_index {
                    // 計算過程
                    // prev[pli] = x + y
                    // prev[pri] = K(x - y) なので
                    // next[nli] = x = ((prev[pri] / K) + prev[pli]) / 2である。
                    // next[nri] = prev[pli] - xとなる。
                    let prev_lhs_i = period_i + local_i;
                    let prev_rhs_i = period_i + local_i + half_index;

                    let coefficient =
                        Complex::<f64>::from_exp(PI2 * (local_i as f64) / (index_period as f64)).conjugate();
                    let lhs_value = 0.5 * ((prev_signals[prev_rhs_i] / coefficient) + prev_signals[prev_lhs_i]);
                    let rhs_value = prev_signals[prev_lhs_i] - lhs_value;

                    let next_lhs_i = period_i + local_i;
                    let next_rhs_i = period_i + local_i + half_index;
                    next_signals[next_lhs_i] = lhs_value;
                    next_signals[next_rhs_i] = rhs_value;
                }
            }

            // 次のレベルでprev→nextをするためにswapする。
            std::mem::swap(&mut prev_signals, &mut next_signals);
        }

        prev_signals
    };

    // `final_signals`はまだComplexなので、しかし計算がちゃんとしていればimagはなくなると思う。
    // mapでrealだけを取得してUniformedSampleに変換する。
    final_signals
        .into_iter()
        .map(|signal| UniformedSample::from_f64(signal.real))
        .collect_vec()
}

/// サイン波形の周波数の特性を表す。
#[derive(Default, Debug, Clone, Copy)]
pub struct SineFrequency {
    pub frequency: f64,
    pub amplitude: f64,
    pub phase: f64,
}

impl SineFrequency {
    pub fn from(frequency: f64, (freq_real, freq_imag): (f32, f32)) -> Self {
        Self {
            frequency,
            amplitude: (freq_real.powi(2) + freq_imag.powi(2)).sqrt() as f64,
            phase: (freq_imag / freq_real).atan() as f64,
        }
    }

    pub fn from_complex_f32(frequency: f32, complex: Complex<f32>) -> Self {
        Self {
            frequency: frequency as f64,
            amplitude: complex.absolute() as f64,
            phase: complex.phase() as f64,
        }
    }

    pub fn from_complex_f64(frequency: f64, complex: Complex<f64>) -> Self {
        Self {
            frequency,
            amplitude: complex.absolute(),
            phase: complex.phase(),
        }
    }

    pub fn to_complex_f64(&self) -> Complex<f64> {
        let real = self.phase.cos() * self.amplitude;
        let imag = self.phase.sin() * self.amplitude;
        Complex::<f64> { real, imag }
    }
}
