//! Reporting & deliberation subsystem.
//!
//! - `agent`: LLM tool-calling report generator over `sim::store::Store`
//!   (evidence loop -> boosted synthesis -> reflection), plus report Q&A chat.
//! - `registry`: in-process report state (status/progress/sections/logs).
//! - `panel_chat` / `survey`: deliberation features over live simulated agents;
//!   route-free service functions the simulation router mounts.
//! - `interview`: the `AgentInterviewer` seam between panel chat / surveys and
//!   the simulation engine, with an LLM-backed `MockInterviewer` until the
//!   real engine implementation lands.

pub mod agent;
// interview/panel_chat/survey are dead code until the orchestrator mounts the
// panel-chat and survey endpoints on the simulation router; covered by tests.
#[allow(dead_code)]
pub mod interview;
#[allow(dead_code)]
pub mod panel_chat;
pub mod registry;
#[allow(dead_code)]
pub mod survey;

// Consumed by the simulation router / engine once the orchestrator mounts the
// panel-chat and survey endpoints; unused inside this subsystem by design.
#[allow(unused_imports)]
pub use interview::{AgentInterviewer, MockInterviewer};
#[allow(unused_imports)]
pub use panel_chat::{panel_chat, PanelChatOptions, PanelChatResult, Persona};
#[allow(unused_imports)]
pub use survey::{
    survey_create, survey_deploy, survey_get, survey_list, SurveyResult, SurveyTemplate,
};
