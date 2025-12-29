"""
Tests for RWSP comparison tools
"""

import pytest
from datetime import datetime

from app.models.pubop import ScrapedPost, ScrapedUser, CrawlResult, ScrapedTrend
from app.services.pubop_comparison import (
    PubopComparisonTools,
    ComparisonMetrics,
    PredictionValidation,
)


class TestComparisonMetrics:
    """Tests for ComparisonMetrics dataclass"""
    
    def test_create_metrics(self):
        """Test creating metrics"""
        metrics = ComparisonMetrics()
        assert metrics.overall_accuracy == 0.0
        assert metrics.topic_overlap_score == 0.0
    
    def test_metrics_to_dict(self):
        """Test converting to dictionary"""
        metrics = ComparisonMetrics(
            topic_overlap_score=0.75,
            overall_accuracy=0.65,
            matched_topics=["topic1", "topic2"]
        )
        d = metrics.to_dict()
        assert d["topic_overlap_score"] == 0.75
        assert len(d["matched_topics"]) == 2
    
    def test_metrics_to_text(self):
        """Test generating readable text"""
        metrics = ComparisonMetrics(
            overall_accuracy=0.72,
            topic_overlap_score=0.8,
            sentiment_accuracy=0.9,
            simulation_sentiment=0.5,
            reality_sentiment=0.4,
        )
        text = metrics.to_text()
        assert "72.0%" in text
        assert "TOPIC ANALYSIS" in text
        assert "SENTIMENT ANALYSIS" in text


class TestPredictionValidation:
    """Tests for PredictionValidation dataclass"""
    
    def test_create_validation(self):
        """Test creating validation"""
        v = PredictionValidation(
            prediction="Topic will trend",
            evidence="Found in post",
            validated=True,
            confidence=0.85
        )
        assert v.validated == True
        assert v.confidence == 0.85
    
    def test_validation_to_dict(self):
        """Test converting to dict"""
        v = PredictionValidation(
            prediction="Test",
            evidence="Evidence",
            validated=False,
            confidence=0.3
        )
        d = v.to_dict()
        assert d["validated"] == False


class TestPubopComparisonTools:
    """Tests for PubopComparisonTools"""
    
    @pytest.fixture
    def tools(self):
        return PubopComparisonTools()
    
    @pytest.fixture
    def sample_crawl(self):
        """Create sample crawl result"""
        return CrawlResult(
            platform="twitter",
            query="test",
            posts=[
                ScrapedPost(
                    platform="twitter",
                    post_id="1",
                    content="This is great news about Python!",
                    author_id="user1",
                    author_name="User One",
                    timestamp=datetime.now(),
                    likes=100,
                    shares=50,
                    hashtags=["python", "programming"]
                ),
                ScrapedPost(
                    platform="twitter",
                    post_id="2",
                    content="Terrible update, very worried about this.",
                    author_id="user2",
                    author_name="User Two",
                    timestamp=datetime.now(),
                    likes=20,
                    shares=10,
                    hashtags=["concerns"]
                ),
            ],
            trends=[
                ScrapedTrend(
                    platform="twitter",
                    topic="python",
                    post_count=1000,
                    timestamp=datetime.now()
                )
            ]
        )
    
    def test_compare_with_real_data_topic_overlap(self, tools, sample_crawl):
        """Test topic comparison"""
        simulation_topics = ["python", "javascript", "coding"]
        simulation_posts = []
        
        metrics = tools.compare_with_real_data(
            simulation_topics=simulation_topics,
            simulation_posts=simulation_posts,
            real_crawl=sample_crawl
        )
        
        assert metrics.topic_overlap_score > 0  # Python should match
        assert "python" in [t.lower() for t in metrics.matched_topics]
    
    def test_compare_sentiment(self, tools, sample_crawl):
        """Test sentiment comparison"""
        simulation_posts = [
            {"content": "This is really good and positive!", "likes": 10}
        ]
        
        metrics = tools.compare_with_real_data(
            simulation_topics=[],
            simulation_posts=simulation_posts,
            real_crawl=sample_crawl
        )
        
        assert metrics.sentiment_accuracy >= 0
        assert metrics.sentiment_accuracy <= 1
    
    def test_compare_engagement(self, tools, sample_crawl):
        """Test engagement comparison"""
        simulation_posts = [
            {"content": "Test", "likes": 50, "shares": 25}
        ]
        
        metrics = tools.compare_with_real_data(
            simulation_topics=[],
            simulation_posts=simulation_posts,
            real_crawl=sample_crawl
        )
        
        assert metrics.actual_engagement == 180  # 100+50+20+10
        assert metrics.predicted_engagement == 75  # 50+25
    
    def test_validate_predictions(self, tools, sample_crawl):
        """Test prediction validation"""
        predictions = [
            "There will be positive news about Python",
            "Users will share concerns",
            "Random unrelated prediction"
        ]
        
        validations = tools.validate_predictions(
            predictions=predictions,
            real_posts=sample_crawl.posts
        )
        
        assert len(validations) == 3
        # First prediction should have some confidence (python, positive, news)
        assert validations[0].confidence >= 0
    
    def test_generate_comparison_report(self, tools):
        """Test report generation"""
        metrics = ComparisonMetrics(
            overall_accuracy=0.75,
            topic_overlap_score=0.8,
            sentiment_accuracy=0.7,
            engagement_accuracy=0.6,
            matched_topics=["topic1", "topic2"]
        )
        
        report = tools.generate_comparison_report(metrics)
        
        assert "# Simulation Prediction Validation Report" in report
        assert "75.0%" in report
        assert "topic1" in report
    
    def test_analyze_sentiment_batch(self, tools):
        """Test sentiment analysis"""
        positive_texts = ["This is great!", "Amazing work!", "Love it!"]
        negative_texts = ["This is terrible", "Awful outcome", "Hate this"]
        
        pos_score = tools._analyze_sentiment_batch(positive_texts)
        neg_score = tools._analyze_sentiment_batch(negative_texts)
        
        assert pos_score > 0
        assert neg_score < 0
    
    def test_extract_topics_from_posts(self, tools, sample_crawl):
        """Test topic extraction"""
        topics = tools._extract_topics_from_posts(sample_crawl.posts)
        
        assert "python" in topics or "programming" in topics
