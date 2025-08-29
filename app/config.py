"""
Configuration settings for the FastAPI application.
"""
import os
from typing import Dict, Any
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    """Application settings."""
    
    # Database
    database_url: str = "postgresql://codejudge:codejudge@localhost:5432/codejudge"
    
    # Redis
    redis_url: str = "redis://localhost:6379/0"
    
    # Rustbox
    rustbox_binary_path: str = "/usr/local/bin/rustbox"
    rustbox_work_dir: str = "/tmp/rustbox"
    
    # Server
    host: str = "0.0.0.0"
    port: int = 8000
    debug: bool = False
    log_level: str = "INFO"
    
    # Service mode
    service_mode: str = "server"
    
    # Security
    secret_key: str = "your-secret-key-change-in-production"
    access_token_expire_minutes: int = 30
    
    # Rate limiting
    rate_limit_per_minute: int = 60
    
    # Resource limits
    default_memory_limit_mb: int = 512
    default_time_limit_seconds: int = 10
    default_cpu_limit_seconds: int = 5
    max_memory_limit_mb: int = 2048
    max_time_limit_seconds: int = 60
    
    # Worker settings
    worker_concurrency: int = 4
    worker_prefetch_multiplier: int = 1
    
    # CORS
    cors_origins: str = '["http://localhost:3000", "http://localhost:8080"]'
    
    class Config:
        env_file = ".env"
        case_sensitive = False


# Global settings instance
settings = Settings()

# Language configurations
LANGUAGES: Dict[int, Dict[str, Any]] = {
    1: {
        "name": "Python",
        "version": "3.11.1",
        "extension": ".py",
        "compile_command": None,
        "run_command": "/usr/local/bin/python3",
        "rustbox_language": "python",
        "is_active": True
    },
    2: {
        "name": "C++",
        "version": "9.2.0",
        "extension": ".cpp",
        "compile_command": "g++ -o {executable} {source}",
        "run_command": "./{executable}",
        "rustbox_language": "cpp",
        "is_active": True
    },
    3: {
        "name": "Java",
        "version": "13.0.1",
        "extension": ".java",
        "compile_command": "javac {source}",
        "run_command": "java {class_name}",
        "rustbox_language": "java",
        "is_active": True
    }
}

# Status codes
STATUS_CODES = {
    "In Queue": 1,
    "Processing": 2,
    "Accepted": 3,
    "Wrong Answer": 4,
    "Time Limit Exceeded": 5,
    "Compilation Error": 6,
    "Runtime Error (SIGSEGV)": 7,
    "Runtime Error (SIGXFSZ)": 8,
    "Runtime Error (SIGFPE)": 9,
    "Runtime Error (SIGABRT)": 10,
    "Runtime Error (NZEC)": 11,
    "Runtime Error (Other)": 12,
    "Internal Error": 13,
    "Exec Format Error": 14
}