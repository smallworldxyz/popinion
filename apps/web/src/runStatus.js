/**
 * One place for the backend run-status strings and the labels users see.
 * Two vocabularies, deliberately: the run's stored lifecycle, and the live
 * engine's liveness while a run is in flight.
 */

// The run's lifecycle status, not /run-status's liveness - a finished run is
// "not_running", which read as "Ready" on the run page while the World page
// said "Completed" for the same run. Same source, same labels, one answer.
// "alive"/"completed" both mean the run finished; "prepared" is
// ready-but-never-run.
const LIFECYCLE = {
  running: { label: 'Running', cls: 'running' },
  initializing: { label: 'Running', cls: 'running' },
  alive: { label: 'Completed', cls: 'completed' },
  completed: { label: 'Completed', cls: 'completed' },
  stopped: { label: 'Stopped', cls: 'stopped' },
  prepared: { label: 'Ready', cls: 'ready' },
}

export const runLifecycleStatus = (status) => {
  const s = status || ''
  if (s.startsWith('error')) return { label: 'Failed', cls: 'failed' }
  return LIFECYCLE[s] || { label: 'Draft', cls: 'draft' }
}

// What /run-status reports about a run that is currently attached to the
// engine — finer grained than the lifecycle, and only meaningful while live.
const LIVE = {
  initializing: { label: 'Initializing…', cls: 'run' },
  running: { label: 'Running', cls: 'run' },
  alive: { label: 'Complete — agents standing by', cls: 'done' },
  stopped: { label: 'Stopped', cls: 'done' },
  not_running: { label: 'Not running', cls: 'idle' },
}

export const liveRunStatus = (status) => LIVE[status] || { label: status, cls: 'err' }
