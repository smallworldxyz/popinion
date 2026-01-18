# Popinion

> **Popinion** (Public Opinion) — Don't guess the future. Rehearse it.


---

## ⚡ Overview

**Popinion** is an AI prediction engine. It creates a "Digital Mirror World" — a simulation of your society where thousands of AI agents react to your ideas. Test a policy, a message, or a strategy here first. See how the world might react. Then act with confidence.

### Core Capabilities

| Feature | Description |
|---------|-------------|
| **Smart Scraping** | AI-driven search planning with specialized crawlers (Telegram, News, etc.) |
| **Swarm Simulation** | Thousands of autonomous agents with memory, personality, and social behavior |
| **Rehearsal Engine** | **Director Console** for manual event injection, live context search, and scenario scripting |
| **Agent Interrogation** | **The Green Room** for 1-on-1 agent interviews and **Session Replay** for historical critique |
| **Prediction Reports** | Deep analysis with agent interviews and trend forecasting |

> **Input:** Upload seed materials (news, reports, social data) + describe your prediction scenario  
> **Output:** A detailed prediction report + an interactive digital world you can query

### Our Vision

Popinion enables **High-Stakes Social Wargaming**:

- **Policy Crash Testing**: Test laws and announcements before they hit headlines
- **Diplomatic Save Game**: Simulate negotiation outcomes before entering the room
- **Cognitive Vaccination**: Pre-test counter-narratives against disinformation

From serious predictions to playful simulations, we let every "what if" see its outcome.

## 🔄 Workflow

1. **Graph Building**: Seed extraction → AI Search Planning → Smart Scraping (Telegram, News, etc.) → GraphRAG construction
2. **Environment Setup**: Entity extraction → Persona generation → **Scripted Scenarios** setup
3. **Simulation**: Dual-platform (Twitter + Reddit) parallel evolution with dynamic memory
4. **Rehearsal & Intervention**: Use **Director Console** to inject events, pause/resume, or enter **The Green Room**
5. **Replay & Critique**: Review session history, scrub through rounds, and annotate critical moments
6. **Report Generation**: ReportAgent with Deep Insight, Panorama Search, and Agent Interview tools

## 🎬 Rehearsal Engine: The Director's Suite

Popinion goes beyond passive simulation. You are the Director.

### 1. Scripted Scenarios
Define precognitive branches before the simulation starts.
- **Trigger Events**: Schedule "October Surprises" or "Market Crashes" to happen at specific rounds.
- **Conditional Logic**: "If Trump approval > 50%, inject Scandal B."

### 2. Director Console
Control the simulation involved in real-time.
- **Reality Injection**: Insert a breaking news event or social trend instantly to see agent reactions.
- **Live Context Search**: Is the simulation drifting? Search the real web and inject missing context immediately.

### 3. The Green Room (Interrogation)
Pull any agent out of the simulation for a private 1-on-1 interview.
- **Ask "Why?"**: "Why did you like that post?" "What is your true stance on Policy X?"
- **Assess Drift**: Verify if the agent is staying true to its persona.

### 4. Replay & Critique
Don't just watch it once. Scrub through history.
- **Session Replay**: Interactive timeline scrubber.
- **Annotation**: Add notes and critiques to specific actions (e.g., "Agent 42 broke character here").

## 🚀 Quick Start

### Prerequisites

> Note: Popinion was developed and tested on Mac and Linux. Windows compatibility is experimental.

| Tool | Version | Description | Check Installation |
|------|---------|-------------|-------------------|
| **Bun** | Latest | Fast JavaScript runtime & package manager | `bun -v` |
| **Python** | 3.11+ | Backend runtime | `python --version` |
| **uv** | Latest | Python package manager | `uv --version` |
| **Docker** | Latest | For Neo4j database | `docker --version` |

### 1. Start Neo4j Database

```bash
# Start Neo4j using Docker Compose
docker-compose up -d

# Neo4j Browser available at: http://localhost:7474
# Default credentials: neo4j / pubop123
```

### 2. Configure Environment Variables

```bash
# Copy the example configuration file
cp .env.example .env

# Edit the .env file and fill in the required API keys
```

**Required Environment Variables:**

```env
# LLM API Configuration (supports any LLM with OpenAI SDK format)
# Recommended: Use a capable model like GPT-4, Claude, or Qwen
LLM_API_KEY=your_api_key
LLM_BASE_URL=https://api.openai.com/v1
LLM_MODEL_NAME=gpt-4

# Neo4j Graph Database Configuration
# Use Docker Compose to start local Neo4j: docker-compose up -d
NEO4J_URI=bolt://localhost:7687
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=pubop123
NEO4J_DATABASE=neo4j
```

### 3. Install Dependencies

```bash
# One-click installation of all dependencies (root + frontend + backend)
bun run setup:all
```

Or install step by step:

```bash
# Install dependencies (root + frontend)
bun install

# Install Python dependencies (auto-creates virtual environment)
bun run setup:backend
```

### 4. Start Services

```bash
# Start both frontend and backend (run from project root)
bun dev
```

**Service URLs:**
- Frontend: `http://localhost:3000`
- Backend API: `http://localhost:5001`

**Start Individually:**

```bash
bun run backend   # Start backend only
bun run frontend  # Start frontend only
```

## 🛠️ Tech Stack

- **Frontend**: Vue 3 + Vite + TypeScript
- **Backend**: Python + Flask
- **Database**: Neo4j (Graph Database)
- **Simulation**: OASIS (Open Agent Social Interaction Simulations)
- **LLM Integration**: OpenAI-compatible API

## 📄 Acknowledgments

Popinion stands on the shoulders of giants:

| Project | Contribution |
|---------|--------------|
| **[OASIS](https://github.com/camel-ai/oasis)** | Core multi-agent simulation engine (by [CAMEL-AI](https://github.com/camel-ai)) |
| **[MiroFish](https://github.com/rithythul/mirofish)** | Social simulation architecture & workflow design |
| **[BettaFish](https://github.com/rithythul/bettafish)** | Real-world data extraction & scraping framework |

We are grateful to these open-source communities for making Popinion possible.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.