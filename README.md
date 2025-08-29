# codejudge-like System with Rustbox

A secure, scalable code execution system built with FastAPI, PostgreSQL, Redis, and powered by the rustbox sandboxing engine. This system provides a codejudge-compatible API for safe execution of untrusted code with comprehensive resource limits and isolation.

## 🚀 Features

-   **Secure Code Execution**: Uses rustbox for process isolation and resource control
-   **codejudge-Compatible API**: Drop-in replacement for codejudge with similar endpoints
-   **Multiple Languages**: Support for Python, C++, C, Java, JavaScript, Rust, Go
-   **Scalable Architecture**: Horizontal scaling with multiple worker processes
-   **Queue Management**: Priority-based job queuing with Redis
-   **Resource Limits**: Configurable memory, CPU, and time limits
-   **Real-time Monitoring**: Health checks and system statistics
-   **Docker Deployment**: Complete containerized deployment with Docker Compose

## 📋 Prerequisites

-   Docker and Docker Compose
-   Rust (for building rustbox)
-   Linux system with cgroups support (for rustbox)

## 🛠️ Quick Start

### 1. Build Rustbox

First, build the rustbox binary from the rustbox-core project:

```bash
cd ../rustbox-core
cargo build --release
cd ../rustbox-api
```

### 2. Deploy the System

Use the deployment script to build and deploy the complete system:

```bash
chmod +x deploy.sh
./deploy.sh build
```

This will:

-   Build the rustbox binary
-   Copy it to the API directory
-   Start all services with Docker Compose
-   Run health checks

### 3. Test the API

Test the system with a simple Python program:

```bash
curl -X POST http://localhost:8000/submissions \
  -H "Content-Type: application/json" \
  -d '{
    "source_code": "print(\"Hello, World!\")",
    "language_id": 1,
    "stdin": ""
  }'
```

## 📁 Project Structure

```
rustbox-api/
├── app/
│   ├── __init__.py
│   ├── main.py              # FastAPI application
│   ├── worker.py            # Worker process
│   ├── config.py            # Configuration settings
│   ├── models/              # Database models
│   │   ├── __init__.py
│   │   ├── database.py      # SQLAlchemy models
│   │   └── schemas.py       # Pydantic schemas
│   ├── routes/              # API routes
│   │   ├── __init__.py
│   │   ├── submissions.py   # Submission endpoints
│   │   ├── languages.py     # Language endpoints
│   │   └── system.py        # System endpoints
│   └── services/            # Business logic
│       ├── __init__.py
│       ├── database.py      # Database service
│       ├── rustbox_service.py # Rustbox integration
│       └── queue_service.py # Redis queue service
├── docker-compose.yml       # Service orchestration
├── Dockerfile              # Application container
├── deploy.sh               # Deployment script
├── init_db.py              # Database initialization
├── requirements.txt        # Python dependencies
├── env.example             # Environment variables example
└── README.md              # This file
```

## 🔧 Configuration

The system is configured through environment variables. Key settings include:

### Database

-   `DATABASE_URL`: PostgreSQL connection string
-   `REDIS_URL`: Redis connection string

### Rustbox

-   `RUSTBOX_BINARY_PATH`: Path to rustbox binary
-   `RUSTBOX_WORK_DIR`: Working directory for rustbox

### Resource Limits

-   `DEFAULT_MEMORY_LIMIT_MB`: Default memory limit (512 MB)
-   `DEFAULT_TIME_LIMIT_SECONDS`: Default time limit (10 seconds)
-   `MAX_MEMORY_LIMIT_MB`: Maximum allowed memory (2048 MB)
-   `MAX_TIME_LIMIT_SECONDS`: Maximum allowed time (60 seconds)

### Worker Settings

-   `WORKER_CONCURRENCY`: Number of concurrent workers (4)
-   `WORKER_PREFETCH_MULTIPLIER`: Queue prefetch multiplier (1)

## 📚 API Endpoints

### Submissions

-   `POST /submissions` - Create a new submission
-   `POST /submissions/batch` - Create multiple submissions
-   `GET /submissions` - List submissions with filtering
-   `GET /submissions/{id}` - Get specific submission
-   `PUT /submissions/{id}` - Update submission
-   `DELETE /submissions/{id}` - Delete submission

### Languages

-   `GET /languages` - List supported languages
-   `GET /languages/{id}` - Get specific language

### System

-   `GET /system/health` - Health check
-   `GET /system/info` - System information
-   `GET /system/stats` - Detailed statistics

## 🔒 Supported Languages

| ID  | Language | Version | Extension |
| --- | -------- | ------- | --------- |
| 1   | Python   | 3.8.1   | .py       |
| 2   | C++      | 9.2.0   | .cpp      |
| 3   | Java     | 13.0.1  | .java     |

## 🚀 Deployment Options

### Development

For development, you can run the system without Docker:

```bash
# Install dependencies
pip install -r requirements.txt

# Set up database
python init_db.py

# Start the API server
python -m app.main

# Start worker in another terminal
python -m app.worker
```

### Production

For production deployment:

```bash
# Deploy with Docker Compose
./deploy.sh deploy

# Or build and deploy
./deploy.sh build
```

### Scaling

To scale the system horizontally:

```bash
# Scale workers
docker-compose up -d --scale worker=4

# Or modify docker-compose.yml
# deploy:
#   replicas: 4
```

## 📊 Monitoring

### Health Checks

```bash
# Check system health
curl http://localhost:8000/system/health

# Get system information
curl http://localhost:8000/system/info

# Get detailed statistics
curl http://localhost:8000/system/stats
```

### Logs

```bash
# View all logs
./deploy.sh logs

# View specific service logs
docker-compose logs api
docker-compose logs worker
```

## 🛠️ Management Commands

```bash
# Start the system
./deploy.sh deploy

# Stop the system
./deploy.sh stop

# Restart the system
./deploy.sh restart

# Check status
./deploy.sh status

# Test API
./deploy.sh test

# Clean up everything
./deploy.sh cleanup
```

## 🔧 Troubleshooting

### Common Issues

1. **Rustbox binary not found**

    ```bash
    cd ../rustbox-core
    cargo build --release
    cd ../rustbox-api
    ./deploy.sh build
    ```

2. **Database connection failed**

    ```bash
    # Check if PostgreSQL is running
    docker-compose ps postgres

    # Check logs
    docker-compose logs postgres
    ```

3. **Worker not processing jobs**

    ```bash
    # Check worker logs
    docker-compose logs worker

    # Check Redis connection
    docker-compose exec redis redis-cli ping
    ```

### Debug Mode

Enable debug mode for more detailed logging:

```bash
export DEBUG=true
export LOG_LEVEL=DEBUG
./deploy.sh restart
```

## 🔒 Security Considerations

-   **Sandboxing**: All code execution is isolated using rustbox
-   **Resource Limits**: Strict limits on memory, CPU, and execution time
-   **Network Isolation**: Network access is disabled by default
-   **File System**: Restricted access to file system
-   **Process Isolation**: Namespace isolation for security

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

-   Inspired by [codejudge](https://github.com/codejudge/codejudge)
-   Built with [rustbox](https://github.com/your-org/rustbox) for secure sandboxing
-   Powered by FastAPI, PostgreSQL, and Redis
