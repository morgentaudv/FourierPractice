use crate::carg::v2::meta::{pin_category, EPinCategoryFlag};
use crate::carg::v2::{EProcessOutput, ProcessOutputBuffer, ProcessOutputBufferStereo, ProcessOutputFrequency, ProcessOutputText};

/// [`EProcessOutput`]などをまとめて管理するコンテナ。
#[derive(Debug, Clone)]
pub enum EProcessOutputContainer {
    Empty,
    BufferMono(ProcessOutputBuffer),
    BufferStereo(ProcessOutputBufferStereo),
    Text(ProcessOutputText),
    Frequency(ProcessOutputFrequency),
}

impl EProcessOutputContainer {
    pub fn as_pin_category_flag(&self) -> EPinCategoryFlag {
        match self {
            EProcessOutputContainer::Empty => pin_category::START,
            EProcessOutputContainer::BufferMono(_) => pin_category::BUFFER_MONO,
            EProcessOutputContainer::BufferStereo(_) => pin_category::BUFFER_STEREO,
            EProcessOutputContainer::Text(_) => pin_category::TEXT,
            EProcessOutputContainer::Frequency(_) => pin_category::FREQUENCY,
        }
    }

    pub fn reset_with(&mut self, new_output: EProcessOutput) {
        match new_output {
            EProcessOutput::None => *self = EProcessOutputContainer::Empty,
            EProcessOutput::BufferMono(v) => {
                *self = EProcessOutputContainer::BufferMono(v);
            }
            EProcessOutput::Text(v) => {
                *self = EProcessOutputContainer::Text(v);
            }
            EProcessOutput::Frequency(v) => {
                *self = EProcessOutputContainer::Frequency(v);
            }
            EProcessOutput::BufferStereo(v) => {
                *self = EProcessOutputContainer::BufferStereo(v);
            }
        }
    }

    pub fn insert_with(&mut self, new_output: EProcessOutput) -> anyhow::Result<()> {
        if self.as_pin_category_flag() != new_output.as_pin_category_flag() {
            return Err(anyhow::anyhow!("Pin Category Flag is not matched."));
        }

        match self {
            EProcessOutputContainer::Empty => {}
            EProcessOutputContainer::BufferMono(dst) => {
                if let EProcessOutput::BufferMono(src) = new_output {
                    *dst = src;
                } else {
                    unreachable!("Unexpected branch");
                }
            }
            EProcessOutputContainer::Text(dst) => {
                if let EProcessOutput::Text(src) = new_output {
                    *dst = src;
                } else {
                    unreachable!("Unexpected branch");
                }
            }
            EProcessOutputContainer::Frequency(dst) => {
                if let EProcessOutput::Frequency(src) = new_output {
                    *dst = src;
                } else {
                    unreachable!("Unexpected branch");
                }
            }
            EProcessOutputContainer::BufferStereo(dst) => {
                if let EProcessOutput::BufferStereo(src) = new_output {
                    *dst = src;
                } else {
                    unreachable!("Unexpected branch");
                }
            }
        }

        Ok(())
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
