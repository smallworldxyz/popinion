// Simulation types
export interface SimulationConfig {
  simulation_id: string
  num_agents: number
  platforms: ('twitter' | 'reddit')[]
  simulation_duration_hours: number
  rounds_per_hour: number
}

export interface SimulationState {
  simulation_id: string
  runner_status: 'idle' | 'starting' | 'running' | 'paused' | 'stopped' | 'completed' | 'failed'
  current_round: number
  total_rounds: number
  twitter_running: boolean
  reddit_running: boolean
  twitter_completed: boolean
  reddit_completed: boolean
  error?: string
}

export interface AgentAction {
  round_num: number
  timestamp: string
  platform: 'twitter' | 'reddit'
  agent_id: number
  agent_name: string
  action_type: string
  action_args: Record<string, unknown>
  result?: string
  success: boolean
}
