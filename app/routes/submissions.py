"""
Submission API routes.
"""
import logging
from typing import List, Optional
from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy.orm import Session
from sqlalchemy import select, func, desc

from app.models.database import Submission, Language, Status
from app.models.schemas import (
    SubmissionCreate, SubmissionUpdate, Submission as SubmissionSchema,
    SubmissionBatch, SubmissionList, ExecutionResult
)
from app.services.database import db_service
from app.services.queue_service import queue_service
from app.services.rustbox_service import rustbox_service
from app.config import settings, STATUS_CODES

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/submissions", tags=["submissions"])


@router.post("/", response_model=SubmissionSchema, status_code=status.HTTP_201_CREATED)
async def create_submission(
    submission: SubmissionCreate,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Create a new code submission."""
    try:
        # Validate language exists
        language = db.execute(
            select(Language).where(Language.id == submission.language_id)
        ).scalar_one_or_none()
        
        if not language:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Language with ID {submission.language_id} not found"
            )
        
        # Apply default limits if not provided
        time_limit = submission.time_limit or settings.default_time_limit_seconds
        memory_limit = submission.memory_limit or settings.default_memory_limit_mb
        
        # Validate limits
        if time_limit > settings.max_time_limit_seconds:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Time limit cannot exceed {settings.max_time_limit_seconds} seconds"
            )
        
        if memory_limit > settings.max_memory_limit_mb:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Memory limit cannot exceed {settings.max_memory_limit_mb} MB"
            )
        
        # Create submission record
        db_submission = Submission(
            source_code=submission.source_code,
            language_id=submission.language_id,
            stdin=submission.stdin,
            expected_output=submission.expected_output,
            time_limit=time_limit,
            memory_limit=memory_limit,
            status_id=STATUS_CODES["In Queue"]
        )
        
        db.add(db_submission)
        db.commit()
        db.refresh(db_submission)
        
        # Enqueue for processing
        queue_service.enqueue_submission(db_submission.id)
        
        logger.info(f"Created submission {db_submission.id}")
        return db_submission
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to create submission: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to create submission"
        )


@router.post("/batch", response_model=List[SubmissionSchema], status_code=status.HTTP_201_CREATED)
async def create_batch_submissions(
    batch: SubmissionBatch,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Create multiple submissions in batch."""
    try:
        created_submissions = []
        
        for submission_data in batch.submissions:
            # Validate language exists
            language = db.execute(
                select(Language).where(Language.id == submission_data.language_id)
            ).scalar_one_or_none()
            
            if not language:
                raise HTTPException(
                    status_code=status.HTTP_400_BAD_REQUEST,
                    detail=f"Language with ID {submission_data.language_id} not found"
                )
            
            # Apply default limits
            time_limit = submission_data.time_limit or settings.default_time_limit_seconds
            memory_limit = submission_data.memory_limit or settings.default_memory_limit_mb
            
            # Create submission
            db_submission = Submission(
                source_code=submission_data.source_code,
                language_id=submission_data.language_id,
                stdin=submission_data.stdin,
                expected_output=submission_data.expected_output,
                time_limit=time_limit,
                memory_limit=memory_limit,
                status_id=STATUS_CODES["In Queue"]
            )
            
            db.add(db_submission)
            created_submissions.append(db_submission)
        
        db.commit()
        
        # Enqueue all submissions
        for submission in created_submissions:
            db.refresh(submission)
            queue_service.enqueue_submission(submission.id)
        
        logger.info(f"Created {len(created_submissions)} batch submissions")
        return created_submissions
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to create batch submissions: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to create batch submissions"
        )


@router.get("/", response_model=SubmissionList)
async def list_submissions(
    page: int = Query(1, ge=1, description="Page number"),
    per_page: int = Query(10, ge=1, le=100, description="Items per page"),
    language_id: Optional[int] = Query(None, description="Filter by language ID"),
    status_id: Optional[int] = Query(None, description="Filter by status ID"),
    db: Session = Depends(db_service.get_db_dependency)
):
    """List submissions with pagination and filtering."""
    try:
        # Build query
        query = select(Submission)
        
        # Apply filters
        if language_id:
            query = query.where(Submission.language_id == language_id)
        if status_id:
            query = query.where(Submission.status_id == status_id)
        
        # Get total count
        count_query = select(func.count()).select_from(query.subquery())
        total = db.execute(count_query).scalar()
        
        # Apply pagination
        offset = (page - 1) * per_page
        query = query.order_by(desc(Submission.created_at)).offset(offset).limit(per_page)
        
        # Execute query
        submissions = db.execute(query).scalars().all()
        
        # Calculate pages
        pages = (total + per_page - 1) // per_page
        
        return SubmissionList(
            submissions=submissions,
            total=total,
            page=page,
            per_page=per_page,
            pages=pages
        )
        
    except Exception as e:
        logger.error(f"Failed to list submissions: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to list submissions"
        )


@router.get("/{submission_id}", response_model=SubmissionSchema)
async def get_submission(
    submission_id: int,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Get a specific submission by ID."""
    try:
        submission = db.execute(
            select(Submission).where(Submission.id == submission_id)
        ).scalar_one_or_none()
        
        if not submission:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Submission with ID {submission_id} not found"
            )
        
        return submission
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get submission {submission_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to get submission"
        )


@router.put("/{submission_id}", response_model=SubmissionSchema)
async def update_submission(
    submission_id: int,
    submission_update: SubmissionUpdate,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Update a submission."""
    try:
        submission = db.execute(
            select(Submission).where(Submission.id == submission_id)
        ).scalar_one_or_none()
        
        if not submission:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Submission with ID {submission_id} not found"
            )
        
        # Check if submission is already processed
        if submission.status_id != STATUS_CODES["In Queue"]:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Cannot update submission that is already processed"
            )
        
        # Update fields
        update_data = submission_update.dict(exclude_unset=True)
        for field, value in update_data.items():
            setattr(submission, field, value)
        
        db.commit()
        db.refresh(submission)
        
        logger.info(f"Updated submission {submission_id}")
        return submission
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update submission {submission_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to update submission"
        )


@router.delete("/{submission_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_submission(
    submission_id: int,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Delete a submission."""
    try:
        submission = db.execute(
            select(Submission).where(Submission.id == submission_id)
        ).scalar_one_or_none()
        
        if not submission:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Submission with ID {submission_id} not found"
            )
        
        db.delete(submission)
        db.commit()
        
        logger.info(f"Deleted submission {submission_id}")
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to delete submission {submission_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to delete submission"
        )


@router.post("/{submission_id}/execute", response_model=ExecutionResult)
async def execute_submission(
    submission_id: int,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Execute a submission immediately (for testing)."""
    try:
        submission = db.execute(
            select(Submission).where(Submission.id == submission_id)
        ).scalar_one_or_none()
        
        if not submission:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Submission with ID {submission_id} not found"
            )
        
        # Check if rustbox is available
        if not rustbox_service.is_available():
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail="Rustbox service is not available"
            )
        
        # Update status to processing
        submission.status_id = STATUS_CODES["Processing"]
        submission.started_at = db.execute(select(func.now())).scalar()
        db.commit()
        
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
        submission.wall_time = result.wall_time
        submission.cpu_time = result.cpu_time
        submission.memory_peak = result.memory_peak_kb
        submission.exit_code = result.exit_code
        submission.signal = result.signal
        submission.error_message = result.error_message
        submission.finished_at = db.execute(select(func.now())).scalar()
        
        db.commit()
        
        logger.info(f"Executed submission {submission_id} with status {result.status}")
        return result
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to execute submission {submission_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to execute submission"
        )