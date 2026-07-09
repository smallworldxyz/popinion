use serde::{Deserialize, Serialize};

/// Simulation configuration. Mirrors the keys the Python config generator
/// produced (time_config / agent_configs / event_config) so generated configs
/// stay compatible, but with sane defaults everywhere.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SimConfig {
    #[serde(default)]
    pub simulation_id: String,
    #[serde(default)]
    pub platform: Platform,
    #[serde(default)]
    pub time_config: TimeConfig,
    #[serde(default)]
    pub agent_configs: Vec<AgentConfig>,
    #[serde(default)]
    pub event_config: EventConfig,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    #[default]
    Reddit,
    Twitter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeConfig {
    #[serde(default = "d_total_hours")]
    pub total_simulation_hours: u32,
    #[serde(default = "d_minutes_per_round")]
    pub minutes_per_round: u32,
    #[serde(default = "d_per_hour_min")]
    pub agents_per_hour_min: u32,
    #[serde(default = "d_per_hour_max")]
    pub agents_per_hour_max: u32,
    #[serde(default = "d_peak_hours")]
    pub peak_hours: Vec<u32>,
    #[serde(default = "d_off_peak_hours")]
    pub off_peak_hours: Vec<u32>,
    #[serde(default = "d_peak_mult")]
    pub peak_activity_multiplier: f64,
    #[serde(default = "d_off_peak_mult")]
    pub off_peak_activity_multiplier: f64,
}

fn d_total_hours() -> u32 {
    72
}
fn d_minutes_per_round() -> u32 {
    30
}
fn d_per_hour_min() -> u32 {
    5
}
fn d_per_hour_max() -> u32 {
    20
}
fn d_peak_hours() -> Vec<u32> {
    vec![9, 10, 11, 14, 15, 20, 21, 22]
}
fn d_off_peak_hours() -> Vec<u32> {
    vec![0, 1, 2, 3, 4, 5]
}
fn d_peak_mult() -> f64 {
    1.5
}
fn d_off_peak_mult() -> f64 {
    0.3
}

impl Default for TimeConfig {
    fn default() -> Self {
        TimeConfig {
            total_simulation_hours: d_total_hours(),
            minutes_per_round: d_minutes_per_round(),
            agents_per_hour_min: d_per_hour_min(),
            agents_per_hour_max: d_per_hour_max(),
            peak_hours: d_peak_hours(),
            off_peak_hours: d_off_peak_hours(),
            peak_activity_multiplier: d_peak_mult(),
            off_peak_activity_multiplier: d_off_peak_mult(),
        }
    }
}

impl TimeConfig {
    pub fn total_rounds(&self) -> u32 {
        (self.total_simulation_hours * 60) / self.minutes_per_round.max(1)
    }
    pub fn hour_multiplier(&self, hour: u32) -> f64 {
        if self.peak_hours.contains(&hour) {
            self.peak_activity_multiplier
        } else if self.off_peak_hours.contains(&hour) {
            self.off_peak_activity_multiplier
        } else {
            1.0
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: i64,
    #[serde(default = "d_active_hours")]
    pub active_hours: Vec<u32>,
    #[serde(default = "d_activity_level")]
    pub activity_level: f64,
}

fn d_active_hours() -> Vec<u32> {
    (8..23).collect()
}
fn d_activity_level() -> f64 {
    0.5
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EventConfig {
    #[serde(default)]
    pub initial_posts: Vec<InitialPost>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InitialPost {
    // The crawler bridge emits this field as `user_id`; accept both so real
    // seed posts keep their author instead of collapsing to agent 0.
    #[serde(default, alias = "user_id")]
    pub poster_agent_id: i64,
    #[serde(default)]
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_post_accepts_bridge_user_id_field() {
        // The bridge serializes seed posts with `user_id`, not `poster_agent_id`.
        let p: InitialPost = serde_json::from_str(r#"{"user_id": 7, "content": "hi"}"#).unwrap();
        assert_eq!(p.poster_agent_id, 7, "seed post must keep its author, not collapse to 0");
        // The native field name still works.
        let p2: InitialPost = serde_json::from_str(r#"{"poster_agent_id": 3, "content": "x"}"#).unwrap();
        assert_eq!(p2.poster_agent_id, 3);
    }

    #[test]
    fn defaults_fill_from_empty_json() {
        let c: SimConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(c.time_config.total_rounds(), 72 * 60 / 30);
        assert_eq!(c.time_config.hour_multiplier(10), 1.5);
        assert_eq!(c.time_config.hour_multiplier(2), 0.3);
        assert_eq!(c.time_config.hour_multiplier(13), 1.0);
    }
}
