"""
Neo4j Database Service
Handles connections and basic operations for Neo4j graph database
Refactored to use BaseService and Result[T] pattern.
"""

import time
import uuid as uuid_lib
from typing import Dict, Any, List, Optional, Callable, TypeVar
from contextlib import contextmanager
from dataclasses import dataclass

from neo4j import GraphDatabase, Driver, Session
from neo4j import Result as Neo4jResult
from neo4j.exceptions import ServiceUnavailable, TransientError

from ..config import Config
from ..utils.logger import get_logger
from .base_service import BaseService, Result, ErrorCode

logger = get_logger('pubop.neo4j_service')

T = TypeVar('T')


@dataclass
class NodeCreationResult:
    """Result of node creation"""
    uuid: str
    labels: List[str]
    properties_set: int


@dataclass
class RelationshipCreationResult:
    """Result of relationship creation"""
    created: bool
    source_uuid: str
    target_uuid: str
    relationship_type: str


@dataclass
class QueryStats:
    """Statistics from a write query"""
    nodes_created: int = 0
    nodes_deleted: int = 0
    relationships_created: int = 0
    relationships_deleted: int = 0
    properties_set: int = 0
    labels_added: int = 0
    labels_removed: int = 0


class Neo4jService(BaseService):
    """
    Neo4j database service for Popinion
    
    Provides connection management, transaction handling, and basic CRUD operations.
    Inherits from BaseService for consistent error handling.
    """
    
    def __init__(
        self,
        uri: Optional[str] = None,
        username: Optional[str] = None,
        password: Optional[str] = None,
        database: Optional[str] = None
    ):
        """
        Initialize Neo4j service
        
        Args:
            uri: Neo4j URI (bolt://localhost:7687)
            username: Neo4j username
            password: Neo4j password
            database: Neo4j database name
        """
        super().__init__(name="neo4j")
        
        self.uri = uri or Config.NEO4J_URI
        self.username = username or Config.NEO4J_USERNAME
        self.password = password or Config.NEO4J_PASSWORD
        self.database = database or Config.NEO4J_DATABASE
        
        if not all([self.uri, self.username, self.password]):
            raise ValueError("Neo4j connection parameters not configured")
        
        self.driver: Optional[Driver] = None
        self._connect()
    
    def _connect(self):
        """Establish connection to Neo4j"""
        try:
            self.driver = GraphDatabase.driver(
                self.uri,
                auth=(self.username, self.password),
                max_connection_lifetime=3600,
                max_connection_pool_size=50,
                connection_acquisition_timeout=120
            )
            # Verify connectivity
            self.driver.verify_connectivity()
            self.logger.info(f"Connected to Neo4j at {self.uri}")
        except Exception as e:
            self.logger.error(f"Failed to connect to Neo4j: {e}")
            raise
    
    def close(self):
        """Close Neo4j driver connection"""
        if self.driver:
            self.driver.close()
            self.logger.info("Neo4j connection closed")
    
    @contextmanager
    def session(self, database: Optional[str] = None):
        """
        Context manager for Neo4j session
        
        Args:
            database: Override default database
            
        Yields:
            Neo4j Session object
        """
        db = database or self.database
        session = self.driver.session(database=db)
        try:
            yield session
        finally:
            session.close()
    
    # ==========================================================================
    # RESULT-BASED METHODS (New Pattern)
    # ==========================================================================
    
    def safe_execute_query(
        self,
        query: str,
        parameters: Optional[Dict[str, Any]] = None,
        database: Optional[str] = None
    ) -> Result[List[Dict[str, Any]]]:
        """
        Execute a Cypher query and return Result-wrapped results.
        
        Returns:
            Result containing list of records or error
        """
        try:
            parameters = parameters or {}
            with self.session(database) as session:
                result = session.run(query, parameters)
                return Result.success([dict(record) for record in result])
        except Exception as e:
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Query execution failed: {str(e)}"
            )
    
    def safe_execute_write(
        self,
        query: str,
        parameters: Optional[Dict[str, Any]] = None,
        database: Optional[str] = None
    ) -> Result[QueryStats]:
        """
        Execute a write transaction and return Result-wrapped stats.
        
        Returns:
            Result containing QueryStats or error
        """
        try:
            parameters = parameters or {}
            with self.session(database) as session:
                result = session.run(query, parameters)
                summary = result.consume()
                
                return Result.success(QueryStats(
                    nodes_created=summary.counters.nodes_created,
                    nodes_deleted=summary.counters.nodes_deleted,
                    relationships_created=summary.counters.relationships_created,
                    relationships_deleted=summary.counters.relationships_deleted,
                    properties_set=summary.counters.properties_set,
                    labels_added=summary.counters.labels_added,
                    labels_removed=summary.counters.labels_removed
                ))
        except Exception as e:
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Write execution failed: {str(e)}"
            )
    
    def safe_create_node(
        self,
        labels: List[str],
        properties: Dict[str, Any],
        database: Optional[str] = None
    ) -> Result[NodeCreationResult]:
        """
        Create a node with given labels and properties.
        
        Returns:
            Result containing NodeCreationResult or error
        """
        try:
            # Add UUID if not present
            if 'uuid' not in properties:
                properties['uuid'] = str(uuid_lib.uuid4())
            
            labels_str = ':'.join(labels)
            query = f"""
            CREATE (n:{labels_str} $properties)
            RETURN n.uuid as uuid
            """
            
            result = self.execute_query(query, {'properties': properties}, database)
            node_uuid = result[0]['uuid'] if result else properties['uuid']
            
            return Result.success(NodeCreationResult(
                uuid=node_uuid,
                labels=labels,
                properties_set=len(properties)
            ))
        except Exception as e:
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Node creation failed: {str(e)}"
            )
    
    def safe_create_relationship(
        self,
        source_uuid: str,
        target_uuid: str,
        relationship_type: str,
        properties: Optional[Dict[str, Any]] = None,
        database: Optional[str] = None
    ) -> Result[RelationshipCreationResult]:
        """
        Create a relationship between two nodes.
        
        Returns:
            Result containing RelationshipCreationResult or error
        """
        try:
            properties = properties or {}
            
            query = f"""
            MATCH (a {{uuid: $source_uuid}})
            MATCH (b {{uuid: $target_uuid}})
            CREATE (a)-[r:{relationship_type} $properties]->(b)
            RETURN r
            """
            
            result = self.execute_query(
                query,
                {
                    'source_uuid': source_uuid,
                    'target_uuid': target_uuid,
                    'properties': properties
                },
                database
            )
            
            return Result.success(RelationshipCreationResult(
                created=len(result) > 0,
                source_uuid=source_uuid,
                target_uuid=target_uuid,
                relationship_type=relationship_type
            ))
        except Exception as e:
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Relationship creation failed: {str(e)}"
            )
    
    def safe_get_node(
        self,
        uuid: str,
        database: Optional[str] = None
    ) -> Result[Optional[Dict[str, Any]]]:
        """
        Get node by UUID.
        
        Returns:
            Result containing node dict or None, or error
        """
        try:
            node = self.get_node_by_uuid(uuid, database)
            return Result.success(node)
        except Exception as e:
            return Result.failure(
                ErrorCode.INTERNAL_ERROR,
                f"Node retrieval failed: {str(e)}"
            )
    
    # ==========================================================================
    # LEGACY METHODS (Preserved for backward compatibility)
    # ==========================================================================
    
    def execute_query(
        self,
        query: str,
        parameters: Optional[Dict[str, Any]] = None,
        database: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Execute a Cypher query and return results (legacy method).
        
        Args:
            query: Cypher query string
            parameters: Query parameters
            database: Override default database
            
        Returns:
            List of result records as dictionaries
        """
        parameters = parameters or {}
        
        with self.session(database) as session:
            result = session.run(query, parameters)
            return [dict(record) for record in result]
    
    def execute_write(
        self,
        query: str,
        parameters: Optional[Dict[str, Any]] = None,
        database: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Execute a write transaction (legacy method).
        
        Args:
            query: Cypher query string
            parameters: Query parameters
            database: Override default database
            
        Returns:
            Summary statistics
        """
        parameters = parameters or {}
        
        with self.session(database) as session:
            result = session.run(query, parameters)
            summary = result.consume()
            
            return {
                "nodes_created": summary.counters.nodes_created,
                "nodes_deleted": summary.counters.nodes_deleted,
                "relationships_created": summary.counters.relationships_created,
                "relationships_deleted": summary.counters.relationships_deleted,
                "properties_set": summary.counters.properties_set,
                "labels_added": summary.counters.labels_added,
                "labels_removed": summary.counters.labels_removed
            }
    
    def execute_with_retry(
        self,
        func: Callable[[], T],
        operation_name: str,
        max_retries: int = 3,
        initial_delay: float = 2.0
    ) -> T:
        """
        Execute operation with retry logic for transient errors
        
        Args:
            func: Function to execute
            operation_name: Name for logging
            max_retries: Maximum retry attempts
            initial_delay: Initial delay in seconds
            
        Returns:
            Function result
        """
        last_exception = None
        delay = initial_delay
        
        for attempt in range(max_retries):
            try:
                return func()
            except (ServiceUnavailable, TransientError) as e:
                last_exception = e
                if attempt < max_retries - 1:
                    self.logger.warning(
                        f"Neo4j {operation_name} attempt {attempt + 1} failed: {str(e)[:100]}, "
                        f"retrying in {delay:.1f}s..."
                    )
                    time.sleep(delay)
                    delay *= 2  # Exponential backoff
                else:
                    self.logger.error(f"Neo4j {operation_name} failed after {max_retries} attempts: {str(e)}")
        
        raise last_exception
    
    def create_node(
        self,
        labels: List[str],
        properties: Dict[str, Any],
        database: Optional[str] = None
    ) -> str:
        """
        Create a node with given labels and properties (legacy method).
        
        Args:
            labels: Node labels
            properties: Node properties
            database: Override default database
            
        Returns:
            Node UUID
        """
        # Add UUID if not present
        if 'uuid' not in properties:
            properties['uuid'] = str(uuid_lib.uuid4())
        
        labels_str = ':'.join(labels)
        query = f"""
        CREATE (n:{labels_str} $properties)
        RETURN n.uuid as uuid
        """
        
        result = self.execute_query(query, {'properties': properties}, database)
        return result[0]['uuid'] if result else properties['uuid']
    
    def create_relationship(
        self,
        source_uuid: str,
        target_uuid: str,
        relationship_type: str,
        properties: Optional[Dict[str, Any]] = None,
        database: Optional[str] = None
    ) -> bool:
        """
        Create a relationship between two nodes (legacy method).
        
        Args:
            source_uuid: Source node UUID
            target_uuid: Target node UUID
            relationship_type: Relationship type
            properties: Relationship properties
            database: Override default database
            
        Returns:
            Success status
        """
        properties = properties or {}
        
        query = f"""
        MATCH (a {{uuid: $source_uuid}})
        MATCH (b {{uuid: $target_uuid}})
        CREATE (a)-[r:{relationship_type} $properties]->(b)
        RETURN r
        """
        
        result = self.execute_query(
            query,
            {
                'source_uuid': source_uuid,
                'target_uuid': target_uuid,
                'properties': properties
            },
            database
        )
        return len(result) > 0
    
    def get_node_by_uuid(
        self,
        uuid: str,
        database: Optional[str] = None
    ) -> Optional[Dict[str, Any]]:
        """
        Get node by UUID (legacy method).
        
        Args:
            uuid: Node UUID
            database: Override default database
            
        Returns:
            Node properties or None
        """
        query = """
        MATCH (n {uuid: $uuid})
        RETURN n, labels(n) as labels
        """
        
        result = self.execute_query(query, {'uuid': uuid}, database)
        if result:
            node = result[0]['n']
            return {
                **dict(node),
                'labels': result[0]['labels']
            }
        return None
    
    def verify_connection(self) -> bool:
        """
        Verify Neo4j connection is working
        
        Returns:
            True if connection is healthy
        """
        try:
            with self.session() as session:
                result = session.run("RETURN 1 as test")
                return result.single()['test'] == 1
        except Exception as e:
            self.logger.error(f"Connection verification failed: {e}")
            return False
    
    def create_constraints(self, graph_id: str):
        """
        Create constraints and indexes for a graph
        
        Args:
            graph_id: Graph identifier
        """
        constraints = [
            # UUID uniqueness constraint
            f"CREATE CONSTRAINT {graph_id}_node_uuid IF NOT EXISTS FOR (n:GraphNode) REQUIRE n.uuid IS UNIQUE",
            # Graph ID index
            f"CREATE INDEX {graph_id}_graph_id IF NOT EXISTS FOR (n:GraphNode) ON (n.graph_id)",
        ]
        
        with self.session() as session:
            for constraint in constraints:
                try:
                    session.run(constraint)
                    self.logger.debug(f"Created constraint/index: {constraint[:50]}...")
                except Exception as e:
                    self.logger.warning(f"Constraint/index already exists or error: {e}")
    
    def delete_graph(self, graph_id: str):
        """
        Delete all nodes and relationships for a graph
        
        Args:
            graph_id: Graph identifier
        """
        query = """
        MATCH (n:GraphNode {graph_id: $graph_id})
        DETACH DELETE n
        """
        
        summary = self.execute_write(query, {'graph_id': graph_id})
        self.logger.info(f"Deleted graph {graph_id}: {summary['nodes_deleted']} nodes, "
                   f"{summary['relationships_deleted']} relationships")
    
    def __enter__(self):
        """Context manager entry"""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        self.close()
        return False


# Singleton instance
_service_instance: Optional[Neo4jService] = None

def get_neo4j_service() -> Neo4jService:
    """Get or create singleton Neo4jService instance"""
    global _service_instance
    if _service_instance is None:
        _service_instance = Neo4jService()
    return _service_instance
