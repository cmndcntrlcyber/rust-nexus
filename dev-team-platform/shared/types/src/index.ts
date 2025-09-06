// Core Types for Dev Team Platform (Refactored from VS Code Extension)

// Agent Types
export type AgentId = string;
export type AgentType = 
  | 'ARCHITECTURE_LEAD'
  | 'FRONTEND_CORE'
  | 'FRONTEND_UIUX'
  | 'FRONTEND_VISUALIZATION'
  | 'BACKEND_INTEGRATION'
  | 'QUALITY_ASSURANCE'
  | 'DEVOPS'
  | 'MCP_INTEGRATION';

export type AgentStatus = 
  | 'INITIALIZING'
  | 'READY'
  | 'BUSY'
  | 'BLOCKED'
  | 'ERROR'
  | 'OFFLINE';

export interface AgentCapabilities {
  supportedTaskTypes: TaskType[];
  requiredAPIs: string[];
  skillLevel: 'junior' | 'mid' | 'senior' | 'expert';
  maxConcurrentTasks: number;
  estimatedTaskDuration: Record<TaskType, number>;
}

export interface AgentConfig {
  anthropicApiKey: string;
  tavilyApiKey?: string;
  maxRetries: number;
  timeout: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
  workingDirectory: string;
  serviceUrl?: string;
  natsUrl?: string;
}

export interface BaseAgent {
  readonly id: AgentId;
  readonly type: AgentType;
  readonly capabilities: AgentCapabilities;
  readonly status: AgentStatus;
  
  // Core lifecycle methods
  initialize(config: AgentConfig): Promise<void>;
  start(): Promise<void>;
  stop(): Promise<void>;
  restart(): Promise<void>;
  
  // Task execution
  executeTask(task: AgentTask): Promise<TaskResult>;
  canHandleTask(task: AgentTask): boolean;
  getTaskProgress(taskId: string): TaskProgress | null;
  
  // Communication
  sendMessage(message: AgentMessage): Promise<void>;
  receiveMessage(message: AgentMessage): Promise<AgentResponse>;
  subscribeToTopic(topic: string): void;
  
  // Health and monitoring
  getHealthStatus(): HealthStatus;
  getMetrics(): AgentMetrics;
  getConfiguration(): AgentConfig;
}

// Task Types
export type TaskType = 
  | 'FOUNDATION'
  | 'AGENT_DEVELOPMENT'
  | 'INTEGRATION'
  | 'UI_DEVELOPMENT'
  | 'TESTING'
  | 'DOCUMENTATION'
  | 'DEPLOYMENT'
  | 'CODE_GENERATION'
  | 'CODE_REVIEW'
  | 'BUG_FIX'
  | 'REFACTORING';

export type TaskStatus = 
  | 'NOT_STARTED'
  | 'IN_PROGRESS'
  | 'BLOCKED'
  | 'REVIEW'
  | 'TESTING'
  | 'COMPLETED'
  | 'DEFERRED'
  | 'CANCELLED';

export type TaskPriority = 
  | 'CRITICAL'
  | 'HIGH'
  | 'MEDIUM'
  | 'LOW';

export interface AgentTask {
  id: string;
  title: string;
  description: string;
  type: TaskType;
  priority: TaskPriority;
  status: TaskStatus;
  assignedTo?: AgentId;
  projectId?: string;
  createdAt: Date;
  updatedAt: Date;
  startedAt?: Date;
  completedAt?: Date;
  dueDate?: Date;
  dependencies: string[];
  blockers: string[];
  estimatedHours: number;
  actualHours?: number;
  tags: string[];
  metadata: Record<string, any>;
}

export interface TaskResult {
  taskId: string;
  status: 'SUCCESS' | 'FAILURE' | 'PARTIAL';
  output: any;
  artifacts: TaskArtifact[];
  duration: number;
  errors?: string[];
  warnings?: string[];
  nextSteps?: string[];
  metrics?: TaskMetrics;
}

export interface TaskArtifact {
  id: string;
  type: 'FILE' | 'CODE' | 'DOCUMENTATION' | 'TEST' | 'CONFIG';
  name: string;
  path: string;
  content?: string;
  size: number;
  mimeType: string;
  createdAt: Date;
}

export interface TaskProgress {
  taskId: string;
  percentage: number;
  currentStep: string;
  totalSteps: number;
  completedSteps: number;
  timeSpent: number;
  estimatedRemaining: number;
  lastUpdate: Date;
  details?: string[];
}

export interface TaskMetrics {
  complexity: number;
  quality: number;
  testCoverage?: number;
  codeLines: number;
  filesChanged: number;
}

// Communication Types
export type MessageType = 
  | 'TASK_ASSIGNMENT'
  | 'TASK_COMPLETION'
  | 'TASK_UPDATE'
  | 'STATUS_UPDATE'
  | 'COORDINATION_REQUEST'
  | 'DEPENDENCY_NOTIFICATION'
  | 'QUALITY_GATE_RESULT'
  | 'HUMAN_INPUT_REQUIRED'
  | 'ERROR_REPORT'
  | 'KNOWLEDGE_SHARING'
  | 'AGENT_REGISTRATION'
  | 'HEALTH_CHECK'
  | 'WORKFLOW_EVENT';

export type MessagePriority = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

export interface AgentMessage {
  id: string;
  type: MessageType;
  sender: AgentId | 'orchestrator' | 'user';
  recipient?: AgentId; // undefined for broadcast
  timestamp: Date;
  payload: MessagePayload;
  priority: MessagePriority;
  requiresResponse: boolean;
  correlationId?: string;
  ttl?: number; // time to live in seconds
}

export interface AgentResponse {
  messageId: string;
  success: boolean;
  data?: any;
  error?: string;
  timestamp: Date;
  processingTime: number;
}

export type MessagePayload = Record<string, any>;

// Health and Monitoring Types
export interface HealthStatus {
  status: 'HEALTHY' | 'DEGRADED' | 'UNHEALTHY';
  lastCheck: Date;
  uptime: number;
  issues: HealthIssue[];
  systemInfo: SystemInfo;
}

export interface HealthIssue {
  severity: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  message: string;
  code: string;
  timestamp: Date;
}

export interface SystemInfo {
  memoryUsage: number;
  cpuUsage: number;
  diskUsage?: number;
  networkLatency?: number;
  activeConnections: number;
}

export interface AgentMetrics {
  productivity: ProductivityMetrics;
  quality: QualityMetrics;
  reliability: ReliabilityMetrics;
  coordination: CoordinationMetrics;
  performance: PerformanceMetrics;
}

export interface ProductivityMetrics {
  tasksCompleted: number;
  tasksInProgress: number;
  averageCompletionTime: number;
  velocityTrend: number[];
  estimationAccuracy: number;
  throughput: number; // tasks per hour
}

export interface QualityMetrics {
  codeQualityScore: number;
  testCoverageAchieved: number;
  defectRate: number;
  reworkPercentage: number;
  reviewScore: number;
  complianceScore: number;
}

export interface ReliabilityMetrics {
  uptime: number;
  errorRate: number;
  responseTime: number;
  successRate: number;
  mtbf: number; // mean time between failures
  mttr: number; // mean time to recovery
}

export interface CoordinationMetrics {
  communicationEfficiency: number;
  conflictResolutionTime: number;
  dependencyResolutionRate: number;
  collaborationScore: number;
  knowledgeSharingRate: number;
}

export interface PerformanceMetrics {
  averageResponseTime: number;
  peakResponseTime: number;
  requestsPerSecond: number;
  concurrentTasks: number;
  resourceUtilization: number;
}

// Project Types
export interface Project {
  id: string;
  name: string;
  description: string;
  templateId?: string;
  status: ProjectStatus;
  createdAt: Date;
  updatedAt: Date;
  startedAt?: Date;
  completedAt?: Date;
  dueDate?: Date;
  ownerId: string;
  teamMembers: string[];
  tags: string[];
  metadata: ProjectMetadata;
  configuration: ProjectConfiguration;
}

export type ProjectStatus = 
  | 'PLANNING'
  | 'IN_PROGRESS'
  | 'ON_HOLD'
  | 'COMPLETED'
  | 'CANCELLED'
  | 'ARCHIVED';

export interface ProjectMetadata {
  repository?: string;
  framework: string;
  language: string;
  estimatedDuration: number;
  actualDuration?: number;
  complexity: 'LOW' | 'MEDIUM' | 'HIGH';
  budget?: number;
}

export interface ProjectConfiguration {
  workingDirectory: string;
  buildCommand?: string;
  testCommand?: string;
  deployCommand?: string;
  environmentVariables: Record<string, string>;
  dependencies: string[];
  devDependencies: string[];
}

export interface ProjectTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  technologies: string[];
  agents: Record<AgentType, string[]>;
  phases: ProjectPhase[];
  estimatedDuration: number;
  complexity: 'LOW' | 'MEDIUM' | 'HIGH';
  version: string;
  author: string;
  icon?: string;
  tags: string[];
  requirements: TemplateRequirements;
}

export interface TemplateRequirements {
  minAgents: number;
  requiredCapabilities: string[];
  optionalCapabilities: string[];
  systemRequirements: Record<string, string>;
}

export interface ProjectPhase {
  id: string;
  name: string;
  description: string;
  duration: string;
  tasks: string[];
  dependencies: string[];
  deliverables: string[];
  milestones: Milestone[];
}

export interface Milestone {
  id: string;
  name: string;
  description: string;
  dueDate: Date;
  status: 'PENDING' | 'IN_PROGRESS' | 'COMPLETED' | 'OVERDUE';
  criteria: string[];
}

// User and Authentication Types
export interface User {
  id: string;
  email: string;
  name: string;
  role: UserRole;
  avatar?: string;
  createdAt: Date;
  lastLoginAt: Date;
  preferences: UserPreferences;
  permissions: UserPermissions;
}

export type UserRole = 'ADMIN' | 'MANAGER' | 'DEVELOPER' | 'VIEWER';

export interface UserPreferences {
  theme: 'light' | 'dark' | 'auto';
  notifications: NotificationPreferences;
  dashboard: DashboardPreferences;
  language: string;
  timezone: string;
}

export interface NotificationPreferences {
  email: boolean;
  push: boolean;
  taskUpdates: boolean;
  projectUpdates: boolean;
  agentAlerts: boolean;
}

export interface DashboardPreferences {
  defaultView: 'projects' | 'tasks' | 'agents' | 'analytics';
  widgetLayout: string[];
  refreshInterval: number;
}

export interface UserPermissions {
  canCreateProjects: boolean;
  canManageAgents: boolean;
  canViewAnalytics: boolean;
  canManageUsers: boolean;
  projects: ProjectPermissions[];
}

export interface ProjectPermissions {
  projectId: string;
  canRead: boolean;
  canWrite: boolean;
  canManage: boolean;
  canDelete: boolean;
}

// Configuration Types
export interface PlatformConfig {
  anthropicApiKey: string;
  tavilyApiKey?: string;
  maxConcurrentTasks: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
  databaseUrl: string;
  redisUrl: string;
  natsUrl: string;
  jwtSecret: string;
  agentTimeout: number;
  enableTelemetry: boolean;
  enableMonitoring: boolean;
  corsOrigins: string[];
  rateLimit: RateLimitConfig;
}

export interface RateLimitConfig {
  windowMs: number;
  maxRequests: number;
  skipSuccessfulRequests: boolean;
}

// API Types
export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: ApiError;
  meta?: ResponseMetadata;
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, any>;
  stack?: string;
}

export interface ResponseMetadata {
  timestamp: Date;
  requestId: string;
  version: string;
  pagination?: PaginationInfo;
}

export interface PaginationInfo {
  page: number;
  limit: number;
  total: number;
  totalPages: number;
  hasNext: boolean;
  hasPrev: boolean;
}

export interface ApiRequest {
  path: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  headers: Record<string, string>;
  query: Record<string, string>;
  body?: any;
  user?: User;
}

// WebSocket Types
export interface WebSocketMessage {
  type: string;
  payload: any;
  timestamp: Date;
  id: string;
}

export interface WebSocketEvent {
  event: string;
  data: any;
  room?: string;
  userId?: string;
}

// Database Types
export interface DatabaseRecord {
  id: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface TaskRecord extends DatabaseRecord {
  title: string;
  description: string;
  type: string;
  priority: string;
  status: string;
  assignedTo: string | null;
  projectId: string | null;
  startedAt: Date | null;
  completedAt: Date | null;
  dueDate: Date | null;
  dependencies: string;
  blockers: string;
  estimatedHours: number;
  actualHours: number | null;
  tags: string;
  metadata: string;
}

export interface ProjectRecord extends DatabaseRecord {
  name: string;
  description: string;
  templateId: string | null;
  status: string;
  ownerId: string;
  teamMembers: string;
  tags: string;
  metadata: string;
  configuration: string;
  startedAt: Date | null;
  completedAt: Date | null;
  dueDate: Date | null;
}

// Error Types
export class PlatformError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode: number = 500,
    public context?: Record<string, any>
  ) {
    super(message);
    this.name = 'PlatformError';
  }
}

export class ValidationError extends PlatformError {
  constructor(message: string, context?: Record<string, any>) {
    super(message, 'VALIDATION_ERROR', 400, context);
    this.name = 'ValidationError';
  }
}

export class AuthenticationError extends PlatformError {
  constructor(message: string = 'Authentication failed', context?: Record<string, any>) {
    super(message, 'AUTHENTICATION_ERROR', 401, context);
    this.name = 'AuthenticationError';
  }
}

export class AuthorizationError extends PlatformError {
  constructor(message: string = 'Access denied', context?: Record<string, any>) {
    super(message, 'AUTHORIZATION_ERROR', 403, context);
    this.name = 'AuthorizationError';
  }
}

export class NotFoundError extends PlatformError {
  constructor(message: string, context?: Record<string, any>) {
    super(message, 'NOT_FOUND', 404, context);
    this.name = 'NotFoundError';
  }
}

// Utility Types
export interface Logger {
  debug(message: string, ...args: any[]): void;
  info(message: string, ...args: any[]): void;
  warn(message: string, ...args: any[]): void;
  error(message: string, error?: Error, ...args: any[]): void;
}

export interface CacheManager {
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T, ttl?: number): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
  has(key: string): Promise<boolean>;
}

export interface EventEmitter {
  on(event: string, listener: (...args: any[]) => void): void;
  off(event: string, listener: (...args: any[]) => void): void;
  emit(event: string, ...args: any[]): void;
  once(event: string, listener: (...args: any[]) => void): void;
}

// Export all types as a namespace for easier imports
export * as Types from './index';
