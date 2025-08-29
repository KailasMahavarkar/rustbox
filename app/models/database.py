"""
SQLAlchemy database models.
"""
from datetime import datetime
from typing import Optional
from sqlalchemy import Column, Integer, String, Text, DateTime, Boolean, ForeignKey, JSON
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship

Base = declarative_base()


class Language(Base):
    """Programming language model."""
    __tablename__ = "languages"
    
    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(50), nullable=False, unique=True)
    version = Column(String(20), nullable=False)
    extension = Column(String(10), nullable=False)
    compile_command = Column(Text, nullable=True)
    run_command = Column(String(200), nullable=False)
    rustbox_language = Column(String(20), nullable=False)
    is_active = Column(Boolean, default=True)
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)


class Status(Base):
    """Submission status model."""
    __tablename__ = "statuses"
    
    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(50), nullable=False, unique=True)
    description = Column(Text, nullable=True)
    created_at = Column(DateTime, default=datetime.utcnow)


class Submission(Base):
    """Code submission model."""
    __tablename__ = "submissions"
    
    id = Column(Integer, primary_key=True, index=True)
    source_code = Column(Text, nullable=False)
    language_id = Column(Integer, ForeignKey("languages.id"), nullable=False)
    stdin = Column(Text, nullable=True)
    expected_output = Column(Text, nullable=True)
    stdout = Column(Text, nullable=True)
    stderr = Column(Text, nullable=True)
    compile_output = Column(Text, nullable=True)
    status_id = Column(Integer, ForeignKey("statuses.id"), nullable=False)
    
    # Execution details
    time_limit = Column(Integer, nullable=True)  # in seconds
    memory_limit = Column(Integer, nullable=True)  # in MB
    wall_time = Column(Integer, nullable=True)  # in milliseconds
    cpu_time = Column(Integer, nullable=True)  # in milliseconds
    memory_peak = Column(Integer, nullable=True)  # in KB
    
    # Additional metadata
    exit_code = Column(Integer, nullable=True)
    signal = Column(Integer, nullable=True)
    error_message = Column(Text, nullable=True)
    execution_metadata = Column(JSON, nullable=True)
    
    # Timestamps
    created_at = Column(DateTime, default=datetime.utcnow)
    started_at = Column(DateTime, nullable=True)
    finished_at = Column(DateTime, nullable=True)
    
    # Relationships
    language = relationship("Language", backref="submissions")
    status = relationship("Status", backref="submissions")


class SystemStats(Base):
    """System statistics model."""
    __tablename__ = "system_stats"
    
    id = Column(Integer, primary_key=True, index=True)
    timestamp = Column(DateTime, default=datetime.utcnow)
    active_submissions = Column(Integer, default=0)
    total_submissions = Column(Integer, default=0)
    queue_size = Column(Integer, default=0)
    worker_count = Column(Integer, default=0)
    memory_usage_mb = Column(Integer, default=0)
    cpu_usage_percent = Column(Integer, default=0)
    extra_metadata = Column(JSON, nullable=True)