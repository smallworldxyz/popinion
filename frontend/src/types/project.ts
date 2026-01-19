// Project types
export interface Project {
  id: string
  name: string
  created_at: string
  updated_at: string
  status: 'draft' | 'prepared' | 'running' | 'completed'
  simulation_id?: string
  graph_id?: string
}

export interface ProjectListResponse {
  projects: Project[]
  total: number
}
