"""
Pydantic schemas for API request/response models.
"""
from datetime import datetime
from typing import Optional, List, Dict, Any
from pydantic import BaseModel, Field


class LanguageBase(BaseModel):
    """Base language schema."""
    name: str
    version: str
    extension: str
    compile_command: Optional[str] = None
    run_command: str
    rustbox_language: str
    is_active: bool = True


class LanguageCreate(LanguageBase):
    """Language creation schema."""
    pass


class LanguageUpdate(BaseModel):
    """Language update schema."""
    name: Optional[str] = None
    version: Optional[str] = None
    extension: Optional[str] = None
    compile_command: Optional[str] = None
    run_command: Optional[str] = None
    rustbox_language: Optional[str] = None
    is_active: Optional[bool] = None


class Language(LanguageBase):
    """Language response schema."""
    id: int
    created_at: datetime
    updated_at: Optional[datetime] = None
    
    class Config:
        from_attributes = True


class StatusBase(BaseModel):
    """Base status schema."""
    name: str
    description: Optional[str] = None


class StatusCreate(StatusBase):
    """Status creation schema."""
    pass


class Status(StatusBase):
    """Status response schema."""
    id: int
    created_at: datetime
    
    class Config:
        from_attributes = True


class SubmissionBase(BaseModel):
    """Base submission schema."""
    source_code: str
    language_id: int
    stdin: Optional[str] = None
    expected_output: Optional[str] = None
    time_limit: Optional[int] = Field(None, ge=1, le=60, description="Time limit in seconds")
    memory_limit: Optional[int] = Field(None, ge=1, le=2048, description="Memory limit in MB")


class SubmissionCreate(SubmissionBase):
    """Submission creation schema."""
    pass


class SubmissionUpdate(BaseModel):
    """Submission update schema."""
    source_code: Optional[str] = None
    language_id: Optional[int] = None
    stdin: Optional[str] = None
    expected_output: Optional[str] = None
    time_limit: Optional[int] = Field(None, ge=1, le=60)
    memory_limit: Optional[int] = Field(None, ge=1, le=2048)


class Submission(SubmissionBase):
    """Submission response schema."""
    id: int
    stdout: Optional[str] = None
    stderr: Optional[str] = None
    compile_output: Optional[str] = None
    status_id: int
    wall_time: Optional[float] = None
    cpu_time: Optional[float] = None
    memory_peak: Optional[int] = None
    exit_code: Optional[int] = None
    signal: Optional[int] = None
    error_message: Optional[str] = None
    execution_metadata: Optional[Dict[str, Any]] = None
    created_at: datetime
    started_at: Optional[datetime] = None
    finished_at: Optional[datetime] = None
    language: Language
    status: Status
    
    class Config:
        from_attributes = True


class SubmissionBatch(BaseModel):
    """Batch submission schema."""
    submissions: List[SubmissionCreate]


class SubmissionList(BaseModel):
    """Submission list response schema."""
    submissions: List[Submission]
    total: int
    page: int
    per_page: int
    pages: int


class ExecutionResult(BaseModel):
    """Execution result schema."""
    status: str
    exit_code: Optional[int] = None
    stdout: Optional[str] = None
    stderr: Optional[str] = None
    compile_output: Optional[str] = None
    wall_time: Optional[float] = None
    cpu_time: Optional[float] = None
    memory_peak_kb: Optional[int] = None
    success: bool
    signal: Optional[int] = None
    error_message: Optional[str] = None
    language: Optional[str] = None


class SystemHealth(BaseModel):
    """System health check schema."""
    status: str
    timestamp: datetime
    version: str
    uptime_seconds: int
    database_connected: bool
    redis_connected: bool
    rustbox_available: bool
    worker_count: int
    queue_size: int


class SystemInfo(BaseModel):
    """System information schema."""
    version: str
    languages: List[Language]
    statuses: List[Status]
    limits: Dict[str, Any]
    features: List[str]


class SystemStats(BaseModel):
    """System statistics schema."""
    timestamp: datetime
    active_submissions: int
    total_submissions: int
    queue_size: int
    worker_count: int
    memory_usage_mb: int
    cpu_usage_percent: int
    submissions_by_status: Dict[str, int]
    submissions_by_language: Dict[str, int]
    average_execution_time_ms: Optional[float] = None
    success_rate_percent: Optional[float] = None


class ErrorResponse(BaseModel):
    """Error response schema."""
    error: str
    message: str
    details: Optional[Dict[str, Any]] = None