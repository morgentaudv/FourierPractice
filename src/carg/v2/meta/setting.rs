use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    pub time_tick_mode: ETimeTickMode,
    /// 更新時の推奨されるサンプル数。
    /// たとえば48kHzだと約21ms弱ぐらいになる。
    /// この値は必ず2のべき乗数でなければならない。
    pub sample_count_frame: usize,
    /// 音生成のために使うサンプルレートを指す。0より上であること。
    pub sample_rate: u64,
}

impl Setting {
    pub fn from_serde_value(value: Value) -> anyhow::Result<Self> {
        let setting: Setting = serde_json::from_value(value)?;
        if !setting.sample_count_frame.is_power_of_two() {
            return Err(anyhow::anyhow!(
            "Given `sample_count_frame` is not power of two. (256, 512, 1024...)"
            ));
        }

        Ok(setting)
    }

    pub fn get_default_tick_threshold(&self) -> f64 {
        (self.sample_count_frame as f64) / (self.sample_rate as f64)
    }
}

/// フレームTickのモード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ETimeTickMode {
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "realtime")]
    Realtime,
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
