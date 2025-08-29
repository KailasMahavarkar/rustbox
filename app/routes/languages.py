"""
Language API routes.
"""
import logging
from typing import List
from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy.orm import Session
from sqlalchemy import select, func

from app.models.database import Language
from app.models.schemas import Language as LanguageSchema, LanguageCreate, LanguageUpdate
from app.services.database import db_service

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/languages", tags=["languages"])


@router.get("/", response_model=List[LanguageSchema])
async def list_languages(
    active_only: bool = True,
    db: Session = Depends(db_service.get_db_dependency)
):
    """List all supported programming languages."""
    try:
        query = select(Language)
        
        if active_only:
            query = query.where(Language.is_active == True)
        
        languages = db.execute(query.order_by(Language.id)).scalars().all()
        return languages
        
    except Exception as e:
        logger.error(f"Failed to list languages: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to list languages"
        )


@router.get("/{language_id}", response_model=LanguageSchema)
async def get_language(
    language_id: int,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Get a specific language by ID."""
    try:
        language = db.execute(
            select(Language).where(Language.id == language_id)
        ).scalar_one_or_none()
        
        if not language:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Language with ID {language_id} not found"
            )
        
        return language
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get language {language_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to get language"
        )


@router.post("/", response_model=LanguageSchema, status_code=status.HTTP_201_CREATED)
async def create_language(
    language: LanguageCreate,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Create a new programming language."""
    try:
        # Check if language with same name already exists
        existing = db.execute(
            select(Language).where(Language.name == language.name)
        ).scalar_one_or_none()
        
        if existing:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Language with name '{language.name}' already exists"
            )
        
        db_language = Language(**language.dict())
        db.add(db_language)
        db.commit()
        db.refresh(db_language)
        
        logger.info(f"Created language: {language.name}")
        return db_language
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to create language: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to create language"
        )


@router.put("/{language_id}", response_model=LanguageSchema)
async def update_language(
    language_id: int,
    language_update: LanguageUpdate,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Update a programming language."""
    try:
        language = db.execute(
            select(Language).where(Language.id == language_id)
        ).scalar_one_or_none()
        
        if not language:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Language with ID {language_id} not found"
            )
        
        # Update fields
        update_data = language_update.dict(exclude_unset=True)
        for field, value in update_data.items():
            setattr(language, field, value)
        
        db.commit()
        db.refresh(language)
        
        logger.info(f"Updated language {language_id}")
        return language
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to update language {language_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to update language"
        )


@router.delete("/{language_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_language(
    language_id: int,
    db: Session = Depends(db_service.get_db_dependency)
):
    """Delete a programming language."""
    try:
        language = db.execute(
            select(Language).where(Language.id == language_id)
        ).scalar_one_or_none()
        
        if not language:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Language with ID {language_id} not found"
            )
        
        # Check if language is used by any submissions
        from app.models.database import Submission
        submission_count = db.execute(
            select(func.count(Submission.id)).where(Submission.language_id == language_id)
        ).scalar()
        
        if submission_count > 0:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Cannot delete language that is used by {submission_count} submissions"
            )
        
        db.delete(language)
        db.commit()
        
        logger.info(f"Deleted language {language_id}")
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to delete language {language_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to delete language"
        )