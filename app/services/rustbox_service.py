"""
Rustbox service for executing code in sandboxed environments.
"""
import json
import logging
import os
import subprocess
import tempfile
import uuid
from typing import Optional, Dict, Any
from datetime import datetime

from app.config import settings, LANGUAGES
from app.models.schemas import ExecutionResult

logger = logging.getLogger(__name__)


class RustboxService:
    """Service for executing code using rustbox sandboxing."""
    
    def __init__(self):
        """Initialize rustbox service."""
        self.binary_path = settings.rustbox_binary_path
        self.work_dir = settings.rustbox_work_dir
        self._ensure_work_directory()
    
    def _ensure_work_directory(self):
        """Ensure rustbox work directory exists."""
        try:
            os.makedirs(self.work_dir, exist_ok=True)
            logger.info(f"Rustbox work directory: {self.work_dir}")
        except Exception as e:
            logger.error(f"Failed to create rustbox work directory: {e}")
            raise
    
    def is_available(self) -> bool:
        """Check if rustbox binary is available."""
        try:
            result = subprocess.run(
                [self.binary_path, "--help"],
                capture_output=True,
                timeout=5
            )
            return result.returncode == 0
        except (subprocess.TimeoutExpired, FileNotFoundError, PermissionError):
            return False
    
    def get_info(self) -> Dict[str, Any]:
        """Get rustbox service information."""
        return {
            "available": self.is_available(),
            "binary_path": self.binary_path,
            "work_dir": self.work_dir,
            "binary_exists": os.path.exists(self.binary_path),
            "work_dir_writable": os.access(self.work_dir, os.W_OK)
        }
    
    def execute_code(
        self,
        source_code: str,
        language_id: int,
        stdin: Optional[str] = None,
        time_limit: Optional[int] = None,
        memory_limit: Optional[int] = None
    ) -> ExecutionResult:
        """Execute code in rustbox sandbox."""
        try:
            # Get language configuration
            if language_id not in LANGUAGES:
                raise ValueError(f"Unsupported language ID: {language_id}")
            
            lang_config = LANGUAGES[language_id]
            rustbox_language = lang_config["rustbox_language"]
            
            # Generate unique box ID
            box_id = uuid.uuid4().int % 1000000
            
            # Prepare rustbox command
            cmd = [
                self.binary_path,
                "execute-code",
                "--box-id", str(box_id),
                "--language", rustbox_language,
                "--code", source_code
            ]
            
            # Add resource limits
            if time_limit:
                cmd.extend(["--time", str(time_limit)])
            if memory_limit:
                cmd.extend(["--mem", str(memory_limit)])
            
            # Add stdin if provided
            if stdin:
                cmd.extend(["--stdin", stdin])
            
            # Add strict mode for security (only if running as root)
            import os
            if os.geteuid() == 0:  # Running as root
                cmd.append("--strict")
            
            logger.info(f"Executing rustbox command: {' '.join(cmd[:6])}...")
            
            # Execute rustbox
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=time_limit + 10 if time_limit else 30
            )
            
            # Parse rustbox output
            if result.returncode == 0:
                try:
                    rustbox_output = json.loads(result.stdout)
                    return self._parse_rustbox_output(rustbox_output, language_id)
                except json.JSONDecodeError:
                    logger.error(f"Failed to parse rustbox JSON output: {result.stdout}")
                    return ExecutionResult(
                        status="Internal Error",
                        success=False,
                        error_message="Failed to parse execution output"
                    )
            else:
                logger.error(f"Rustbox execution failed: {result.stderr}")
                return ExecutionResult(
                    status="Internal Error",
                    success=False,
                    error_message=f"Rustbox execution failed: {result.stderr}"
                )
                
        except subprocess.TimeoutExpired:
            logger.error("Rustbox execution timed out")
            return ExecutionResult(
                status="Time Limit Exceeded",
                success=False,
                error_message="Execution timed out"
            )
        except Exception as e:
            logger.error(f"Rustbox execution error: {e}")
            return ExecutionResult(
                status="Internal Error",
                success=False,
                error_message=str(e)
            )
    
    def _parse_rustbox_output(self, rustbox_output: Dict[str, Any], language_id: int) -> ExecutionResult:
        """Parse rustbox output into ExecutionResult."""
        try:
            # Map rustbox status to our status codes
            status_mapping = {
                "TLE": "Time Limit Exceeded",
                "Memory Limit Exceeded": "Runtime Error (SIGSEGV)",
                "Success": "Accepted",
                "Runtime Error": "Runtime Error (Other)",
                "Compilation Error": "Compilation Error"
            }
            
            rustbox_status = rustbox_output.get("status", "Unknown")
            mapped_status = status_mapping.get(rustbox_status, "Runtime Error (Other)")
            
            # Determine success based on status and exit code
            success = (
                mapped_status == "Accepted" and 
                rustbox_output.get("exit_code", 1) == 0
            )
            
            return ExecutionResult(
                status=mapped_status,
                exit_code=rustbox_output.get("exit_code"),
                stdout=rustbox_output.get("stdout"),
                stderr=rustbox_output.get("stderr"),
                wall_time=rustbox_output.get("wall_time"),
                cpu_time=rustbox_output.get("cpu_time"),
                memory_peak_kb=rustbox_output.get("memory_peak_kb"),
                success=success,
                signal=rustbox_output.get("signal"),
                error_message=rustbox_output.get("error_message"),
                language=LANGUAGES[language_id]["name"]
            )
            
        except Exception as e:
            logger.error(f"Failed to parse rustbox output: {e}")
            return ExecutionResult(
                status="Internal Error",
                success=False,
                error_message=f"Failed to parse execution result: {e}"
            )
    
    def test_execution(self) -> Dict[str, Any]:
        """Test rustbox execution with a simple Python program."""
        try:
            test_code = "print('Hello, World!')"
            result = self.execute_code(
                source_code=test_code,
                language_id=1,  # Python
                time_limit=5,
                memory_limit=128
            )
            
            return {
                "test_passed": result.success and result.stdout and "Hello, World!" in result.stdout,
                "result": result.model_dump(),
                "rustbox_available": self.is_available()
            }
            
        except Exception as e:
            return {
                "test_passed": False,
                "error": str(e),
                "rustbox_available": self.is_available()
            }


# Global rustbox service instance
rustbox_service = RustboxService()