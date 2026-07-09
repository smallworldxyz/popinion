use crate::llm::{Llm, Msg};

/// Seam between deliberation features (panel chat, surveys) and the live
/// simulation engine. The engine will provide the real implementation that
/// routes a question to a running agent; until then `MockInterviewer` stands in.
#[async_trait::async_trait]
pub trait AgentInterviewer: Send + Sync {
    async fn interview(&self, agent_id: i64, prompt: &str) -> anyhow::Result<String>;
}

/// LLM-backed stand-in for the live engine: answers as a generic persona so
/// panel chats and surveys can run end-to-end before the engine exists.
pub struct MockInterviewer {
    llm: Llm,
}

impl MockInterviewer {
    pub fn new(llm: Llm) -> Self {
        MockInterviewer { llm }
    }
}

#[async_trait::async_trait]
impl AgentInterviewer for MockInterviewer {
    async fn interview(&self, agent_id: i64, prompt: &str) -> anyhow::Result<String> {
        let system = format!(
            "You are simulated social-media user #{agent_id}, an ordinary member of the public \
             with your own opinions. Answer the question in character, first person, in 2-4 \
             sentences. Do not mention being an AI or part of a simulation."
        );
        self.llm
            .chat(&[Msg::system(system), Msg::user(prompt)], 0.8, 512)
            .await
    }
}

/// Deterministic interviewer for tests: returns a canned answer per agent_id.
#[cfg(test)]
pub struct ScriptedInterviewer(pub std::collections::HashMap<i64, String>);

#[cfg(test)]
#[async_trait::async_trait]
impl AgentInterviewer for ScriptedInterviewer {
    async fn interview(&self, agent_id: i64, _prompt: &str) -> anyhow::Result<String> {
        self.0
            .get(&agent_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no scripted answer for agent {agent_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a configured LLM endpoint"]
    async fn mock_interviewer_answers() {
        let cfg = crate::config::Config::from_env();
        let llm = Llm::new(&cfg.llm_api_key, &cfg.llm_base_url, &cfg.llm_model);
        let mock = MockInterviewer::new(llm);
        let answer = mock.interview(1, "Do you support the new policy?").await.unwrap();
        assert!(!answer.is_empty());
    }
}
