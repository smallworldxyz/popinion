<template>
  <div class="report-view">
    <div class="report-header">
      <h1>Intelligence Briefing</h1>
    </div>
    <div class="report-container">
      <div class="report-main-content">
        <ExecutiveSummary :reportData="summaryData" />
        <PlatformBreakdown :platformData="platformData" />
      </div>
      <div class="report-sidebar">
        <InfluencerCard v-for="influencer in topInfluencers" :key="influencer.id" :influencer="influencer" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import ExecutiveSummary from '../components/report/ExecutiveSummary.vue'
import InfluencerCard from '../components/report/InfluencerCard.vue'
import PlatformBreakdown from '../components/report/PlatformBreakdown.vue'

const summaryData = ref({ status: 'completed', impactScore: 87, totalReach: 1200000, avgSentiment: 0.62, totalEngagement: 45000, twitterActions: 28000, redditActions: 17000 })
const platformData = ref({ twitterSentiment: 0.68, redditSentiment: 0.54, timeline: [], actions: { create_post: 1200, like_post: 2800, comment: 1500, share: 900 } })
const topInfluencers = ref([{ id: 1, name: 'Alexandra Chen', handle: 'alexchen_policy', reach: 250000, actionCount: 48, sentiment: 0.85, onTwitter: true, onReddit: false }])
</script>

<style scoped>
@import './report_dossier.css';
</style>
