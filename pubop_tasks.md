# pubop: Global Public Opinion Analysis Platform

## 🎯 Mission

```
Real Data  →  Simulation  →  Prediction
```

---

## Architecture

```
Browserless Chrome (CDP) → Crawlers → pubop_bridge → Simulation → Reports
```

---

## File Structure

```
backend/app/
├── models/
│   └── pubop.py                    # Data models (with persistence)
├── services/
│   ├── crawler/
│   │   ├── __init__.py
│   │   ├── client.py               # CDP client + ProxyConfig
│   │   ├── base.py                 # Abstract BaseCrawler
│   │   ├── telegram.py
│   │   ├── twitter.py
│   │   └── facebook.py
│   ├── pubop_bridge.py             # Scraped → Simulation bridge
│   ├── pubop_comparison.py         # Prediction comparison tools
│   ├── oasis_profile_generator.py  # MOD: generate_profiles_from_scraped_users()
│   ├── simulation_config_generator.py  # MOD: inject_real_data_seed()
│   └── report_agent.py             # MOD: compare_predictions tool
├── api/
│   └── crawl.py                    # Crawl API endpoints
scripts/
└── pubop_demo.py                   # CLI demo script

docker-compose.yml                  # Browserless Chrome service
backend/.env.example                # Environment configuration
```

---

## Docker Setup

```yaml
services:
  chrome:
    image: browserless/chrome:latest
    container_name: pubop-chrome
    ports:
      - "9222:3000"
    environment:
      - CONNECTION_TIMEOUT=300000
      - MAX_CONCURRENT_SESSIONS=5
    restart: unless-stopped
```

---

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/crawl/telegram` | Crawl Telegram channel |
| POST | `/api/crawl/twitter` | Crawl Twitter search/profile |
| POST | `/api/crawl/facebook` | Crawl Facebook page |
| GET | `/api/crawl/results` | List saved crawl results |
| GET | `/api/crawl/results/<file>` | Load saved result |
| POST | `/api/crawl/bridge` | Convert crawl → simulation seed |

---

## CLI Usage

```bash
# Crawl Telegram
python scripts/pubop_demo.py telegram --channel durov --max-posts 20

# Crawl Twitter
python scripts/pubop_demo.py twitter --query "AI news" --max-posts 20

# Full pipeline: crawl → bridge → summary
python scripts/pubop_demo.py full --platform telegram --channel durov
```

---

## Proxy Configuration

```bash
# In .env
CRAWLER_PROXY_SERVER=http://proxy:8080
CRAWLER_PROXY_POOL=http://p1:8080,http://p2:8080
```

```python
from app.services.crawler import ProxyConfig, LightPandaClient

proxy = ProxyConfig.from_env()
async with LightPandaClient(proxy=proxy) as client:
    ...
```

---

## Tasks

### Phase 1: Infrastructure [P0] - 3.5h ✅
- [x] Add Browserless Chrome to docker-compose.yml
- [x] Create `backend/app/models/pubop.py`
- [x] Create `backend/app/services/crawler/__init__.py`
- [x] Create `backend/app/services/crawler/client.py`
- [x] Create `backend/app/services/crawler/base.py`

### Phase 2: Crawlers [P0] - 11h ✅
- [x] Create `telegram.py` crawler
- [x] Create `twitter.py` crawler  
- [x] Create `facebook.py` crawler

### Phase 3: Integration [P1] - 6h ✅
- [x] Create `pubop_bridge.py`
- [x] Add `generate_profiles_from_scraped_users()` to `oasis_profile_generator.py`
- [x] Add `inject_real_data_seed()` to `simulation_config_generator.py`

### Phase 4: Prediction [P1] - 5h ✅
- [x] Create `pubop_comparison.py` with `PubopComparisonTools`
- [x] Add `compare_predictions` tool to ReportAgent
- [x] Add `set_real_data_seed()` method to ReportAgent

### Phase 5: Production Ready [P1] - 3h ✅
- [x] Add persistence methods to `CrawlResult` (to_json, save, load)
- [x] Create crawl API endpoints (`/api/crawl/`)
- [x] Create CLI demo script (`scripts/pubop_demo.py`)
- [x] Add proxy rotation support (`ProxyConfig`)
- [x] Create `.env.example` with all config options

---

**Total: ~28.5 hours** ✅ ALL PHASES COMPLETE

**52 tests passing**

