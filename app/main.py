"""
Main FastAPI application.
"""
import logging
from contextlib import asynccontextmanager
from fastapi import FastAPI, HTTPException, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from app.config import settings
from app.services.database import db_service
from app.services.queue_service import queue_service
from app.services.rustbox_service import rustbox_service
from app.routes import submissions, languages, system

# Configure logging
logging.basicConfig(
    level=getattr(logging, settings.log_level.upper()),
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Application lifespan manager."""
    # Startup
    logger.info("Starting up Rustbox API...")
    
    try:
        # Initialize database
        logger.info("Initializing database...")
        db_service.create_tables()
        
        # Test connections
        if not db_service.test_connection():
            logger.error("Database connection failed")
            raise Exception("Database connection failed")
        
        if not queue_service.test_connection():
            logger.error("Redis connection failed")
            raise Exception("Redis connection failed")
        
        if not rustbox_service.is_available():
            logger.warning("Rustbox service is not available")
        
        logger.info("Rustbox API started successfully")
        
    except Exception as e:
        logger.error(f"Failed to start Rustbox API: {e}")
        raise
    
    yield
    
    # Shutdown
    logger.info("Shutting down Rustbox API...")


# Create FastAPI application
app = FastAPI(
    title="Rustbox - Secure Code Execution Platform",
    description="A secure, scalable code execution system built with FastAPI, PostgreSQL, Redis, and powered by the rustbox sandboxing engine.",
    version="1.0.0",
    docs_url="/docs",
    redoc_url="/redoc",
    lifespan=lifespan
)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000", "http://localhost:8080"] if not settings.debug else ["*"],
    allow_credentials=True,
    allow_methods=["GET", "POST", "PUT", "DELETE", "OPTIONS"],
    allow_headers=["*"],
)


# Global exception handler
@app.exception_handler(Exception)
async def global_exception_handler(request, exc):
    """Global exception handler."""
    logger.error(f"Unhandled exception: {exc}", exc_info=True)
    return JSONResponse(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        content={
            "error": "Internal Server Error",
            "message": "An unexpected error occurred",
            "details": str(exc) if settings.debug else None
        }
    )


# Root endpoint
@app.get("/")
async def root():
    """Root endpoint."""
    return {
        "message": "Rustbox - Secure Code Execution Platform",
        "version": "1.0.0",
        "status": "running",
        "docs": "/docs",
        "health": "/system/health"
    }


# Ping endpoint
@app.get("/ping")
async def ping():
    """Health check endpoint."""
    from datetime import datetime
    return {"status": "ok", "timestamp": datetime.utcnow().isoformat() + "Z"}


# Include routers
app.include_router(submissions.router)
app.include_router(languages.router)
app.include_router(system.router)


if __name__ == "__main__":
    import uvicorn
    
    uvicorn.run(
        "app.main:app",
        host=settings.host,
        port=settings.port,
        reload=settings.debug,
        log_level=settings.log_level.lower()
    )