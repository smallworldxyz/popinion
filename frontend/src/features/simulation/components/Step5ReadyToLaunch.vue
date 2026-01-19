<template>
  <div class="step-card" :class="{ 'active': phase === 4 }">
    <div class="card-header">
      <div class="step-info">
        <span class="step-num">05</span>
        <span class="step-title">Ready to Launch</span>
      </div>
    </div>

    <div class="card-content">
      <div class="launch-actions">
        <!-- Duration Slider -->
        <div class="round-config">
           <label>Detailed Simulation Rounds</label>
           <input type="range" min="10" max="100" />
           <span>{{ customMaxRounds }} rounds (approx {{ (customMaxRounds * 30) / 60 }} hours)</span>
        </div>

        <button class="launch-btn" @click="$emit('launch')">
          🚀 Run Simulation
        </button>
      </div>
      
      <p v-if="error" class="error-msg">Error: {{ error }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import '../styles/envSetup.css';

defineProps<{
  phase: number
  customMaxRounds: number
  error?: string
}>();

defineEmits(['launch']);
</script>

<style scoped>
.launch-actions {
  display: flex;
  flex-direction: column;
  gap: 16px;
  align-items: center;
}

.launch-btn {
  background: linear-gradient(135deg, var(--primary) 0%, #FF9800 100%);
  color: white;
  border: none;
  padding: 16px 48px;
  border-radius: 30px;
  font-size: 18px;
  font-weight: 700;
  cursor: pointer;
  box-shadow: 0 4px 15px rgba(var(--primary-rgb), 0.4);
  transition: all 0.3s;
}

.launch-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(var(--primary-rgb), 0.5);
}

.error-msg {
  color: #ff4d4d;
  margin-top: 16px;
  text-align: center;
}
</style>
