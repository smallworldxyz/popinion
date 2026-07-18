import { createRouter, createWebHistory } from 'vue-router'
import Home from '../views/Home.vue'
import WorldsView from '../views/WorldsView.vue'
import WorldView from '../views/WorldView.vue'
import FieldDemoView from '../views/FieldDemoView.vue'
import RunView from '../views/RunView.vue'
import PrepareView from '../views/PrepareView.vue'
import WorldShell from '../views/WorldShell.vue'
import SettingsView from '../views/SettingsView.vue'

// The five wizard surfaces (process → simulation → run → report → interaction)
// are altitudes of one shell; WorldShell renders the right one by route name.
const routes = [
  {
    path: '/',
    name: 'Home',
    component: Home
  },
  {
    path: '/worlds',
    name: 'Worlds',
    component: WorldsView
  },
  {
    path: '/field-demo',
    name: 'FieldDemo',
    component: FieldDemoView
  },
  {
    path: '/world/:graphId',
    name: 'World',
    component: WorldView,
    props: true
  },
  {
    path: '/world/:graphId/run/:simId',
    name: 'Run',
    component: RunView,
    props: true
  },
  {
    path: '/world/:graphId/prepare/:simId',
    name: 'Prepare',
    component: PrepareView,
    props: true
  },
  {
    path: '/process/:projectId',
    name: 'Process',
    component: WorldShell
  },
  {
    path: '/simulation/:simulationId',
    name: 'Simulation',
    component: WorldShell
  },
  {
    path: '/simulation/:simulationId/start',
    name: 'SimulationRun',
    component: WorldShell
  },
  {
    path: '/report/:reportId',
    name: 'Report',
    component: WorldShell
  },
  {
    path: '/interaction/:reportId',
    name: 'Interaction',
    component: WorldShell
  },
  {
    path: '/settings',
    name: 'Settings',
    component: SettingsView
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router
