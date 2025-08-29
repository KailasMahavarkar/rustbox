"""
Database initialization script.
"""
import asyncio
import logging
from sqlalchemy import select

from app.config import settings, LANGUAGES, STATUS_CODES
from app.models.database import Language, Status
from app.services.database import db_service

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


async def init_database():
    """Initialize database with default data."""
    try:
        logger.info("Initializing database...")
        
        with db_service.get_db() as db:
            # Create statuses
            logger.info("Creating statuses...")
            for status_name, status_id in STATUS_CODES.items():
                existing = db.execute(select(Status).where(Status.id == status_id)).scalar_one_or_none()
                if not existing:
                    status = Status(
                        id=status_id,
                        name=status_name,
                        description=f"Submission status: {status_name}"
                    )
                    db.add(status)
                    logger.info(f"Created status: {status_name}")
            
            # Create languages
            logger.info("Creating languages...")
            for lang_id, lang_config in LANGUAGES.items():
                existing = db.execute(select(Language).where(Language.id == lang_id)).scalar_one_or_none()
                if not existing:
                    language = Language(
                        id=lang_id,
                        name=lang_config["name"],
                        version=lang_config["version"],
                        extension=lang_config["extension"],
                        compile_command=lang_config["compile_command"],
                        run_command=lang_config["run_command"],
                        rustbox_language=lang_config["rustbox_language"],
                        is_active=True
                    )
                    db.add(language)
                    logger.info(f"Created language: {lang_config['name']}")
            
            db.commit()
            logger.info("Database initialization completed successfully")
            
    except Exception as e:
        logger.error(f"Database initialization failed: {e}")
        raise


if __name__ == "__main__":
    asyncio.run(init_database())
