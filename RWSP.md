# pubop RWSP: Real-World Simulation Prediction

## 🎯 Mission

```
Real Data  →  Simulation  →  Prediction
(scrape)      (AI agents)    (forecasts)
```

Collect real social media data, simulate public behavior, predict trends.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         pubop RWSP                              │
├─────────────┬─────────────────┬─────────────────────────────────┤
│ REAL DATA   │   SIMULATION    │         PREDICTION              │
├─────────────┼─────────────────┼─────────────────────────────────┤
│ LightPanda  │   OASIS         │   ReportAgent                   │
│ (headless)  │   (multi-agent) │   (analysis)                    │
│     ↓       │       ↓         │         ↓                       │
│ Crawlers    │   Agent         │   Trend forecasts               │
│ (per-plat)  │   behaviors     │   Sentiment shifts              │
│     ↓       │       ↓         │   Viral predictions             │
│ Posts/Users │   Simulated     │   Decision support              │
│ Trends      │   interactions  │                                 │
└─────────────┴─────────────────┴─────────────────────────────────┘
```

---

## Implementation Tasks

### Phase 1: Infrastructure [P0]

| Task | Priority | Est. Hours | Status |
|------|----------|------------|--------|
| Add LightPanda to docker-compose.yml | P0 | 0.5 | TODO |
| Create `crawler/` module structure | P0 | 1 | TODO |
| Build LightPanda CDP client | P0 | 2 | TODO |
| Write connection tests | P0 | 1 | TODO |

**Files to create:**
```
backend/app/services/crawler/
├── __init__.py
├── client.py          # LightPanda CDP connection
└── base.py            # Abstract crawler interface
```

---

### Phase 2: Platform Crawlers [P0]

| Task | Priority | Est. Hours | Status |
|------|----------|------------|--------|
| Telegram channel crawler | P0 | 3 | TODO |
| X/Twitter scraper | P0 | 4 | TODO |
| Facebook public pages crawler | P0 | 4 | TODO |

**Files to create:**
```
backend/app/services/crawler/
├── telegram.py        # Telegram web scraper
├── twitter.py         # X/Twitter scraper
└── facebook.py        # Facebook Graph API + scrape
```

---

### Phase 3: Data Bridge [P1]

| Task | Priority | Est. Hours | Status |
|------|----------|------------|--------|
| Create data models for scraped content | P1 | 2 | TODO |
| Build bridge service (scraped → profiles) | P1 | 3 | TODO |
| Simulation seeding from real data | P1 | 3 | TODO |

**Files to create:**
```
backend/app/services/
├── rwsp_models.py     # Scraped data models
└── rwsp_bridge.py     # Real data → simulation bridge
```

---

### Phase 4: Enhanced Prediction [P2]

| Task | Priority | Est. Hours | Status |
|------|----------|------------|--------|
| Compare simulated vs real trends | P2 | 3 | TODO |
| Add prediction confidence scores | P2 | 2 | TODO |
| Real-time data refresh during simulation | P2 | 4 | TODO |

---

## Target Platforms

| Platform | Method | Priority | Notes |
|----------|--------|----------|-------|
| **Telegram** | Web + Bot API | P0 | Easiest, official API available |
| **X/Twitter** | Web scrape | P0 | No API access needed |
| **Facebook** | Graph API + Web | P0 | For public pages/groups |
| YouTube | Data API v3 | P1 | Comments & trends |
| Reddit | API | P2 | Already in OASIS |

---

## Tech Stack

- **Scraping**: LightPanda (self-hosted headless browser)
- **Protocol**: Chrome DevTools Protocol (CDP)
- **Driver**: Playwright/Puppeteer compatible
- **Simulation**: OASIS multi-agent framework
- **Reports**: ReportAgent with LLM

---

## Docker Setup

```yaml
# Add to docker-compose.yml
services:
  lightpanda:
    image: lightpanda/lightpanda:latest
    container_name: pubop-lightpanda
    ports:
      - "9222:9222"
    restart: unless-stopped
```

---

## Priority Legend

| Priority | Meaning |
|----------|---------|
| P0 | Critical - must have for MVP |
| P1 | Important - needed for full functionality |
| P2 | Nice to have - enhances experience |

---

## Estimated Total Effort

| Phase | Hours |
|-------|-------|
| Phase 1: Infrastructure | ~4.5 |
| Phase 2: Crawlers | ~11 |
| Phase 3: Data Bridge | ~8 |
| Phase 4: Prediction | ~9 |
| **Total** | **~32.5 hours** |

---

## Success Criteria

1. ✅ Can scrape real posts from Telegram, X, Facebook
2. ✅ Real data seeds agent profiles and initial posts
3. ✅ Simulation produces behavior based on real patterns
4. ✅ Reports compare predictions against real outcomes
