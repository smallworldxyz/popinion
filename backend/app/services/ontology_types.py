"""
Ontology Type Definitions
Dataclasses for ontology generation structure
Extracted from ontology_generator.py following Kaizen principles
"""

from dataclasses import dataclass, field
from typing import List, Dict, Any, Optional

@dataclass
class OntologyAttribute:
    name: str
    type: str
    description: str

@dataclass
class OntologyEntityType:
    name: str
    description: str
    attributes: List[Dict[str, str]]
    examples: List[str]

@dataclass
class OntologyEdgeType:
    name: str
    description: str
    source_targets: List[Dict[str, str]]
    attributes: List[Dict[str, str]] = field(default_factory=list)

@dataclass
class OntologyDefinition:
    entity_types: List[Dict[str, Any]]
    edge_types: List[Dict[str, Any]]
    analysis_summary: str
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "entity_types": self.entity_types,
            "edge_types": self.edge_types,
            "analysis_summary": self.analysis_summary
        }
