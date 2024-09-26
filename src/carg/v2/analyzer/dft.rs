use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::wave::analyze::{
    analyzer::{FrequencyAnalyzerV2, WaveContainerSetting},
    method::EAnalyzeMethod,
    window::EWindowFunction,
};
use itertools::Itertools;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::{EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputFrequency, ProcessOutputText, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};

#[derive(Debug)]
pub struct AnalyzerDFTProcessData {
    common: ProcessControlItem,
    level: usize,
}

impl TPinCategory for AnalyzerDFTProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> { vec!["in"] }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> { vec!["out_info", "out_freq"] }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::BUFFER_MONO),
            "out_info" => Some(pin_category::TEXT),
            "out_freq" => Some(pin_category::FREQUENCY),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl AnalyzerDFTProcessData {
    pub fn create_from(node: &ENode, _setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::AnalyzerDFT { level } => {
                let item = Self::new(*level);
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    fn new(level: usize) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::AnalyzerDFT),
            level,
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // チェックしてself.levelよりバッファが多くないと処理しない。
        let can_process = match &*self.common.get_input_internal("in").unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() >= self.level,
            _ => false,
        };
        if !can_process {
            return;
        }

        let (buffer, sample_rate) = match &mut *self.common.get_input_internal_mut("in").unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => {
                let buffer = v.buffer.drain(..self.level).collect_vec();
                (buffer, v.setting.as_ref().unwrap().sample_rate)
            }
            _ => unreachable!("Unexpected input."),
        };

        // このノードでは最初からADを行う。
        // もし尺が足りなければ、そのまま終わる。
        // inputのSettingのsample_rateから各バッファのサンプルの発生時間を計算する。
        let samples_count = self.level;
        let frequencies = {
            let analyzer = FrequencyAnalyzerV2 {
                analyze_method: EAnalyzeMethod::DFT,
                frequency_start: 0.0,
                frequency_width: sample_rate as f64,
                frequency_bin_count: self.level as u32,
                window_function: EWindowFunction::None,
            };

            let setting = WaveContainerSetting {
                container: &buffer,
                start_sample_index: 0,
                samples_count,
            };
            analyzer.analyze_container(&setting).unwrap()
        };

        // out_info関連出力処理
        if self.common.is_output_pin_connected("out_info") {
            let mut log = "".to_owned();
            for frequency in &frequencies {
                if frequency.amplitude < 5.0 {
                    continue;
                }

                log += &format!("(Freq: {:.0}, Amp: {:.2}) ", frequency.frequency, frequency.amplitude);
            }

            self.common
                .insert_to_output_pin("out_info", EProcessOutput::Text(ProcessOutputText { text: log }))
                .unwrap();
        }

        // out_freq関連出力処理
        if self.common.is_output_pin_connected("out_freq") {
            let analyzed_sample_len = self.level;
            self.common
                .insert_to_output_pin(
                    "out_freq",
                    EProcessOutput::Frequency(ProcessOutputFrequency {
                        frequencies,
                        analyzed_sample_len,
                    }),
                )
                .unwrap();
        }

        // 状態変更。
        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TProcess for AnalyzerDFTProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        self.common.is_all_input_pins_update_notified()
    }

    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------