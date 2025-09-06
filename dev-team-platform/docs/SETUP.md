# Dev Team Platform Setup Guide

This guide will help you set up and run the Dev Team Platform, a microservices-based AI development team system refactored from the original VS Code extension.

## Prerequisites

- **Docker & Docker Compose** (v2.0+)
- **Node.js** (v18.0+) 
- **npm** (v9.0+)
- **Git**

## Quick Start

### 1. Clone and Setup

```bash
# Clone the repository (if not already done)
git clone <repository-url>
cd dev-team-platform

# Copy environment file
cp .env.example .env

# Edit .env with your API keys
nano .env  # Add your ANTHROPIC_API_KEY and TAVILY_API_KEY
```

### 2. Configure API Keys

Edit the `.env` file and add your API keys:

```env
ANTHROPIC_API_KEY=your_actual_anthropic_key
TAVILY_API_KEY=your_actual_tavily_key
JWT_SECRET=your_secure_jwt_secret_here
```

### 3. Start the Platform

```bash
# Start all services with Docker Compose
npm run dev:build

# Or alternatively:
docker-compose up --build -d
```

### 4. Verify Installation

```bash
# Check service health
curl http://localhost:3001/health  # Orchestrator Service
curl http://localhost:3000/health  # API Gateway
curl http://localhost:3080         # Frontend Dashboard

# View logs
npm run dev:logs
```

## Architecture Overview

The platform consists of the following services:

### Core Services
- **API Gateway** (`:3000`) - Main entry point, authentication, routing
- **Orchestrator Service** (`:3001`) - Central agent coordination
- **Project Service** (`:3002`) - Project management
- **Task Service** (`:3003`) - Task assignment and tracking

### Agent Services
- **Architecture Agent** (`:3010`) - Project coordination and decisions
- **Frontend Agent** (`:3011`) - React/Vue/Angular development  
- **Backend Agent** (`:3012`) - API and database development
- **QA Agent** (`:3013`) - Testing and quality assurance
- **DevOps Agent** (`:3014`) - CI/CD and deployment
- **MCP Agent** (`:3015`) - Model Context Protocol integration

### Infrastructure
- **PostgreSQL** (`:5432`) - Primary database
- **Redis** (`:6379`) - Caching and sessions
- **NATS** (`:4222`) - Inter-service messaging
- **Frontend** (`:3080`) - React dashboard

## Development Setup

### Install Dependencies

```bash
# Install root dependencies
npm install

# Install all workspace dependencies
npm run setup
```

### Build Shared Packages

```bash
# Build shared types package
npm run build:shared
```

### Run Services Locally

```bash
# Start infrastructure services
docker-compose up -d postgres redis nats

# Start orchestrator service in development mode
cd services/orchestrator-service
npm run dev

# Start other services similarly...
```

### Database Setup

```bash
# Run database migrations
npm run db:migrate

# Seed with sample data
npm run db:seed
```

## Usage Guide

### 1. Access the Dashboard

Open your browser to `http://localhost:3080`

Default credentials:
- Email: `admin@devteam.local`
- Password: `admin123`

### 2. Create a New Project

1. Click "New Project" in the dashboard
2. Select a template (React App or MCP Server)
3. Enter project details
4. Watch as AI agents automatically:
   - Set up the project structure
   - Install dependencies
   - Generate code components
   - Create tests
   - Set up deployment

### 3. Monitor Progress

- **Dashboard**: Real-time project and task progress
- **Agent Status**: Monitor all 6 AI agents
- **Task Kanban**: Visual task management
- **Analytics**: Performance metrics and insights

## API Reference

### Authentication

```bash
# Login to get JWT token
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@devteam.local", "password": "admin123"}'

# Use token in subsequent requests
curl -H "Authorization: Bearer <jwt-token>" \
  http://localhost:3000/api/v1/projects
```

### Projects API

```bash
# List projects
GET /api/v1/projects

# Create project
POST /api/v1/projects
{
  "name": "My Project",
  "templateId": "react-app",
  "description": "A new React application"
}

# Get project details
GET /api/v1/projects/:id
```

### Tasks API

```bash
# List tasks
GET /api/v1/tasks

# Assign task to agent
POST /api/v1/tasks/:id/assign
{
  "agentId": "frontend-core-001"
}
```

### Agents API

```bash
# List all agents
GET /api/v1/agents

# Get agent status
GET /api/v1/agents/:id/status

# Send command to agent
POST /api/v1/agents/:id/command
{
  "type": "TASK_ASSIGNMENT",
  "payload": { ... }
}
```

## WebSocket Events

Connect to `ws://localhost:3000/ws` for real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.on('message', (data) => {
  const event = JSON.parse(data);
  console.log('Received event:', event.type, event.payload);
});

// Subscribe to project events
ws.send(JSON.stringify({
  type: 'SUBSCRIBE_TO_PROJECT',
  projectId: 'project-id-here'
}));
```

## Monitoring & Debugging

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f orchestrator-service

# Live log streaming
npm run dev:logs
```

### Health Checks

```bash
# Overall system health
curl http://localhost:3001/health

# Individual service health
curl http://localhost:3002/health  # Project Service
curl http://localhost:3003/health  # Task Service
```

### Metrics

```bash
# System metrics
curl http://localhost:3001/metrics

# Agent performance metrics
curl http://localhost:3001/api/v1/agents/metrics
```

## Configuration

### Environment Variables

Key configuration options in `.env`:

```env
# Database
DATABASE_URL=postgresql://user:pass@host:port/db

# Message Queue
NATS_URL=nats://localhost:4222

# Security
JWT_SECRET=your-secret-key

# Agent Configuration
ANTHROPIC_API_KEY=your-key
MAX_CONCURRENT_TASKS=3
AGENT_TIMEOUT=30000

# Logging
LOG_LEVEL=info  # debug, info, warn, error
```

### Service Configuration

Each service can be configured via environment variables:

- **Orchestrator Service**: Agent coordination settings
- **Project Service**: File system and Git integration
- **Task Service**: Task scheduling and dependencies
- **Agent Services**: AI model settings and capabilities

## Troubleshooting

### Common Issues

**Service won't start:**
```bash
# Check Docker is running
docker info

# Rebuild services
docker-compose down
docker-compose up --build
```

**Database connection errors:**
```bash
# Check PostgreSQL is running
docker-compose ps postgres

# Reset database
docker-compose down -v
docker-compose up postgres -d
```

**Agent not responding:**
```bash
# Check agent service logs
docker-compose logs agent-architecture

# Restart specific agent
docker-compose restart agent-architecture
```

**Authentication issues:**
```bash
# Verify JWT secret is set
echo $JWT_SECRET

# Check user exists in database
docker-compose exec postgres psql -U devteam -d dev_team_platform -c "SELECT * FROM users;"
```

### Performance Optimization

**High Memory Usage:**
- Reduce `MAX_CONCURRENT_TASKS`
- Increase `AGENT_TIMEOUT` for better resource management
- Monitor with `docker stats`

**Slow Response Times:**
- Enable Redis caching
- Optimize database queries
- Scale agent services horizontally

## Migration from VS Code Extension

This platform provides all the functionality of the original VS Code extension:

### Key Differences

| VS Code Extension | Microservices Platform |
|-------------------|------------------------|
| VS Code UI | Web Dashboard |
| Extension storage | PostgreSQL Database |
| In-process agents | Microservice agents |
| Local workspace | Project management |
| VS Code commands | REST API + WebSocket |

### Migration Benefits

1. **Multi-user Support**: Team collaboration
2. **Web Access**: Use from any browser
3. **Scalability**: Horizontal scaling of agents
4. **API Integration**: Integrate with other tools
5. **Production Ready**: Monitoring and deployment

## Advanced Configuration

### Custom Agent Development

Add new specialized agents:

1. Create new agent service directory
2. Implement `BaseAgent` interface
3. Add to docker-compose.yml
4. Register with orchestrator

### Horizontal Scaling

Scale agents for higher throughput:

```bash
# Scale frontend agents
docker-compose up --scale agent-frontend=3

# Configure load balancing in orchestrator
```

### Custom Project Templates

Add new project templates via API:

```bash
POST /api/v1/templates
{
  "id": "custom-template",
  "name": "Custom Project",
  "technologies": ["Next.js", "Prisma"],
  "phases": [...]
}
```

## Support

- **Documentation**: `/docs` directory
- **API Reference**: `http://localhost:3000/api-docs`
- **Health Dashboard**: `http://localhost:3001/health`
- **Monitoring**: `http://localhost:9090` (Prometheus)

For issues and questions, check the troubleshooting section above or review service logs.
