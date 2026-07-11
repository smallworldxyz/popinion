//! Reporting & deliberation subsystem.
//!
//! - `agent`: LLM tool-calling report generator over `sim::store::Store`
//!   (evidence loop -> boosted synthesis -> reflection), plus report Q&A chat.
//! - `registry`: in-process report state (status/progress/sections/logs).
//! - `panel_chat` / `survey`: deliberation features over live simulated agents;
//!   route-free service functions the simulation router mounts.
//! - `interview`: the `AgentInterviewer` seam between panel chat / surveys and
//!   the simulation engine — implemented live by the simulation router, with an
//!   LLM-backed `MockInterviewer` fallback for standalone report use.

pub mod agent;
pub mod interview;
pub mod panel_chat;
pub mod registry;
pub mod survey;

// Consumed by the simulation router / engine (the panel-chat & survey endpoints).
pub use interview::{AgentInterviewer, MockInterviewer};
pub use panel_chat::{panel_chat, PanelChatOptions, PanelChatResult, Persona};
pub use survey::{
    survey_create, survey_deploy, survey_get, survey_list, SurveyResult, SurveyTemplate,
};
