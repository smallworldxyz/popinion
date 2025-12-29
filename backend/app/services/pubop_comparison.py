"""
RWSP Comparison Tools
Tools for comparing simulation predictions with real-world data

These tools enable the ReportAgent to:
1. Compare simulation predictions with actual scraped data
2. Calculate accuracy metrics
3. Identify prediction hits and misses
4. Generate prediction validation reports
"""

from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field
from datetime import datetime
import json

from ..models.pubop import ScrapedPost, ScrapedUser, ScrapedTrend, CrawlResult
from ..utils.logger import get_logger

logger = get_logger('pubop.comparison')


@dataclass
class ComparisonMetrics:
    """Metrics for simulation vs reality comparison"""
    
    # Topic overlap
    topic_overlap_score: float = 0.0  # 0-1 Jaccard similarity
    matched_topics: List[str] = field(default_factory=list)
    simulation_only_topics: List[str] = field(default_factory=list)
    reality_only_topics: List[str] = field(default_factory=list)
    
    # Sentiment comparison
    simulation_sentiment: float = 0.0  # -1 to 1
    reality_sentiment: float = 0.0  # -1 to 1
    sentiment_accuracy: float = 0.0  # 0-1
    
    # Engagement prediction
    predicted_engagement: float = 0.0
    actual_engagement: float = 0.0
    engagement_accuracy: float = 0.0  # 0-1
    
    # Entity mentions
    correctly_predicted_entities: List[str] = field(default_factory=list)
    missed_entities: List[str] = field(default_factory=list)
    
    # Overall score
    overall_accuracy: float = 0.0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "topic_overlap_score": round(self.topic_overlap_score, 3),
            "matched_topics": self.matched_topics,
            "simulation_only_topics": self.simulation_only_topics,
            "reality_only_topics": self.reality_only_topics,
            "simulation_sentiment": round(self.simulation_sentiment, 3),
            "reality_sentiment": round(self.reality_sentiment, 3),
            "sentiment_accuracy": round(self.sentiment_accuracy, 3),
            "predicted_engagement": round(self.predicted_engagement, 2),
            "actual_engagement": round(self.actual_engagement, 2),
            "engagement_accuracy": round(self.engagement_accuracy, 3),
            "correctly_predicted_entities": self.correctly_predicted_entities,
            "missed_entities": self.missed_entities,
            "overall_accuracy": round(self.overall_accuracy, 3),
        }
    
    def to_text(self) -> str:
        """Generate human-readable comparison report"""
        lines = [
            "═══════════════════════════════════════════",
            "📊 SIMULATION vs REALITY COMPARISON",
            "═══════════════════════════════════════════",
            "",
            f"🎯 Overall Accuracy: {self.overall_accuracy:.1%}",
            "",
            "📌 TOPIC ANALYSIS",
            f"  • Topic Overlap Score: {self.topic_overlap_score:.1%}",
            f"  • Matched Topics: {', '.join(self.matched_topics[:5]) or 'None'}",
            f"  • Simulation-only: {', '.join(self.simulation_only_topics[:3]) or 'None'}",
            f"  • Reality-only: {', '.join(self.reality_only_topics[:3]) or 'None'}",
            "",
            "💭 SENTIMENT ANALYSIS",
            f"  • Predicted Sentiment: {self._sentiment_label(self.simulation_sentiment)}",
            f"  • Actual Sentiment: {self._sentiment_label(self.reality_sentiment)}",
            f"  • Sentiment Accuracy: {self.sentiment_accuracy:.1%}",
            "",
            "📈 ENGAGEMENT",
            f"  • Predicted: {self.predicted_engagement:.0f}",
            f"  • Actual: {self.actual_engagement:.0f}",
            f"  • Accuracy: {self.engagement_accuracy:.1%}",
            "",
            "👥 ENTITY PREDICTIONS",
            f"  • Correctly Predicted: {len(self.correctly_predicted_entities)}",
            f"  • Missed: {len(self.missed_entities)}",
        ]
        return "\n".join(lines)
    
    def _sentiment_label(self, score: float) -> str:
        if score > 0.3:
            return f"Positive ({score:.2f})"
        elif score < -0.3:
            return f"Negative ({score:.2f})"
        return f"Neutral ({score:.2f})"


@dataclass
class PredictionValidation:
    """Validation result for a specific prediction"""
    prediction: str
    evidence: str
    validated: bool
    confidence: float  # 0-1
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "prediction": self.prediction,
            "evidence": self.evidence,
            "validated": self.validated,
            "confidence": round(self.confidence, 2),
        }


class PubopComparisonTools:
    """
    Tools for comparing OASIS simulation predictions with real-world data.
    
    Used by ReportAgent to validate predictions and generate accuracy metrics.
    
    Example:
        tools = PubopComparisonTools()
        
        # Compare simulation with real crawl
        metrics = tools.compare_with_real_data(
            simulation_topics=["topic1", "topic2"],
            simulation_posts=[...],
            real_crawl=crawl_result
        )
        
        print(metrics.to_text())
    """
    
    # Simple sentiment keywords (for lightweight analysis)
    POSITIVE_KEYWORDS = [
        'great', 'good', 'excellent', 'amazing', 'wonderful', 'fantastic',
        'love', 'happy', 'excited', 'support', 'agree', 'success', 'win'
    ]
    NEGATIVE_KEYWORDS = [
        'bad', 'terrible', 'awful', 'horrible', 'hate', 'angry', 'sad',
        'worried', 'concerned', 'fail', 'wrong', 'problem', 'issue'
    ]
    
    def compare_with_real_data(
        self,
        simulation_topics: List[str],
        simulation_posts: List[Dict[str, Any]],
        real_crawl: CrawlResult,
        simulation_entities: Optional[List[str]] = None,
    ) -> ComparisonMetrics:
        """
        Compare simulation predictions with real crawled data.
        
        Args:
            simulation_topics: Hot topics from simulation
            simulation_posts: Posts generated in simulation
            real_crawl: Real data from crawler
            simulation_entities: Key entities mentioned in simulation
            
        Returns:
            ComparisonMetrics with accuracy scores
        """
        metrics = ComparisonMetrics()
        
        # 1. Topic comparison
        real_topics = self._extract_topics_from_posts(real_crawl.posts)
        real_topics.update([t.topic for t in real_crawl.trends])
        
        sim_topics_set = set(t.lower() for t in simulation_topics)
        real_topics_set = set(t.lower() for t in real_topics)
        
        if sim_topics_set or real_topics_set:
            intersection = sim_topics_set & real_topics_set
            union = sim_topics_set | real_topics_set
            metrics.topic_overlap_score = len(intersection) / len(union) if union else 0
            metrics.matched_topics = list(intersection)[:10]
            metrics.simulation_only_topics = list(sim_topics_set - real_topics_set)[:5]
            metrics.reality_only_topics = list(real_topics_set - sim_topics_set)[:5]
        
        # 2. Sentiment comparison
        metrics.simulation_sentiment = self._analyze_sentiment_batch(
            [p.get("content", "") for p in simulation_posts]
        )
        metrics.reality_sentiment = self._analyze_sentiment_batch(
            [p.content for p in real_crawl.posts]
        )
        
        # Sentiment accuracy: 1 - normalized difference
        sentiment_diff = abs(metrics.simulation_sentiment - metrics.reality_sentiment) / 2
        metrics.sentiment_accuracy = 1 - sentiment_diff
        
        # 3. Engagement comparison
        sim_engagement = sum(p.get("likes", 0) + p.get("shares", 0) for p in simulation_posts)
        real_engagement = sum(p.likes + p.shares for p in real_crawl.posts)
        
        metrics.predicted_engagement = sim_engagement
        metrics.actual_engagement = real_engagement
        
        if real_engagement > 0:
            ratio = min(sim_engagement, real_engagement) / max(sim_engagement, real_engagement)
            metrics.engagement_accuracy = ratio
        else:
            metrics.engagement_accuracy = 1.0 if sim_engagement == 0 else 0.0
        
        # 4. Entity comparison
        if simulation_entities:
            real_entities = self._extract_entities_from_posts(real_crawl.posts)
            sim_entities_set = set(e.lower() for e in simulation_entities)
            real_entities_set = set(e.lower() for e in real_entities)
            
            metrics.correctly_predicted_entities = list(
                sim_entities_set & real_entities_set
            )[:10]
            metrics.missed_entities = list(
                real_entities_set - sim_entities_set
            )[:10]
        
        # 5. Overall accuracy (weighted average)
        weights = {
            "topic": 0.3,
            "sentiment": 0.3,
            "engagement": 0.2,
            "entity": 0.2,
        }
        
        entity_score = 0.0
        if simulation_entities:
            total_entities = len(set(simulation_entities)) + len(metrics.missed_entities)
            if total_entities > 0:
                entity_score = len(metrics.correctly_predicted_entities) / total_entities
        
        metrics.overall_accuracy = (
            weights["topic"] * metrics.topic_overlap_score +
            weights["sentiment"] * metrics.sentiment_accuracy +
            weights["engagement"] * metrics.engagement_accuracy +
            weights["entity"] * entity_score
        )
        
        logger.info(f"Comparison complete: overall accuracy = {metrics.overall_accuracy:.1%}")
        return metrics
    
    def validate_predictions(
        self,
        predictions: List[str],
        real_posts: List[ScrapedPost],
    ) -> List[PredictionValidation]:
        """
        Validate specific predictions against real data.
        
        Args:
            predictions: List of prediction statements
            real_posts: Real posts to validate against
            
        Returns:
            List of validation results
        """
        validations = []
        
        # Combine all real content for searching
        real_content = " ".join([p.content.lower() for p in real_posts])
        real_hashtags = []
        for p in real_posts:
            real_hashtags.extend([h.lower() for h in (p.hashtags or [])])
        
        for prediction in predictions:
            # Extract key terms from prediction
            pred_lower = prediction.lower()
            key_terms = [
                word for word in pred_lower.split()
                if len(word) > 4 and word.isalpha()
            ]
            
            # Check for matches
            matches = sum(1 for term in key_terms if term in real_content)
            confidence = matches / len(key_terms) if key_terms else 0
            
            # Find evidence
            evidence = ""
            for post in real_posts[:20]:  # Check first 20 posts
                post_lower = post.content.lower()
                if any(term in post_lower for term in key_terms[:3]):
                    evidence = post.content[:200]
                    break
            
            validations.append(PredictionValidation(
                prediction=prediction,
                evidence=evidence,
                validated=confidence > 0.3,
                confidence=min(confidence * 1.5, 1.0),  # Scale up
            ))
        
        return validations
    
    def generate_comparison_report(
        self,
        metrics: ComparisonMetrics,
        validations: Optional[List[PredictionValidation]] = None,
    ) -> str:
        """
        Generate a markdown comparison report.
        
        Args:
            metrics: Comparison metrics
            validations: Optional prediction validations
            
        Returns:
            Markdown formatted report
        """
        report_parts = [
            "# Simulation Prediction Validation Report",
            "",
            f"> Generated at: {datetime.now().strftime('%Y-%m-%d %H:%M')}",
            "",
            "## Overall Accuracy",
            "",
            f"**Overall Score: {metrics.overall_accuracy:.1%}**",
            "",
            "| Metric | Score |",
            "|--------|-------|",
            f"| Topic Overlap | {metrics.topic_overlap_score:.1%} |",
            f"| Sentiment Accuracy | {metrics.sentiment_accuracy:.1%} |",
            f"| Engagement Accuracy | {metrics.engagement_accuracy:.1%} |",
            "",
            "## Topic Analysis",
            "",
            f"**Matched Topics ({len(metrics.matched_topics)}):** " +
            (", ".join(metrics.matched_topics[:5]) or "None"),
            "",
            f"**Simulation-only ({len(metrics.simulation_only_topics)}):** " +
            (", ".join(metrics.simulation_only_topics[:3]) or "None"),
            "",
            f"**Reality-only ({len(metrics.reality_only_topics)}):** " +
            (", ".join(metrics.reality_only_topics[:3]) or "None"),
            "",
            "## Sentiment Comparison",
            "",
            f"- **Predicted:** {metrics.simulation_sentiment:.2f}",
            f"- **Actual:** {metrics.reality_sentiment:.2f}",
            "",
        ]
        
        if validations:
            report_parts.extend([
                "## Prediction Validations",
                "",
            ])
            
            for i, v in enumerate(validations, 1):
                status = "✅" if v.validated else "❌"
                report_parts.append(
                    f"{i}. {status} **{v.prediction[:80]}**"
                )
                report_parts.append(
                    f"   - Confidence: {v.confidence:.1%}"
                )
                if v.evidence:
                    report_parts.append(
                        f"   - Evidence: *\"{v.evidence[:100]}...\"*"
                    )
                report_parts.append("")
        
        return "\n".join(report_parts)
    
    def _extract_topics_from_posts(
        self,
        posts: List[ScrapedPost]
    ) -> set:
        """Extract topics from posts via hashtags and keywords"""
        topics = set()
        
        for post in posts:
            # Extract hashtags
            if post.hashtags:
                for tag in post.hashtags:
                    topics.add(tag.lower().strip('#'))
            
            # Extract capitalized words that might be topics
            import re
            words = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b', post.content)
            for word in words:
                if len(word) > 4:
                    topics.add(word.lower())
        
        return topics
    
    def _extract_entities_from_posts(
        self,
        posts: List[ScrapedPost]
    ) -> set:
        """Extract entity mentions from posts"""
        entities = set()
        
        for post in posts:
            # Extract @mentions
            if post.mentions:
                entities.update(m.lower() for m in post.mentions)
            
            # Extract capitalized names
            import re
            names = re.findall(r'@(\w+)', post.content)
            entities.update(n.lower() for n in names)
            
            # Add author
            if post.author_name:
                entities.add(post.author_name.lower())
        
        return entities
    
    def _analyze_sentiment_batch(self, texts: List[str]) -> float:
        """
        Simple sentiment analysis for a batch of texts.
        Returns average sentiment score from -1 to 1.
        """
        if not texts:
            return 0.0
        
        total_score = 0.0
        
        for text in texts:
            text_lower = text.lower()
            
            pos_count = sum(1 for word in self.POSITIVE_KEYWORDS if word in text_lower)
            neg_count = sum(1 for word in self.NEGATIVE_KEYWORDS if word in text_lower)
            
            if pos_count + neg_count > 0:
                score = (pos_count - neg_count) / (pos_count + neg_count)
            else:
                score = 0.0
            
            total_score += score
        
        return total_score / len(texts)
