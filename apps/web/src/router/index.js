import { createRouter, createWebHistory } from 'vue-router'
import Home from '../views/Home.vue'
import Process from '../views/MainView.vue'
import WorldsView from '../views/WorldsView.vue'
import WorldView from '../views/WorldView.vue'
import FieldDemoView from '../views/FieldDemoView.vue'
import RunView from '../views/RunView.vue'
import SimulationView from '../views/SimulationView.vue'
import SimulationRunView from '../views/SimulationRunView.vue'
import ReportView from '../views/ReportView.vue'
import InteractionView from '../views/InteractionView.vue'
import SettingsView from '../views/SettingsView.vue'

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
    path: '/process/:projectId',
    name: 'Process',
    component: Process,
    props: true
  },
  {
    path: '/simulation/:simulationId',
    name: 'Simulation',
    component: SimulationView,
    props: true
  },
  {
    path: '/simulation/:simulationId/start',
    name: 'SimulationRun',
    component: SimulationRunView,
    props: true
  },
  {
    path: '/report/:reportId',
    name: 'Report',
    component: ReportView,
    props: true
  },
  {
    path: '/interaction/:reportId',
    name: 'Interaction',
    component: InteractionView,
    props: true
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
