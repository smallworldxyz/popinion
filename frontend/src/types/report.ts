// Report types
export interface ReportData {
  status: 'pending' | 'running' | 'completed' | 'failed'
  impactScore: number
  totalReach: number
  avgSentiment: number
  totalEngagement: number
  twitterActions: number
  redditActions: number
}

export interface Influencer {
  id: number
  name: string
  handle: string
  reach: number
  actionCount: number
  sentiment: number
  onTwitter: boolean
  onReddit: boolean
}

export interface PlatformData {
  twitterSentiment: number
  redditSentiment: number
  timeline: { round: number; value: number }[]
  actions: {
    create_post: number
    like_post: number
    comment: number
    share: number
  }
}
