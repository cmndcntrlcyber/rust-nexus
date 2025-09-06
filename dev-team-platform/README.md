# Dev Team Platform

**AI-Powered Multi-Agent Development Team - Microservices Architecture**

This is a refactored version of the VS Code Dev Team Coordinator extension, transformed into a standalone microservices platform that can be used by standard development teams without requiring VS Code.

## Architecture Overview

The platform consists of:

- **Frontend**: React dashboard for project and task management
- **API Gateway**: Authentication, routing, and rate limiting
- **Core Services**: Orchestrator, Project Management, Task Management
- **Agent Services**: 6 specialized AI agents as individual microservices
- **Infrastructure**: PostgreSQL, Redis, NATS for messaging

## Services

### Core Services
- `orchestrator-service` (Port 3001) - Central coordination and agent management
- `project-service` (Port 3002) - Project CRUD and file management
- `task-service` (Port 3003) - Task assignment and tracking
- `api-gateway` (Port 3000) - Main entry point

### Agent Services
- `agent-architecture` (Port 3010) - Architecture decisions and coordination
- `agent-frontend` (Port 3011) - Frontend development
- `agent-backend` (Port 3012) - Backend and API development
- `agent-qa` (Port 3013) - Quality assurance and testing
- `agent-devops` (Port 3014) - CI/CD and deployment
- `agent-mcp` (Port 3015) - MCP server development

### Infrastructure
- PostgreSQL (Port 5432) - Primary database
- Redis (Port 6379) - Caching and sessions
- NATS (Port 4222) - Inter-service messaging

## Quick Start

```bash
# Start all services
docker-compose up -d

# Access the dashboard
open http://localhost:3000
```

## Migration from VS Code Extension

This platform extracts all the intelligent coordination and agent capabilities from the original VS Code extension while providing:

1. **Web-based Interface**: Access from any browser
2. **Team Collaboration**: Multiple users and projects
3. **Scalable Architecture**: Each agent runs as a separate service
4. **API-first Design**: Can be integrated with other tools
5. **Production Ready**: Monitoring, logging, and deployment configurations

## Documentation

See the `docs/` directory for detailed setup and usage instructions.
