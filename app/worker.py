"""
Background worker for processing code submissions.
"""
import asyncio
import logging
import signal
import sys
import time
import uuid
from datetime import datetime
from typing import Optional

from sqlalchemy.orm import Session
from sqlalchemy import select, func

from app.config import settings, STATUS_CODES
from app.models.database import Submission, Language, Status
from app.services.database import db_service
from app.services.queue_service import queue_service
from app.services.rustbox_service import rustbox_service

# Configure logging
logging.basicConfig(
    level=getattr(logging, settings.log_level.upper()),
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


class Worker:
    """Background worker for processing submissions."""
    
    def __init__(self, worker_id: Optional[str] = None):
        """Initialize worker."""
        self.worker_id = worker_id or str(uuid.uuid4())
        self.running = False
        self.concurrency = settings.worker_concurrency
        self.semaphore = asyncio.Semaphore(self.concurrency)
        
        # Setup signal handlers
        signal.signal(signal.SIGINT, self._signal_handler)
        signal.signal(signal.SIGTERM, self._signal_handler)
    
    def _signal_handler(self, signum, frame):
        """Handle shutdown signals."""
        logger.info(f"Received signal {signum}, shutting down worker {self.worker_id}")
        self.running = False
    
    async def start(self):
        """Start the worker."""
        logger.info(f"Starting worker {self.worker_id} with concurrency {self.concurrency}")
        self.running = True
        
        # Register worker
        queue_service.set_worker_status(
            self.worker_id,
            "running",
            {"concurrency": self.concurrency, "started_at": datetime.utcnow().isoformat()}
        )
        
        try:
            while self.running:
                try:
                    # Check for new jobs
                    job_data = queue_service.dequeue_submission()
                    
                    if job_data:
                        # Process job asynchronously
                        asyncio.create_task(self._process_submission(job_data))
                    else:
                        # No jobs available, wait a bit
                        await asyncio.sleep(1)
                
                except Exception as e:
                    logger.error(f"Error in worker main loop: {e}")
                    await asyncio.sleep(5)  # Wait before retrying
        
        finally:
            # Unregister worker
            queue_service.set_worker_status(self.worker_id, "stopped")
            logger.info(f"Worker {self.worker_id} stopped")
    
    async def _process_submission(self, job_data: dict):
        """Process a single submission."""
        async with self.semaphore:
            submission_id = job_data.get("submission_id")
            job_id = job_data.get("job_id")
            
            logger.info(f"Processing submission {submission_id} (job {job_id})")
            
            try:
                # Update worker status
                queue_service.set_worker_status(
                    self.worker_id,
                    "processing",
                    {"submission_id": submission_id, "job_id": job_id}
                )
                
                # Process the submission
                await self._execute_submission(submission_id)
                
                logger.info(f"Completed processing submission {submission_id}")
                
            except Exception as e:
                logger.error(f"Failed to process submission {submission_id}: {e}")
                
                # Mark submission as failed
                try:
                    with db_service.get_db() as db:
                        submission = db.execute(
                            select(Submission).where(Submission.id == submission_id)
                        ).scalar_one_or_none()
                        
                        if submission:
                            submission.status_id = STATUS_CODES["Internal Error"]
                            submission.error_message = str(e)
                            submission.finished_at = datetime.utcnow()
                            db.commit()
                except Exception as db_error:
                    logger.error(f"Failed to update submission status: {db_error}")
            
            finally:
                # Update worker status back to running
                queue_service.set_worker_status(self.worker_id, "running")
    
    async def _execute_submission(self, submission_id: int):
        """Execute a submission using rustbox."""
        with db_service.get_db() as db:
            # Get submission
            submission = db.execute(
                select(Submission).where(Submission.id == submission_id)
            ).scalar_one_or_none()
            
            if not submission:
                logger.error(f"Submission {submission_id} not found")
                return
            
            # Check if already processed
            if submission.status_id != STATUS_CODES["In Queue"]:
                logger.warning(f"Submission {submission_id} already processed")
                return
            
            # Update status to processing
            submission.status_id = STATUS_CODES["Processing"]
            submission.started_at = datetime.utcnow()
            db.commit()
            
            try:
                # Check if rustbox is available
                if not rustbox_service.is_available():
                    raise Exception("Rustbox service is not available")
                
                # Execute code
                result = rustbox_service.execute_code(
                    source_code=submission.source_code,
                    language_id=submission.language_id,
                    stdin=submission.stdin,
                    time_limit=submission.time_limit,
                    memory_limit=submission.memory_limit
                )
                
                # Update submission with results
                submission.status_id = STATUS_CODES.get(result.status, STATUS_CODES["Runtime Error (Other)"])
                submission.stdout = result.stdout
                submission.stderr = result.stderr
                submission.compile_output = result.compile_output
                submission.wall_time = result.wall_time
                submission.cpu_time = result.cpu_time
                submission.memory_peak = result.memory_peak_kb
                submission.exit_code = result.exit_code
                submission.signal = result.signal
                submission.error_message = result.error_message
                submission.finished_at = datetime.utcnow()
                
                # Store execution metadata
                submission.execution_metadata = {
                    "worker_id": self.worker_id,
                    "execution_time": datetime.utcnow().isoformat(),
                    "success": result.success
                }
                
                db.commit()
                
                logger.info(f"Submission {submission_id} executed with status: {result.status}")
                
            except Exception as e:
                # Mark as internal error
                submission.status_id = STATUS_CODES["Internal Error"]
                submission.error_message = str(e)
                submission.finished_at = datetime.utcnow()
                db.commit()
                
                logger.error(f"Failed to execute submission {submission_id}: {e}")
                raise


async def main():
    """Main worker entry point."""
    worker_id = f"worker-{uuid.uuid4().hex[:8]}"
    worker = Worker(worker_id)
    
    try:
        await worker.start()
    except KeyboardInterrupt:
        logger.info("Worker interrupted by user")
    except Exception as e:
        logger.error(f"Worker failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())