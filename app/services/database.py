"""
Database service for managing database connections and operations.
"""
import logging
from contextlib import contextmanager
from typing import Generator, Optional
from sqlalchemy import create_engine, text
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.pool import StaticPool

from app.config import settings
from app.models.database import Base

logger = logging.getLogger(__name__)


class DatabaseService:
    """Database service for managing connections and operations."""
    
    def __init__(self):
        """Initialize database service."""
        self.engine = None
        self.SessionLocal = None
        self._initialize_engine()
    
    def _initialize_engine(self):
        """Initialize SQLAlchemy engine and session factory."""
        try:
            # Create engine with connection pooling
            self.engine = create_engine(
                settings.database_url,
                pool_pre_ping=True,
                pool_recycle=300,
                pool_size=10,
                max_overflow=20,
                echo=settings.debug
            )
            
            # Create session factory
            self.SessionLocal = sessionmaker(
                autocommit=False,
                autoflush=False,
                bind=self.engine
            )
            
            logger.info("Database engine initialized successfully")
            
        except Exception as e:
            logger.warning(f"Failed to initialize database engine: {e}")
            # Don't raise during import, will be handled at runtime
            self.engine = None
            self.SessionLocal = None
    
    def create_tables(self):
        """Create all database tables."""
        if self.engine is None:
            logger.error("Database engine not initialized")
            return
        try:
            Base.metadata.create_all(bind=self.engine)
            logger.info("Database tables created successfully")
        except Exception as e:
            logger.error(f"Failed to create database tables: {e}")
            raise
    
    def drop_tables(self):
        """Drop all database tables."""
        try:
            Base.metadata.drop_all(bind=self.engine)
            logger.info("Database tables dropped successfully")
        except Exception as e:
            logger.error(f"Failed to drop database tables: {e}")
            raise
    
    @contextmanager
    def get_db(self) -> Generator[Session, None, None]:
        """Get database session context manager."""
        if self.SessionLocal is None:
            raise Exception("Database not initialized")
        session = self.SessionLocal()
        try:
            yield session
            session.commit()
        except Exception as e:
            session.rollback()
            logger.error(f"Database session error: {e}")
            raise
        finally:
            session.close()
    
    def get_db_dependency(self) -> Session:
        """Get database session for FastAPI dependency injection."""
        if self.SessionLocal is None:
            raise Exception("Database not initialized")
        return self.SessionLocal()
    
    def get_session(self) -> Session:
        """Get database session (manual management required)."""
        return self.SessionLocal()
    
    def test_connection(self) -> bool:
        """Test database connection."""
        if self.engine is None:
            return False
        try:
            with self.get_db() as db:
                db.execute(text("SELECT 1"))
            logger.info("Database connection test successful")
            return True
        except Exception as e:
            logger.error(f"Database connection test failed: {e}")
            return False
    
    def get_connection_info(self) -> dict:
        """Get database connection information."""
        try:
            with self.get_db() as db:
                result = db.execute(text("SELECT version()")).scalar()
                return {
                    "connected": True,
                    "version": result,
                    "url": settings.database_url.split("@")[-1] if "@" in settings.database_url else "hidden"
                }
        except Exception as e:
            return {
                "connected": False,
                "error": str(e),
                "url": settings.database_url.split("@")[-1] if "@" in settings.database_url else "hidden"
            }


# Global database service instance
db_service = DatabaseService()