
"""
Simulation Process Management
Handles subprocess spawning, monitoring, and termination.
"""

import os
import sys
import signal
import atexit
import subprocess
import threading
from typing import Dict, Optional, List, Any
from ...utils.logger import get_logger

logger = get_logger('pubop.engine.process')

class ProcessManager:
    _processes: Dict[str, subprocess.Popen] = {}
    _stdout_files: Dict[str, Any] = {}
    _stderr_files: Dict[str, Any] = {}
    _cleanup_registered = False

    @classmethod
    def start_process(cls, simulation_id: str, script_path: str, config_path: str, cwd: str, max_rounds: int = None) -> int:
        """
        Start a simulation subprocess.
        Returns: PID
        """
        cmd = [
            sys.executable,
            script_path,
            "--config", config_path,
        ]
        if max_rounds and max_rounds > 0:
            cmd.extend(["--max-rounds", str(max_rounds)])
            
        main_log_path = os.path.join(cwd, "simulation.log")
        main_log_file = open(main_log_path, 'w', encoding='utf-8')
        
        # Start new session
        process = subprocess.Popen(
            cmd,
            cwd=cwd,
            stdout=main_log_file,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
            start_new_session=True,
        )
        
        cls._processes[simulation_id] = process
        cls._stdout_files[simulation_id] = main_log_file
        
        return process.pid

    @classmethod
    def get_process(cls, simulation_id: str) -> Optional[subprocess.Popen]:
        return cls._processes.get(simulation_id)

    @classmethod
    def cleanup_process_resources(cls, simulation_id: str):
        cls._processes.pop(simulation_id, None)
        
        if simulation_id in cls._stdout_files:
            try:
                cls._stdout_files[simulation_id].close()
            except: pass
            cls._stdout_files.pop(simulation_id, None)

    @classmethod
    def cleanup_all_simulations(cls):
        """Clean up all running processes on shutdown"""
        if not cls._processes:
            return
            
        logger.info("Cleaning up all simulation processes...")
        
        # Copy to avoid modification during iteration
        for simulation_id, process in list(cls._processes.items()):
            try:
                if process.poll() is None:
                    logger.info(f"Terminating process: {simulation_id}, pid={process.pid}")
                    try:
                        pgid = os.getpgid(process.pid)
                        os.killpg(pgid, signal.SIGTERM)
                        process.wait(timeout=5)
                    except:
                        try:
                            process.terminate()
                            process.wait(timeout=3)
                        except:
                            process.kill()
            except Exception as e:
                logger.error(f"Cleanup failed for {simulation_id}: {e}")
                
        # Close files
        for f in cls._stdout_files.values():
            try: f.close()
            except: pass
        
        cls._processes.clear()
        cls._stdout_files.clear()

    @classmethod
    def register_cleanup(cls):
        """Register signal handlers"""
        global _cleanup_registered
        if cls._cleanup_registered:
            return
            
        def cleanup_handler(signum=None, frame=None):
            cls.cleanup_all_simulations()
            # re-raise for default handling if needed, or exit
            # For simplicity, just exit
            sys.exit(0)

        atexit.register(cls.cleanup_all_simulations)
        
        try:
            signal.signal(signal.SIGTERM, cleanup_handler)
            signal.signal(signal.SIGINT, cleanup_handler)
        except ValueError:
            pass # Not in main thread
            
        cls._cleanup_registered = True

    @classmethod
    def cleanup_logs(cls, sim_dir: str) -> List[str]:
        """Delete log files for reset"""
        files_to_delete = [
            "run_state.json", "simulation.log", "stdout.log", "stderr.log",
            "twitter_simulation.db", "reddit_simulation.db", "env_status.json"
        ]
        cleaned = []
        
        if not os.path.exists(sim_dir):
            return cleaned

        for f in files_to_delete:
            p = os.path.join(sim_dir, f)
            if os.path.exists(p):
                try: 
                    os.remove(p)
                    cleaned.append(f)
                except: pass
                
        # Clean subdirs
        for sd in ["twitter", "reddit"]:
            p = os.path.join(sim_dir, sd, "actions.jsonl")
            if os.path.exists(p):
                try: 
                    os.remove(p)
                    cleaned.append(f"{sd}/actions.jsonl")
                except: pass
                
        return cleaned
