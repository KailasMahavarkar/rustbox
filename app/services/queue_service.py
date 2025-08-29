"""
Redis queue service for managing job queues and background tasks.
"""
import json
import logging
import uuid
from typing import Optional, Dict, Any, List
from datetime import datetime, timedelta
import redis
from redis.exceptions import RedisError

from app.config import settings

logger = logging.getLogger(__name__)


class QueueService:
    """Redis-based queue service for managing job queues."""
    
    def __init__(self):
        """Initialize queue service."""
        self.redis_client = None
        self._initialize_redis()
    
    def _initialize_redis(self):
        """Initialize Redis connection."""
        try:
            self.redis_client = redis.from_url(
                settings.redis_url,
                decode_responses=True,
                socket_connect_timeout=5,
                socket_timeout=5,
                retry_on_timeout=True
            )
            
            # Test connection
            self.redis_client.ping()
            logger.info("Redis connection initialized successfully")
            
        except Exception as e:
            logger.warning(f"Failed to initialize Redis connection: {e}")
            # Don't raise during import, will be handled at runtime
            self.redis_client = None
    
    def test_connection(self) -> bool:
        """Test Redis connection."""
        if self.redis_client is None:
            return False
        try:
            self.redis_client.ping()
            return True
        except Exception as e:
            logger.error(f"Redis connection test failed: {e}")
            return False
    
    def get_connection_info(self) -> dict:
        """Get Redis connection information."""
        if self.redis_client is None:
            return {
                "connected": False,
                "error": "Redis client not initialized"
            }
        try:
            info = self.redis_client.info()
            return {
                "connected": True,
                "version": info.get("redis_version", "unknown"),
                "memory_used": info.get("used_memory_human", "unknown"),
                "connected_clients": info.get("connected_clients", 0)
            }
        except Exception as e:
            return {
                "connected": False,
                "error": str(e)
            }
    
    def enqueue_submission(self, submission_id: int, priority: int = 0) -> str:
        """Enqueue a submission for processing."""
        try:
            job_id = str(uuid.uuid4())
            job_data = {
                "job_id": job_id,
                "submission_id": submission_id,
                "priority": priority,
                "created_at": datetime.utcnow().isoformat(),
                "type": "submission"
            }
            
            # Use priority-based queue
            queue_name = f"submissions:priority:{priority}"
            self.redis_client.lpush(queue_name, json.dumps(job_data))
            
            # Also add to general queue for monitoring
            self.redis_client.lpush("submissions:all", job_id)
            
            logger.info(f"Enqueued submission {submission_id} with job_id {job_id}")
            return job_id
            
        except Exception as e:
            logger.error(f"Failed to enqueue submission {submission_id}: {e}")
            raise
    
    def dequeue_submission(self, priority: int = 0) -> Optional[Dict[str, Any]]:
        """Dequeue a submission for processing."""
        try:
            queue_name = f"submissions:priority:{priority}"
            
            # Try high priority first, then normal priority
            for p in [priority, 0]:
                queue_name = f"submissions:priority:{p}"
                result = self.redis_client.rpop(queue_name)
                if result:
                    job_data = json.loads(result)
                    logger.info(f"Dequeued submission {job_data.get('submission_id')}")
                    return job_data
            
            return None
            
        except Exception as e:
            logger.error(f"Failed to dequeue submission: {e}")
            raise
    
    def get_queue_size(self, priority: int = 0) -> int:
        """Get queue size for a specific priority."""
        try:
            queue_name = f"submissions:priority:{priority}"
            return self.redis_client.llen(queue_name)
        except Exception as e:
            logger.error(f"Failed to get queue size: {e}")
            return 0
    
    def get_total_queue_size(self) -> int:
        """Get total queue size across all priorities."""
        try:
            total = 0
            for priority in range(10):  # Check priorities 0-9
                total += self.get_queue_size(priority)
            return total
        except Exception as e:
            logger.error(f"Failed to get total queue size: {e}")
            return 0
    
    def clear_queue(self, priority: int = 0) -> int:
        """Clear queue for a specific priority."""
        try:
            queue_name = f"submissions:priority:{priority}"
            return self.redis_client.delete(queue_name)
        except Exception as e:
            logger.error(f"Failed to clear queue: {e}")
            return 0
    
    def clear_all_queues(self) -> int:
        """Clear all submission queues."""
        try:
            total_cleared = 0
            for priority in range(10):
                total_cleared += self.clear_queue(priority)
            return total_cleared
        except Exception as e:
            logger.error(f"Failed to clear all queues: {e}")
            return 0
    
    def set_worker_status(self, worker_id: str, status: str, metadata: Optional[Dict] = None):
        """Set worker status."""
        try:
            worker_data = {
                "worker_id": worker_id,
                "status": status,
                "last_seen": datetime.utcnow().isoformat(),
                "metadata": metadata or {}
            }
            
            self.redis_client.hset(
                "workers:status",
                worker_id,
                json.dumps(worker_data)
            )
            
            # Set expiration for worker status (5 minutes)
            self.redis_client.expire("workers:status", 300)
            
        except Exception as e:
            logger.error(f"Failed to set worker status: {e}")
    
    def get_worker_count(self) -> int:
        """Get number of active workers."""
        try:
            workers = self.redis_client.hgetall("workers:status")
            active_count = 0
            
            for worker_id, worker_data in workers.items():
                try:
                    data = json.loads(worker_data)
                    last_seen = datetime.fromisoformat(data["last_seen"])
                    # Consider worker active if seen within last 2 minutes
                    if datetime.utcnow() - last_seen < timedelta(minutes=2):
                        active_count += 1
                except (json.JSONDecodeError, KeyError, ValueError):
                    continue
            
            return active_count
            
        except Exception as e:
            logger.error(f"Failed to get worker count: {e}")
            return 0
    
    def publish_event(self, event_type: str, data: Dict[str, Any]):
        """Publish an event to Redis pub/sub."""
        try:
            event_data = {
                "type": event_type,
                "data": data,
                "timestamp": datetime.utcnow().isoformat()
            }
            
            self.redis_client.publish("events", json.dumps(event_data))
            logger.debug(f"Published event: {event_type}")
            
        except Exception as e:
            logger.error(f"Failed to publish event: {e}")
    
    def subscribe_to_events(self, callback):
        """Subscribe to Redis events."""
        try:
            pubsub = self.redis_client.pubsub()
            pubsub.subscribe("events")
            
            for message in pubsub.listen():
                if message["type"] == "message":
                    try:
                        event_data = json.loads(message["data"])
                        callback(event_data)
                    except json.JSONDecodeError:
                        continue
                        
        except Exception as e:
            logger.error(f"Failed to subscribe to events: {e}")


# Global queue service instance
queue_service = QueueService()