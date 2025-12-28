# pubop: Real-World Simulation Prediction

## 🎯 Mission

```
Real Data  →  Simulation  →  Prediction
(scrape)      (AI agents)    (forecasts)
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           pubop                                 │
├─────────────┬─────────────────┬─────────────────────────────────┤
│ REAL DATA   │   SIMULATION    │         PREDICTION              │
├─────────────┼─────────────────┼─────────────────────────────────┤
│ LightPanda  │   OASIS         │   ReportAgent                   │
│     ↓       │       ↓         │         ↓                       │
│ Crawlers    │   Profiles      │   Real vs Simulated             │
│     ↓       │       ↓         │   Trend forecasts               │
│ Posts/Users │   Behaviors     │   Decision support              │
└─────────────┴─────────────────┴─────────────────────────────────┘
```

---

## File Structure

```
backend/app/
├── models/
│   └── pubop.py                 # Scraped data models
├── services/
│   ├── crawler/                 # NEW: LightPanda + scrapers
│   │   ├── __init__.py
│   │   ├── client.py            # CDP connection
│   │   ├── base.py              # Abstract interface
│   │   ├── telegram.py
│   │   ├── twitter.py
│   │   └── facebook.py
│   ├── pubop_bridge.py          # NEW: Real data → simulation
│   ├── oasis_profile_generator.py  # MOD: Add from_scraped_users()
│   └── simulation_config_generator.py  # MOD: Add real_data_seed
```

---

## Tasks

### Phase 1: Infrastructure [P0]

| Task | Hours | Status |
|------|-------|--------|
| Add LightPanda to docker-compose | 0.5 | TODO |
| Create `crawler/` module | 1 | TODO |
| Build CDP client | 2 | TODO |

### Phase 2: Crawlers [P0]

| Task | Hours | Status |
|------|-------|--------|
| Telegram crawler | 3 | TODO |
| X/Twitter crawler | 4 | TODO |
| Facebook crawler | 4 | TODO |

### Phase 3: Integration [P1]

| Task | Hours | Status |
|------|-------|--------|
| `pubop.py` models | 2 | TODO |
| `pubop_bridge.py` service | 3 | TODO |
| Modify `oasis_profile_generator.py` | 2 | TODO |
| Modify `simulation_config_generator.py` | 1 | TODO |

### Phase 4: Enhanced Prediction [P2]

| Task | Hours | Status |
|------|-------|--------|
| Real vs simulated comparison tools | 3 | TODO |
| Prediction confidence scores | 2 | TODO |

---

## Integration Points

1. **OasisProfileGenerator** → Add `generate_profiles_from_scraped_users()`
2. **SimulationConfigGenerator** → Add `initial_posts_from_real` parameter
3. **ReportAgent** → Add real vs simulated comparison tools

---

## Docker

```yaml
services:
  lightpanda:
    image: lightpanda/lightpanda:latest
    container_name: pubop-lightpanda
    ports:
      - "9222:9222"
```

---

## Platforms (P0)

| Platform | Method |
|----------|--------|
| Telegram | Web + Bot API |
| X/Twitter | Web scrape |
| Facebook | Graph API + Web |

---

**Total: ~27.5 hours**
