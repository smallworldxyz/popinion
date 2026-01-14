"""
Survey Service
Structured surveys for agents (Opinion Polls + Likert Scales)

Provides:
1. Survey template creation and management
2. Survey deployment to agents
3. Response aggregation by question and faction
"""

import json
import os
import uuid
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum

from ..config import Config
from ..utils.logger import get_logger

logger = get_logger('pubop.survey')


class SurveyType(Enum):
    """Survey question types"""
    OPINION_POLL = "opinion_poll"  # Agree/Disagree/Neutral
    LIKERT_SCALE = "likert"        # 1-5 rating scale


@dataclass
class SurveyQuestion:
    """A single survey question"""
    question_id: str
    question_text: str
    question_type: SurveyType
    options: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "question_id": self.question_id,
            "question_text": self.question_text,
            "question_type": self.question_type.value,
            "options": self.options
        }
    
    @classmethod
    def from_dict(cls, data: Dict) -> 'SurveyQuestion':
        return cls(
            question_id=data.get("question_id", str(uuid.uuid4())[:8]),
            question_text=data.get("question_text", ""),
            question_type=SurveyType(data.get("question_type", "opinion_poll")),
            options=data.get("options", [])
        )


@dataclass
class SurveyTemplate:
    """A complete survey template"""
    survey_id: str
    title: str
    description: str = ""
    questions: List[SurveyQuestion] = field(default_factory=list)
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "survey_id": self.survey_id,
            "title": self.title,
            "description": self.description,
            "questions": [q.to_dict() for q in self.questions],
            "created_at": self.created_at
        }
    
    @classmethod
    def from_dict(cls, data: Dict) -> 'SurveyTemplate':
        questions = [SurveyQuestion.from_dict(q) for q in data.get("questions", [])]
        return cls(
            survey_id=data.get("survey_id", f"survey_{uuid.uuid4().hex[:8]}"),
            title=data.get("title", "Untitled Survey"),
            description=data.get("description", ""),
            questions=questions,
            created_at=data.get("created_at", datetime.now().isoformat())
        )


@dataclass
class AgentSurveyResponse:
    """A single agent's response to a survey"""
    agent_id: int
    agent_name: str
    faction: str
    responses: Dict[str, Any]  # question_id -> response value
    raw_text: str = ""  # Full text response from agent
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "agent_id": self.agent_id,
            "agent_name": self.agent_name,
            "faction": self.faction,
            "responses": self.responses,
            "raw_text": self.raw_text,
            "timestamp": self.timestamp
        }


@dataclass
class SurveyResult:
    """Aggregated survey results"""
    survey_id: str
    template: SurveyTemplate
    total_respondents: int
    responses: List[AgentSurveyResponse]
    
    # Aggregated statistics per question
    aggregated: Dict[str, Dict] = field(default_factory=dict)
    # By faction breakdown
    by_faction: Dict[str, List[AgentSurveyResponse]] = field(default_factory=dict)
    
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "survey_id": self.survey_id,
            "template": self.template.to_dict(),
            "total_respondents": self.total_respondents,
            "responses": [r.to_dict() for r in self.responses],
            "aggregated": self.aggregated,
            "by_faction": {
                faction: [r.to_dict() for r in resps]
                for faction, resps in self.by_faction.items()
            },
            "timestamp": self.timestamp
        }


class SurveyService:
    """
    Survey Service for structured agent surveys
    
    Features:
    - Create survey templates (opinion polls, Likert scales)
    - Deploy surveys to agents
    - Aggregate responses by question and faction
    """
    
    # Valid question types for validation
    VALID_QUESTION_TYPES = {"opinion_poll", "likert"}
    
    def __init__(self):
        self.surveys_dir = os.path.join(Config.UPLOAD_FOLDER, "surveys")
        os.makedirs(self.surveys_dir, exist_ok=True)
    
    def create_survey(
        self,
        title: str,
        questions: List[Dict[str, Any]],
        description: str = ""
    ) -> SurveyTemplate:
        """
        Create a new survey template
        
        Args:
            title: Survey title
            questions: List of question dicts with question_text, question_type, options
            description: Optional survey description
            
        Returns:
            SurveyTemplate
            
        Raises:
            ValueError: If validation fails
        """
        # Validate title
        if not title or not title.strip():
            raise ValueError("Survey title is required and cannot be empty")
        
        # Validate questions
        if not questions:
            raise ValueError("At least one question is required")
        
        survey_id = f"survey_{uuid.uuid4().hex[:8]}"
        
        survey_questions = []
        for i, q in enumerate(questions):
            # Validate question_text
            question_text = q.get("question_text", "").strip()
            if not question_text:
                raise ValueError(f"Question {i+1}: question_text is required and cannot be empty")
            
            # Validate question_type
            question_type = q.get("question_type", "opinion_poll")
            if question_type not in self.VALID_QUESTION_TYPES:
                raise ValueError(
                    f"Question {i+1}: Invalid question_type '{question_type}'. "
                    f"Must be one of: {', '.join(sorted(self.VALID_QUESTION_TYPES))}"
                )
            
            # Validate options for opinion_poll
            options = q.get("options", self._get_default_options(question_type))
            if question_type == "opinion_poll" and len(options) < 2:
                raise ValueError(
                    f"Question {i+1}: Opinion poll requires at least 2 options"
                )
            
            question = SurveyQuestion(
                question_id=f"q{i+1}",
                question_text=question_text,
                question_type=SurveyType(question_type),
                options=options
            )
            survey_questions.append(question)
        
        template = SurveyTemplate(
            survey_id=survey_id,
            title=title.strip(),
            description=description.strip() if description else "",
            questions=survey_questions
        )
        
        # Save template
        self._save_survey(template)
        
        logger.info(f"Created survey: {survey_id} with {len(survey_questions)} questions")
        return template
    
    def get_survey(self, survey_id: str) -> Optional[SurveyTemplate]:
        """Get a survey template by ID"""
        survey_path = os.path.join(self.surveys_dir, f"{survey_id}.json")
        if not os.path.exists(survey_path):
            return None
        
        with open(survey_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        return SurveyTemplate.from_dict(data)
    
    def list_surveys(self) -> List[SurveyTemplate]:
        """List all survey templates"""
        surveys = []
        for filename in os.listdir(self.surveys_dir):
            if filename.endswith('.json') and filename.startswith('survey_'):
                survey_path = os.path.join(self.surveys_dir, filename)
                with open(survey_path, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                surveys.append(SurveyTemplate.from_dict(data))
        return surveys
    
    def build_survey_prompt(self, template: SurveyTemplate) -> str:
        """
        Build a prompt for agents to answer the survey
        
        Args:
            template: Survey template
            
        Returns:
            Formatted prompt string
        """
        prompt_parts = [
            f"Please respond to this survey: {template.title}",
            ""
        ]
        
        if template.description:
            prompt_parts.append(f"Description: {template.description}")
            prompt_parts.append("")
        
        for q in template.questions:
            if q.question_type == SurveyType.OPINION_POLL:
                prompt_parts.append(
                    f"{q.question_id}. {q.question_text}\n"
                    f"   Options: {', '.join(q.options)}\n"
                    f"   Your answer (choose one option):"
                )
            elif q.question_type == SurveyType.LIKERT_SCALE:
                prompt_parts.append(
                    f"{q.question_id}. {q.question_text}\n"
                    f"   Rate from 1 (Strongly Disagree) to 5 (Strongly Agree)\n"
                    f"   Your rating (1-5):"
                )
            prompt_parts.append("")
        
        prompt_parts.append(
            "Please respond with your answers in the format:\n"
            "q1: [your answer]\n"
            "q2: [your answer]\n"
            "...\n"
            "Brief explanation (optional): [your reasoning]"
        )
        
        return "\n".join(prompt_parts)
    
    def parse_agent_response(
        self,
        template: SurveyTemplate,
        agent_id: int,
        agent_name: str,
        faction: str,
        raw_response: str
    ) -> AgentSurveyResponse:
        """
        Parse an agent's raw text response into structured data
        
        Args:
            template: Survey template
            agent_id: Agent ID
            agent_name: Agent name
            faction: Agent faction/type
            raw_response: Raw text response from agent
            
        Returns:
            AgentSurveyResponse with parsed answers
        """
        responses = {}
        
        for q in template.questions:
            # Try to extract answer for this question
            answer = self._extract_answer(raw_response, q)
            responses[q.question_id] = answer
        
        return AgentSurveyResponse(
            agent_id=agent_id,
            agent_name=agent_name,
            faction=faction,
            responses=responses,
            raw_text=raw_response
        )
    
    def aggregate_results(
        self,
        template: SurveyTemplate,
        responses: List[AgentSurveyResponse]
    ) -> SurveyResult:
        """
        Aggregate survey responses into statistics
        
        Args:
            template: Survey template
            responses: List of agent responses
            
        Returns:
            SurveyResult with aggregated statistics
        """
        # Group by faction
        by_faction: Dict[str, List[AgentSurveyResponse]] = {}
        for r in responses:
            faction = r.faction or "unknown"
            if faction not in by_faction:
                by_faction[faction] = []
            by_faction[faction].append(r)
        
        # Aggregate per question
        aggregated = {}
        for q in template.questions:
            q_responses = [r.responses.get(q.question_id) for r in responses]
            q_responses = [r for r in q_responses if r is not None]
            
            if q.question_type == SurveyType.OPINION_POLL:
                # Count options
                counts = {opt: 0 for opt in q.options}
                for resp in q_responses:
                    resp_lower = str(resp).lower()
                    for opt in q.options:
                        if opt.lower() in resp_lower or resp_lower in opt.lower():
                            counts[opt] += 1
                            break
                
                total = sum(counts.values())
                distribution = {
                    opt: round(count / total * 100, 1) if total > 0 else 0
                    for opt, count in counts.items()
                }
                
                aggregated[q.question_id] = {
                    "question_text": q.question_text,
                    "question_type": q.question_type.value,
                    "counts": counts,
                    "distribution": distribution,
                    "total_responses": total
                }
                
            elif q.question_type == SurveyType.LIKERT_SCALE:
                # Calculate numeric stats
                numeric_values = []
                for resp in q_responses:
                    try:
                        val = int(str(resp).strip()[0])  # Take first digit
                        if 1 <= val <= 5:
                            numeric_values.append(val)
                    except (ValueError, IndexError):
                        pass
                
                if numeric_values:
                    avg = sum(numeric_values) / len(numeric_values)
                    distribution = {str(i): numeric_values.count(i) for i in range(1, 6)}
                else:
                    avg = 0
                    distribution = {str(i): 0 for i in range(1, 6)}
                
                aggregated[q.question_id] = {
                    "question_text": q.question_text,
                    "question_type": q.question_type.value,
                    "average": round(avg, 2),
                    "distribution": distribution,
                    "total_responses": len(numeric_values)
                }
        
        return SurveyResult(
            survey_id=template.survey_id,
            template=template,
            total_respondents=len(responses),
            responses=responses,
            aggregated=aggregated,
            by_faction=by_faction
        )
    
    def _get_default_options(self, question_type: str) -> List[str]:
        """Get default options for a question type"""
        if question_type == "opinion_poll":
            return ["Agree", "Disagree", "Neutral"]
        elif question_type == "likert":
            return ["1", "2", "3", "4", "5"]
        return []
    
    def _extract_answer(self, raw_response: str, question: SurveyQuestion) -> Optional[str]:
        """Extract answer for a specific question from raw response"""
        import re
        
        # Look for patterns like "q1: answer" or "q1. answer"
        patterns = [
            rf"{question.question_id}[:\.\)]?\s*(.+?)(?=\n|q\d|$)",
            rf"{question.question_id[1:]}[:\.\)]?\s*(.+?)(?=\n|\d|$)",  # Without 'q' prefix
        ]
        
        for pattern in patterns:
            match = re.search(pattern, raw_response, re.IGNORECASE)
            if match:
                return match.group(1).strip()
        
        # Fallback: look for option keywords in response
        if question.question_type == SurveyType.OPINION_POLL:
            response_lower = raw_response.lower()
            for opt in question.options:
                if opt.lower() in response_lower:
                    return opt
        
        return None
    
    def _save_survey(self, template: SurveyTemplate):
        """Save survey template to file"""
        survey_path = os.path.join(self.surveys_dir, f"{template.survey_id}.json")
        with open(survey_path, 'w', encoding='utf-8') as f:
            json.dump(template.to_dict(), f, ensure_ascii=False, indent=2)
