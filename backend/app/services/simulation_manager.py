"""
OASIS Simulation Manager
Manages Twitter and Reddit dual-platform parallel simulation
Uses preset scripts + LLM intelligent generation of configuration parameters
"""

import os
import json
import shutil
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum

from ..config import Config
from ..utils.logger import get_logger
from .neo4j_entity_reader import Neo4jEntityReader, FilteredEntities
from .oasis_profile_generator import OasisProfileGenerator, OasisAgentProfile
from .simulation_config_generator import SimulationConfigGenerator, SimulationParameters

logger = get_logger('pubop.simulation')


class SimulationStatus(str, Enum):
    """Simulation status"""
    CREATED = "created"
    PREPARING = "preparing"
    READY = "ready"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPED = "stopped"      # Simulation manually stopped
    COMPLETED = "completed"  # Simulation naturally completed
    FAILED = "failed"


class PlatformType(str, Enum):
    """Platform type"""
    TWITTER = "twitter"
    REDDIT = "reddit"


@dataclass
class SimulationState:
    """Simulation state"""
    simulation_id: str
    project_id: str
    graph_id: str
    
    # Platform enable status
    enable_twitter: bool = True
    enable_reddit: bool = True
    
    # Status
    status: SimulationStatus = SimulationStatus.CREATED
    
    # Preparation phase data
    entities_count: int = 0
    profiles_count: int = 0
    entity_types: List[str] = field(default_factory=list)
    
    # Configuration generation info
    config_generated: bool = False
    config_reasoning: str = ""
    
    # Runtime data
    current_round: int = 0
    twitter_status: str = "not_started"
    reddit_status: str = "not_started"
    
    # Timestamps
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    updated_at: str = field(default_factory=lambda: datetime.now().isoformat())
    
    # Error information
    error: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Complete state dictionary (for internal use)"""
        return {
            "simulation_id": self.simulation_id,
            "project_id": self.project_id,
            "graph_id": self.graph_id,
            "enable_twitter": self.enable_twitter,
            "enable_reddit": self.enable_reddit,
            "status": self.status.value,
            "entities_count": self.entities_count,
            "profiles_count": self.profiles_count,
            "entity_types": self.entity_types,
            "config_generated": self.config_generated,
            "config_reasoning": self.config_reasoning,
            "current_round": self.current_round,
            "twitter_status": self.twitter_status,
            "reddit_status": self.reddit_status,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "error": self.error,
        }
    
    def to_simple_dict(self) -> Dict[str, Any]:
        """Simplified state dictionary (for API response)"""
        return {
            "simulation_id": self.simulation_id,
            "project_id": self.project_id,
            "graph_id": self.graph_id,
            "status": self.status.value,
            "entities_count": self.entities_count,
            "profiles_count": self.profiles_count,
            "entity_types": self.entity_types,
            "config_generated": self.config_generated,
            "error": self.error,
        }


class SimulationManager:
    """
    Simulation Manager
    
    Core features:
    1. Read entities from Neo4j graph and filter
    2. Generate OASIS Agent Profiles
    3. Use LLM to intelligently generate simulation configuration parameters
    4. Prepare all files required by preset scripts
    """
    
    # Simulation data storage directory
    SIMULATION_DATA_DIR = os.path.join(
        os.path.dirname(__file__), 
        '../../uploads/simulations'
    )
    
    def __init__(self):
        # Ensure directory exists
        os.makedirs(self.SIMULATION_DATA_DIR, exist_ok=True)
        
        # In-memory simulation state cache
        self._simulations: Dict[str, SimulationState] = {}
    
    def _get_simulation_dir(self, simulation_id: str) -> str:
        """Get simulation data directory"""
        sim_dir = os.path.join(self.SIMULATION_DATA_DIR, simulation_id)
        os.makedirs(sim_dir, exist_ok=True)
        return sim_dir
    
    def _save_simulation_state(self, state: SimulationState):
        """Save simulation state to file"""
        sim_dir = self._get_simulation_dir(state.simulation_id)
        state_file = os.path.join(sim_dir, "state.json")
        
        state.updated_at = datetime.now().isoformat()
        
        with open(state_file, 'w', encoding='utf-8') as f:
            json.dump(state.to_dict(), f, ensure_ascii=False, indent=2)
        
        self._simulations[state.simulation_id] = state
    
    def _load_simulation_state(self, simulation_id: str) -> Optional[SimulationState]:
        """Load simulation state from file"""
        if simulation_id in self._simulations:
            return self._simulations[simulation_id]
        
        sim_dir = self._get_simulation_dir(simulation_id)
        state_file = os.path.join(sim_dir, "state.json")
        
        if not os.path.exists(state_file):
            return None
        
        with open(state_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        state = SimulationState(
            simulation_id=simulation_id,
            project_id=data.get("project_id", ""),
            graph_id=data.get("graph_id", ""),
            enable_twitter=data.get("enable_twitter", True),
            enable_reddit=data.get("enable_reddit", True),
            status=SimulationStatus(data.get("status", "created")),
            entities_count=data.get("entities_count", 0),
            profiles_count=data.get("profiles_count", 0),
            entity_types=data.get("entity_types", []),
            config_generated=data.get("config_generated", False),
            config_reasoning=data.get("config_reasoning", ""),
            current_round=data.get("current_round", 0),
            twitter_status=data.get("twitter_status", "not_started"),
            reddit_status=data.get("reddit_status", "not_started"),
            created_at=data.get("created_at", datetime.now().isoformat()),
            updated_at=data.get("updated_at", datetime.now().isoformat()),
            error=data.get("error"),
        )
        
        self._simulations[simulation_id] = state
        return state
    
    def create_simulation(
        self,
        project_id: str,
        graph_id: str,
        enable_twitter: bool = True,
        enable_reddit: bool = True,
    ) -> SimulationState:
        """
        Create new simulation
        
        Args:
            project_id: Project ID
            graph_id: Neo4j graph ID
            enable_twitter: Whether to enable Twitter simulation
            enable_reddit: Whether to enable Reddit simulation
            
        Returns:
            SimulationState
        """
        import uuid
        simulation_id = f"sim_{uuid.uuid4().hex[:12]}"
        
        state = SimulationState(
            simulation_id=simulation_id,
            project_id=project_id,
            graph_id=graph_id,
            enable_twitter=enable_twitter,
            enable_reddit=enable_reddit,
            status=SimulationStatus.CREATED,
        )
        
        self._save_simulation_state(state)
        logger.info(f"Created simulation: {simulation_id}, project={project_id}, graph={graph_id}")
        
        return state
    
    def prepare_simulation(
        self,
        simulation_id: str,
        simulation_requirement: str,
        document_text: str,
        defined_entity_types: Optional[List[str]] = None,
        selected_entity_ids: Optional[List[str]] = None,
        use_llm_for_profiles: bool = True,
        progress_callback: Optional[callable] = None,
        parallel_profile_count: int = 3,
        enable_realworld_scraping: bool = True,
        realworld_max_posts: int = 30
    ) -> SimulationState:
        """
        Prepare simulation environment (fully automated)
        
        Steps:
        1. Read and filter entities from Neo4j graph
        1.5. Filter to user-selected entities (if specified)
        2. [NEW] Scrape real-world news/posts about entities (pubop integration)
        3. Generate OASIS Agent Profile for each entity (optional LLM enhancement, supports parallel)
        4. Use LLM to intelligently generate simulation configuration parameters (time, activity, post frequency, etc.)
        5. Inject real-world scraped data into configuration
        6. Save configuration files and Profile files
        
        Args:
            simulation_id: Simulation ID
            simulation_requirement: Simulation requirement description (for LLM configuration generation)
            document_text: Original document content (for LLM background understanding)
            defined_entity_types: Predefined entity types (optional)
            selected_entity_ids: List of entity UUIDs to include as agents (optional, if None uses all)
            use_llm_for_profiles: Whether to use LLM to generate detailed profiles
            progress_callback: Progress callback function (stage, progress, message)
            parallel_profile_count: Number of profiles to generate in parallel, default 3
            enable_realworld_scraping: Whether to scrape real news about entities (pubop feature)
            realworld_max_posts: Max posts to scrape in real-world phase
            
        Returns:
            SimulationState
        """
        state = self._load_simulation_state(simulation_id)
        if not state:
            raise ValueError(f"Simulation does not exist: {simulation_id}")
        
        try:
            state.status = SimulationStatus.PREPARING
            self._save_simulation_state(state)
            
            sim_dir = self._get_simulation_dir(simulation_id)
            
            # ========== Phase 1: Read and filter entities ==========
            if progress_callback:
                progress_callback("reading", 0, "Connecting to Neo4j graph...")
            
            reader = Neo4jEntityReader()
            
            if progress_callback:
                progress_callback("reading", 30, "Reading node data...")
            
            filtered = reader.filter_defined_entities(
                graph_id=state.graph_id,
                defined_entity_types=defined_entity_types,
                enrich_with_edges=True
            )
            
            state.entities_count = filtered.filtered_count
            state.entity_types = list(filtered.entity_types)
            
            if progress_callback:
                progress_callback(
                    "reading", 100, 
                    f"Completed, total {filtered.filtered_count} entities",
                    current=filtered.filtered_count,
                    total=filtered.filtered_count
                )
            
            if filtered.filtered_count == 0:
                state.status = SimulationStatus.FAILED
                state.error = "No matching entities found, please check if the graph was built correctly"
                self._save_simulation_state(state)
                return state
            
            # Store all entities for context scraping (before filtering)
            all_entities = filtered.entities.copy()
            all_entity_count = len(all_entities)
            
            # ========== Phase 2: Real-World Data Scraping (pubop) ==========
            # Scrape for ALL entities to provide rich context, not just selected ones
            real_data_seed = None
            
            if enable_realworld_scraping:
                if progress_callback:
                    progress_callback(
                        "scraping_realworld", 0,
                        "Starting smart real-world scraping...",
                        current=0,
                        total=all_entity_count
                    )
                
                try:
                    from .pubop_realworld_scraper import RealWorldScraper
                    import asyncio
                    
                    # Extract ALL entity names for scraping (not just selected)
                    entity_names = [e.name for e in all_entities]
                    
                    logger.info(f"Starting real-world scraping for {len(entity_names)} entities (full context)")
                    
                    def scrape_progress(current, total, msg):
                        if progress_callback:
                            progress_callback(
                                "scraping_realworld",
                                int(current / total * 100),
                                msg,
                                current=current,
                                total=total
                            )
                    
                    # Run async scraping
                    scraper = RealWorldScraper()
                    loop = asyncio.new_event_loop()
                    try:
                        real_data_seed = loop.run_until_complete(
                            scraper.scrape_entity_names(
                                entity_names,
                                simulation_requirement=simulation_requirement,
                                document_text=document_text,
                                max_total_posts=realworld_max_posts,
                                progress_callback=scrape_progress
                            )
                        )
                    finally:
                        loop.close()
                    
                    if progress_callback:
                        progress_callback(
                            "scraping_realworld", 100,
                            f"Smart Scraped {len(real_data_seed.initial_posts)} real articles",
                            current=all_entity_count,
                            total=all_entity_count
                        )
                    
                    logger.info(
                        f"Real-world scraping complete: {len(real_data_seed.initial_posts)} articles, "
                        f"{len(real_data_seed.trending_topics)} topics"
                    )
                    
                except Exception as e:
                    logger.warning(f"Real-world scraping failed (continuing without): {e}")
                    if progress_callback:
                        progress_callback(
                            "scraping_realworld", 100,
                            f"Scraping skipped: {str(e)[:50]}",
                            current=all_entity_count,
                            total=all_entity_count
                        )
            
            # ========== Phase 2.5: Filter to Selected Entities ==========
            # After scraping ALL entities for context, now filter to only selected ones for profile generation
            if selected_entity_ids:
                selected_set = set(selected_entity_ids)
                original_count = len(filtered.entities)
                filtered.entities = [
                    e for e in filtered.entities 
                    if e.uuid in selected_set
                ]
                filtered.filtered_count = len(filtered.entities)
                state.entities_count = filtered.filtered_count
                
                if progress_callback:
                    progress_callback(
                        "filtering", 100,
                        f"Selected {filtered.filtered_count} of {original_count} entities as active agents",
                        current=filtered.filtered_count,
                        total=original_count
                    )
                
                logger.info(f"Filtered to {filtered.filtered_count} selected entities for profile generation (scraped {original_count} for context)")
                
                if filtered.filtered_count == 0:
                    state.status = SimulationStatus.FAILED
                    state.error = "No selected entities found in graph"
                    self._save_simulation_state(state)
                    return state
            
            # ========== Phase 3: Generate Agent Profiles ==========
            total_entities = len(filtered.entities)
            
            if progress_callback:
                progress_callback(
                    "generating_profiles", 0, 
                    "Starting generation...",
                    current=0,
                    total=total_entities
                )
            
            # Pass graph_id to enable Neo4j retrieval function for richer context
            generator = OasisProfileGenerator(graph_id=state.graph_id)
            
            def profile_progress(current, total, msg):
                if progress_callback:
                    progress_callback(
                        "generating_profiles", 
                        int(current / total * 100), 
                        msg,
                        current=current,
                        total=total,
                        item_name=msg
                    )
            
            # Set realtime save file path (prefer Reddit JSON format)
            realtime_output_path = None
            realtime_platform = "reddit"
            if state.enable_reddit:
                realtime_output_path = os.path.join(sim_dir, "reddit_profiles.json")
                realtime_platform = "reddit"
            elif state.enable_twitter:
                realtime_output_path = os.path.join(sim_dir, "twitter_profiles.csv")
                realtime_platform = "twitter"
            
            profiles = generator.generate_profiles_from_entities(
                entities=filtered.entities,
                use_llm=use_llm_for_profiles,
                progress_callback=profile_progress,
                graph_id=state.graph_id,  # Pass graph_id for Neo4j retrieval
                parallel_count=parallel_profile_count,  # Parallel generation count
                realtime_output_path=realtime_output_path,  # Realtime save path
                output_platform=realtime_platform  # Output format
            )
            
            state.profiles_count = len(profiles)
            
            # Save Profile files (Note: Twitter uses CSV format, Reddit uses JSON format)
            # Reddit already saved in real-time during generation, save again here for completeness
            if progress_callback:
                progress_callback(
                    "generating_profiles", 95, 
                    "Saving Profile files...",
                    current=total_entities,
                    total=total_entities
                )
            
            if state.enable_reddit:
                generator.save_profiles(
                    profiles=profiles,
                    file_path=os.path.join(sim_dir, "reddit_profiles.json"),
                    platform="reddit"
                )
            
            if state.enable_twitter:
                # Twitter uses CSV format! This is OASIS's requirement
                generator.save_profiles(
                    profiles=profiles,
                    file_path=os.path.join(sim_dir, "twitter_profiles.csv"),
                    platform="twitter"
                )
            
            if progress_callback:
                progress_callback(
                    "generating_profiles", 100, 
                    f"Completed, total {len(profiles)} Profiles",
                    current=len(profiles),
                    total=len(profiles)
                )
            
            # ========== Phase 3: LLM intelligent configuration generation ==========
            if progress_callback:
                progress_callback(
                    "generating_config", 0, 
                    "Analyzing simulation requirements...",
                    current=0,
                    total=3
                )
            
            config_generator = SimulationConfigGenerator()
            
            if progress_callback:
                progress_callback(
                    "generating_config", 30, 
                    "Calling LLM to generate configuration...",
                    current=1,
                    total=3
                )
            
            sim_params = config_generator.generate_config(
                simulation_id=simulation_id,
                project_id=state.project_id,
                graph_id=state.graph_id,
                simulation_requirement=simulation_requirement,
                document_text=document_text,
                entities=filtered.entities,
                enable_twitter=state.enable_twitter,
                enable_reddit=state.enable_reddit
            )
            
            # ========== Phase 5: Inject Real-World Data (pubop) ==========
            if real_data_seed and len(real_data_seed.initial_posts) > 0:
                if progress_callback:
                    progress_callback(
                        "generating_config", 60,
                        f"Injecting {len(real_data_seed.initial_posts)} real articles...",
                        current=2,
                        total=4
                    )
                
                logger.info(
                    f"Injecting real data: {len(real_data_seed.initial_posts)} posts, "
                    f"{len(real_data_seed.trending_topics)} topics"
                )
                
                sim_params = config_generator.inject_real_data_seed(
                    params=sim_params,
                    initial_posts_from_real=real_data_seed.initial_posts,
                    trending_topics=real_data_seed.trending_topics,
                    merge_mode="prepend"
                )
                
                logger.info(
                    f"After injection: {len(sim_params.event_config.initial_posts)} total posts, "
                    f"{len(sim_params.event_config.hot_topics)} hot topics"
                )
            
            if progress_callback:
                progress_callback(
                    "generating_config", 70, 
                    "Saving configuration file...",
                    current=3,
                    total=4
                )
            
            # Save configuration file
            config_path = os.path.join(sim_dir, "simulation_config.json")
            with open(config_path, 'w', encoding='utf-8') as f:
                f.write(sim_params.to_json())
            
            state.config_generated = True
            state.config_reasoning = sim_params.generation_reasoning
            
            if progress_callback:
                progress_callback(
                    "generating_config", 100, 
                    "Configuration generation completed",
                    current=3,
                    total=3
                )
            
            # Note: Running scripts remain in backend/scripts/ directory, no longer copied to simulation directory
            # When starting simulation, simulation_runner will run scripts from scripts/ directory
            
            # Update status
            state.status = SimulationStatus.READY
            self._save_simulation_state(state)
            
            logger.info(f"Simulation preparation completed: {simulation_id}, "
                       f"entities={state.entities_count}, profiles={state.profiles_count}")
            
            return state
            
        except Exception as e:
            logger.error(f"Simulation preparation failed: {simulation_id}, error={str(e)}")
            import traceback
            logger.error(traceback.format_exc())
            state.status = SimulationStatus.FAILED
            state.error = str(e)
            self._save_simulation_state(state)
            raise
    
    def get_simulation(self, simulation_id: str) -> Optional[SimulationState]:
        """Get simulation state"""
        return self._load_simulation_state(simulation_id)
    
    def list_simulations(self, project_id: Optional[str] = None) -> List[SimulationState]:
        """List all simulations"""
        simulations = []
        
        if os.path.exists(self.SIMULATION_DATA_DIR):
            for sim_id in os.listdir(self.SIMULATION_DATA_DIR):
                state = self._load_simulation_state(sim_id)
                if state:
                    if project_id is None or state.project_id == project_id:
                        simulations.append(state)
        
        return simulations
    
    def get_profiles(self, simulation_id: str, platform: str = "reddit") -> List[Dict[str, Any]]:
        """Get simulation Agent Profiles"""
        state = self._load_simulation_state(simulation_id)
        if not state:
            raise ValueError(f"Simulation does not exist: {simulation_id}")
        
        sim_dir = self._get_simulation_dir(simulation_id)
        profile_path = os.path.join(sim_dir, f"{platform}_profiles.json")
        
        if not os.path.exists(profile_path):
            return []
        
        with open(profile_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def get_simulation_config(self, simulation_id: str) -> Optional[Dict[str, Any]]:
        """Get simulation configuration"""
        sim_dir = self._get_simulation_dir(simulation_id)
        config_path = os.path.join(sim_dir, "simulation_config.json")
        
        if not os.path.exists(config_path):
            return None
        
        with open(config_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def get_run_instructions(self, simulation_id: str) -> Dict[str, str]:
        """Get run instructions"""
        sim_dir = self._get_simulation_dir(simulation_id)
        config_path = os.path.join(sim_dir, "simulation_config.json")
        scripts_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../scripts'))
        
        return {
            "simulation_dir": sim_dir,
            "scripts_dir": scripts_dir,
            "config_file": config_path,
            "commands": {
                "twitter": f"python {scripts_dir}/run_twitter_simulation.py --config {config_path}",
                "reddit": f"python {scripts_dir}/run_reddit_simulation.py --config {config_path}",
                "parallel": f"python {scripts_dir}/run_parallel_simulation.py --config {config_path}",
            },
            "instructions": (
                f"1. Activate conda environment: conda activate pubop\n"
                f"2. Run simulation (scripts located at {scripts_dir}):\n"
                f"   - Run Twitter only: python {scripts_dir}/run_twitter_simulation.py --config {config_path}\n"
                f"   - Run Reddit only: python {scripts_dir}/run_reddit_simulation.py --config {config_path}\n"
                f"   - Run dual platform parallel: python {scripts_dir}/run_parallel_simulation.py --config {config_path}"
            )
        }
