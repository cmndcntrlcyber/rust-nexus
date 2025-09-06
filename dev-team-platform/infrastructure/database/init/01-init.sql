-- Create database extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'DEVELOPER',
    avatar TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE,
    preferences JSONB DEFAULT '{}',
    permissions JSONB DEFAULT '{}'
);

-- Create projects table
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    template_id VARCHAR(100),
    status VARCHAR(50) NOT NULL DEFAULT 'PLANNING',
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_members TEXT DEFAULT '[]',
    tags TEXT DEFAULT '[]',
    metadata JSONB DEFAULT '{}',
    configuration JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    due_date TIMESTAMP WITH TIME ZONE
);

-- Create tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(500) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL,
    priority VARCHAR(20) NOT NULL DEFAULT 'MEDIUM',
    status VARCHAR(50) NOT NULL DEFAULT 'NOT_STARTED',
    assigned_to VARCHAR(100),
    project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    due_date TIMESTAMP WITH TIME ZONE,
    dependencies TEXT DEFAULT '[]',
    blockers TEXT DEFAULT '[]',
    estimated_hours INTEGER DEFAULT 0,
    actual_hours INTEGER,
    tags TEXT DEFAULT '[]',
    metadata JSONB DEFAULT '{}'
);

-- Create agents table
CREATE TABLE IF NOT EXISTS agents (
    id VARCHAR(100) PRIMARY KEY,
    type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'OFFLINE',
    capabilities JSONB DEFAULT '{}',
    configuration JSONB DEFAULT '{}',
    metrics JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_heartbeat TIMESTAMP WITH TIME ZONE,
    service_url VARCHAR(500),
    version VARCHAR(20)
);

-- Create messages table
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    type VARCHAR(50) NOT NULL,
    sender VARCHAR(100) NOT NULL,
    recipient VARCHAR(100),
    payload JSONB DEFAULT '{}',
    priority VARCHAR(20) NOT NULL DEFAULT 'MEDIUM',
    requires_response BOOLEAN DEFAULT FALSE,
    correlation_id VARCHAR(100),
    ttl INTEGER,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE
);

-- Create task artifacts table
CREATE TABLE IF NOT EXISTS task_artifacts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    name VARCHAR(500) NOT NULL,
    path TEXT NOT NULL,
    content TEXT,
    size INTEGER DEFAULT 0,
    mime_type VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create project templates table
CREATE TABLE IF NOT EXISTS project_templates (
    id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(100),
    technologies TEXT DEFAULT '[]',
    agents JSONB DEFAULT '{}',
    phases JSONB DEFAULT '[]',
    estimated_duration INTEGER DEFAULT 0,
    complexity VARCHAR(20) DEFAULT 'MEDIUM',
    version VARCHAR(20) DEFAULT '1.0.0',
    author VARCHAR(255),
    icon TEXT,
    tags TEXT DEFAULT '[]',
    requirements JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create task progress table
CREATE TABLE IF NOT EXISTS task_progress (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    percentage INTEGER DEFAULT 0,
    current_step VARCHAR(500),
    total_steps INTEGER DEFAULT 1,
    completed_steps INTEGER DEFAULT 0,
    time_spent INTEGER DEFAULT 0,
    estimated_remaining INTEGER DEFAULT 0,
    details JSONB DEFAULT '[]',
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(task_id)
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_projects_owner ON projects(owner_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
CREATE INDEX IF NOT EXISTS idx_projects_created ON projects(created_at);

CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned ON tasks(assigned_to);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_type ON tasks(type);
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at);

CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);
CREATE INDEX IF NOT EXISTS idx_messages_recipient ON messages(recipient);
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(type);
CREATE INDEX IF NOT EXISTS idx_messages_processed ON messages(processed);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);

CREATE INDEX IF NOT EXISTS idx_agents_type ON agents(type);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_agents_heartbeat ON agents(last_heartbeat);

CREATE INDEX IF NOT EXISTS idx_task_artifacts_task ON task_artifacts(task_id);
CREATE INDEX IF NOT EXISTS idx_task_artifacts_type ON task_artifacts(type);

-- Create updated_at triggers
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

CREATE TRIGGER update_tasks_updated_at BEFORE UPDATE ON tasks
    FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

CREATE TRIGGER update_agents_updated_at BEFORE UPDATE ON agents
    FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

CREATE TRIGGER update_templates_updated_at BEFORE UPDATE ON project_templates
    FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

-- Insert default admin user (password: admin123)
INSERT INTO users (email, name, password_hash, role) VALUES 
('admin@devteam.local', 'Admin User', crypt('admin123', gen_salt('bf')), 'ADMIN')
ON CONFLICT (email) DO NOTHING;

-- Insert default project templates
INSERT INTO project_templates (
    id, name, description, category, technologies, agents, phases, 
    estimated_duration, complexity, author
) VALUES 
(
    'react-app',
    'React Application',
    'Full-stack React application with TypeScript',
    'Web Application',
    '["React", "TypeScript", "Node.js", "Express"]',
    '{
        "FRONTEND_CORE": ["component-development", "routing", "state-management"],
        "BACKEND_INTEGRATION": ["api-development", "database-design"],
        "QUALITY_ASSURANCE": ["testing", "quality-validation"],
        "DEVOPS": ["build-setup", "deployment"],
        "ARCHITECTURE_LEAD": ["coordination", "decisions"]
    }',
    '[
        {
            "id": "setup",
            "name": "Setup",
            "description": "Project initialization and setup",
            "duration": "1-2 days",
            "tasks": ["project-initialization", "dependency-setup"],
            "dependencies": [],
            "deliverables": ["Project structure", "Dependencies installed"],
            "milestones": []
        },
        {
            "id": "development",
            "name": "Development",
            "description": "Core development phase",
            "duration": "1-2 weeks",
            "tasks": ["component-development", "api-integration"],
            "dependencies": ["setup"],
            "deliverables": ["Working application", "API integration"],
            "milestones": []
        }
    ]',
    14,
    'MEDIUM',
    'Dev Team Platform'
),
(
    'mcp-server',
    'MCP Server',
    'Model Context Protocol server with custom tools',
    'Backend Service',
    '["Node.js", "TypeScript", "gRPC"]',
    '{
        "MCP_INTEGRATION": ["server-scaffold", "tool-creation", "documentation"],
        "BACKEND_INTEGRATION": ["server-setup", "api-endpoints"],
        "QUALITY_ASSURANCE": ["testing", "validation"],
        "DEVOPS": ["deployment", "monitoring"],
        "ARCHITECTURE_LEAD": ["coordination", "architecture"]
    }',
    '[
        {
            "id": "server-setup",
            "name": "Server Setup",
            "description": "Initialize MCP server structure",
            "duration": "1 day",
            "tasks": ["server-scaffold", "basic-configuration"],
            "dependencies": [],
            "deliverables": ["Server skeleton", "Basic configuration"],
            "milestones": []
        },
        {
            "id": "tool-development",
            "name": "Tool Development",
            "description": "Develop custom MCP tools",
            "duration": "3-5 days",
            "tasks": ["custom-tools", "resource-management"],
            "dependencies": ["server-setup"],
            "deliverables": ["Custom tools", "Resource handlers"],
            "milestones": []
        }
    ]',
    7,
    'LOW',
    'Dev Team Platform'
)
ON CONFLICT (id) DO NOTHING;
