"""
System API routes for health checks, info, and statistics.
"""
import logging
import time
from datetime import datetime, timedelta
from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy.orm import Session
from sqlalchemy import select, func, desc

from app.models.database import Submission, Language, Status, SystemStats
from app.models.schemas import SystemHealth, SystemInfo, SystemStats as SystemStatsSchema
from app.services.database import db_service
from app.services.queue_service import queue_service
from app.services.rustbox_service import rustbox_service
from app.config import settings, LANGUAGES, STATUS_CODES

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/system", tags=["system"])

# Track application start time
app_start_time = time.time()


@router.get("/health", response_model=SystemHealth)
async def health_check():
    """Get system health status."""
    try:
        # Test database connection
        db_connected = db_service.test_connection()
        
        # Test Redis connection
        redis_connected = queue_service.test_connection()
        
        # Check rustbox availability
        rustbox_available = rustbox_service.is_available()
        
        # Get worker count
        worker_count = queue_service.get_worker_count()
        
        # Get queue size
        queue_size = queue_service.get_total_queue_size()
        
        # Calculate uptime
        uptime_seconds = int(time.time() - app_start_time)
        
        # Determine overall status
        overall_status = "healthy"
        if not db_connected or not redis_connected or not rustbox_available:
            overall_status = "unhealthy"
        elif queue_size > 100:  # High queue size
            overall_status = "degraded"
        
        return SystemHealth(
            status=overall_status,
            timestamp=datetime.utcnow(),
            version="1.0.0",
            uptime_seconds=uptime_seconds,
            database_connected=db_connected,
            redis_connected=redis_connected,
            rustbox_available=rustbox_available,
            worker_count=worker_count,
            queue_size=queue_size
        )
        
    except Exception as e:
        logger.error(f"Health check failed: {e}")
        return SystemHealth(
            status="unhealthy",
            timestamp=datetime.utcnow(),
            version="1.0.0",
            uptime_seconds=int(time.time() - app_start_time),
            database_connected=False,
            redis_connected=False,
            rustbox_available=False,
            worker_count=0,
            queue_size=0
        )


@router.get("/info", response_model=SystemInfo)
async def system_info(db: Session = Depends(db_service.get_db_dependency)):
    """Get system information."""
    try:
        # Get languages
        languages = db.execute(
            select(Language).where(Language.is_active == True)
        ).scalars().all()
        
        # Get statuses
        statuses = db.execute(select(Status)).scalars().all()
        
        # System limits
        limits = {
            "max_memory_limit_mb": settings.max_memory_limit_mb,
            "max_time_limit_seconds": settings.max_time_limit_seconds,
            "default_memory_limit_mb": settings.default_memory_limit_mb,
            "default_time_limit_seconds": settings.default_time_limit_seconds,
            "worker_concurrency": settings.worker_concurrency
        }
        
        # System features
        features = [
            "secure_code_execution",
            "multiple_languages",
            "resource_limits",
            "queue_management",
            "real_time_monitoring",
            "docker_deployment",
            "horizontal_scaling"
        ]
        
        return SystemInfo(
            version="1.0.0",
            languages=languages,
            statuses=statuses,
            limits=limits,
            features=features
        )
        
    except Exception as e:
        logger.error(f"Failed to get system info: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to get system information"
        )


@router.get("/stats", response_model=SystemStatsSchema)
async def system_stats(db: Session = Depends(db_service.get_db_dependency)):
    """Get detailed system statistics."""
    try:
        # Get basic counts
        total_submissions = db.execute(
            select(func.count(Submission.id))
        ).scalar()
        
        active_submissions = db.execute(
            select(func.count(Submission.id)).where(
                Submission.status_id.in_([
                    STATUS_CODES["In Queue"],
                    STATUS_CODES["Processing"]
                ])
            )
        ).scalar()
        
        # Get queue size
        queue_size = queue_service.get_total_queue_size()
        
        # Get worker count
        worker_count = queue_service.get_worker_count()
        
        # Get submissions by status
        status_stats = db.execute(
            select(
                Status.name,
                func.count(Submission.id)
            )
            .join(Submission, Status.id == Submission.status_id)
            .group_by(Status.id, Status.name)
        ).all()
        
        submissions_by_status = {name: count for name, count in status_stats}
        
        # Get submissions by language
        language_stats = db.execute(
            select(
                Language.name,
                func.count(Submission.id)
            )
            .join(Submission, Language.id == Submission.language_id)
            .group_by(Language.id, Language.name)
        ).all()
        
        submissions_by_language = {name: count for name, count in language_stats}
        
        # Calculate average execution time
        avg_execution_time = db.execute(
            select(func.avg(Submission.wall_time))
            .where(Submission.wall_time.isnot(None))
        ).scalar()
        
        # Calculate success rate
        successful_submissions = db.execute(
            select(func.count(Submission.id))
            .where(Submission.status_id == STATUS_CODES["Accepted"])
        ).scalar()
        
        success_rate = (
            (successful_submissions / total_submissions * 100) 
            if total_submissions > 0 else 0
        )
        
        # Get system resource usage (simplified)
        memory_usage_mb = 0  # Would need psutil or similar for real metrics
        cpu_usage_percent = 0  # Would need psutil or similar for real metrics
        
        return SystemStatsSchema(
            timestamp=datetime.utcnow(),
            active_submissions=active_submissions,
            total_submissions=total_submissions,
            queue_size=queue_size,
            worker_count=worker_count,
            memory_usage_mb=memory_usage_mb,
            cpu_usage_percent=cpu_usage_percent,
            submissions_by_status=submissions_by_status,
            submissions_by_language=submissions_by_language,
            average_execution_time_ms=avg_execution_time,
            success_rate_percent=success_rate
        )
        
    except Exception as e:
        logger.error(f"Failed to get system stats: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to get system statistics"
        )


@router.get("/test")
async def test_system():
    """Test system components."""
    try:
        results = {
            "database": db_service.get_connection_info(),
            "redis": queue_service.get_connection_info(),
            "rustbox": rustbox_service.get_info(),
            "test_execution": rustbox_service.test_execution()
        }
        
        return {
            "status": "ok",
            "timestamp": datetime.utcnow(),
            "results": results
        }
        
    except Exception as e:
        logger.error(f"System test failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"System test failed: {e}"
        )


@router.post("/cleanup")
async def cleanup_system():
    """Clean up system resources."""
    try:
        # Clear all queues
        cleared_queues = queue_service.clear_all_queues()
        
        # Reset worker statuses
        # This would require additional implementation
        
        return {
            "status": "ok",
            "timestamp": datetime.utcnow(),
            "cleared_queues": cleared_queues,
            "message": "System cleanup completed"
        }
        
    except Exception as e:
        logger.error(f"System cleanup failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"System cleanup failed: {e}"
        )