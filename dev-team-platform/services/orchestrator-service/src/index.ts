import Fastify from 'fastify';
import cors from '@fastify/cors';
import helmet from '@fastify/helmet';
import rateLimit from '@fastify/rate-limit';
import jwt from '@fastify/jwt';
import websocket from '@fastify/websocket';
import { 
    AgentId, 
    AgentTask, 
    AgentMessage, 
    PlatformConfig, 
    Logger, 
    ProjectTemplate,
    TaskType,
    PlatformError,
    BaseAgent,
    Project
} from '@dev-team-platform/types';
import { AgentOrchestrator } from './orchestrator/AgentOrchestrator';
import { DatabaseManager } from './database/DatabaseManager';
import { MessageBroker } from './messaging/MessageBroker';
import { createLogger } from './utils/logger';
import { loadConfig } from './utils/config';
import { registerRoutes } from './routes';
import { v4 as uuid } from 'uuid';

async function createServer() {
    // Load configuration
    const config = await loadConfig();
    const logger = createLogger(config.logLevel);

    // Create Fastify instance
    const fastify = Fastify({
        logger: {
            level: config.logLevel,
            transport: {
                target: 'pino-pretty',
                options: {
                    colorize: true,
                    translateTime: 'HH:MM:ss Z',
                    ignore: 'pid,hostname',
                },
            },
        },
    });

    // Register plugins
    await fastify.register(helmet, {
        contentSecurityPolicy: false, // Allow WebSocket connections
    });

    await fastify.register(cors, {
        origin: config.corsOrigins,
        credentials: true,
    });

    await fastify.register(rateLimit, {
        max: config.rateLimit.maxRequests,
        timeWindow: config.rateLimit.windowMs,
        skipOnError: true,
    });

    await fastify.register(jwt, {
        secret: config.jwtSecret,
    });

    await fastify.register(websocket, {
        options: { maxPayload: 1048576 }, // 1MB max payload
    });

    // Initialize core services
    const database = new DatabaseManager(config.databaseUrl, logger);
    const messageBroker = new MessageBroker(config.natsUrl, logger);
    const orchestrator = new AgentOrchestrator(config, logger, database, messageBroker);

    // Initialize services
    await database.initialize();
    await messageBroker.initialize();
    await orchestrator.initialize();

    // Register request context decorator
    fastify.decorateRequest('requestId', '');
    fastify.decorateRequest('user', null);

    // Add request ID to all requests
    fastify.addHook('onRequest', async (request) => {
        request.requestId = uuid();
    });

    // JWT Authentication hook
    fastify.addHook('onRequest', async (request, reply) => {
        // Skip auth for health checks and public endpoints
        if (request.url.startsWith('/health') || request.url.startsWith('/metrics')) {
            return;
        }

        try {
            const token = await request.jwtVerify();
            request.user = token;
        } catch (err) {
            reply.code(401).send({ 
                success: false, 
                error: { code: 'UNAUTHORIZED', message: 'Invalid or missing token' } 
            });
        }
    });

    // Error handler
    fastify.setErrorHandler(async (error, request, reply) => {
        logger.error('Request error', error);

        if (error instanceof PlatformError) {
            return reply.code(error.statusCode).send({
                success: false,
                error: {
                    code: error.code,
                    message: error.message,
                    details: error.context,
                },
                meta: {
                    requestId: request.requestId,
                    timestamp: new Date(),
                },
            });
        }

        // Handle Fastify validation errors
        if (error.validation) {
            return reply.code(400).send({
                success: false,
                error: {
                    code: 'VALIDATION_ERROR',
                    message: 'Invalid request data',
                    details: error.validation,
                },
                meta: {
                    requestId: request.requestId,
                    timestamp: new Date(),
                },
            });
        }

        // Generic error response
        reply.code(500).send({
            success: false,
            error: {
                code: 'INTERNAL_ERROR',
                message: 'Internal server error',
            },
            meta: {
                requestId: request.requestId,
                timestamp: new Date(),
            },
        });
    });

    // Register routes
    await registerRoutes(fastify, orchestrator, database, messageBroker, logger);

    // Health check endpoints
    fastify.get('/health', async () => {
        const dbHealth = await database.healthCheck();
        const brokerHealth = await messageBroker.healthCheck();
        const orchestratorHealth = await orchestrator.getHealthStatus();

        const isHealthy = dbHealth.status === 'HEALTHY' && 
                         brokerHealth.status === 'HEALTHY' && 
                         orchestratorHealth.status === 'HEALTHY';

        return {
            status: isHealthy ? 'HEALTHY' : 'UNHEALTHY',
            timestamp: new Date(),
            services: {
                database: dbHealth,
                messageBroker: brokerHealth,
                orchestrator: orchestratorHealth,
            },
        };
    });

    fastify.get('/metrics', async () => {
        const agentMetrics = await orchestrator.getAllAgentMetrics();
        const systemMetrics = await orchestrator.getSystemMetrics();

        return {
            agents: agentMetrics,
            system: systemMetrics,
            timestamp: new Date(),
        };
    });

    // WebSocket for real-time updates
    fastify.register(async function (fastify) {
        fastify.get('/ws', { websocket: true }, (connection) => {
            logger.info('WebSocket connection established');

            // Subscribe to orchestrator events
            const unsubscribe = orchestrator.subscribe('*', (event) => {
                connection.socket.send(JSON.stringify({
                    type: 'ORCHESTRATOR_EVENT',
                    payload: event,
                    timestamp: new Date(),
                }));
            });

            connection.socket.on('message', (message) => {
                try {
                    const data = JSON.parse(message.toString());
                    logger.debug('Received WebSocket message', data);
                    
                    // Handle WebSocket commands here
                    switch (data.type) {
                        case 'SUBSCRIBE_TO_PROJECT':
                            // Subscribe to project-specific events
                            break;
                        case 'SUBSCRIBE_TO_AGENT':
                            // Subscribe to agent-specific events
                            break;
                    }
                } catch (error) {
                    logger.error('Error processing WebSocket message', error as Error);
                }
            });

            connection.socket.on('close', () => {
                logger.info('WebSocket connection closed');
                unsubscribe();
            });
        });
    });

    // Graceful shutdown
    const gracefulShutdown = async (signal: string) => {
        logger.info(`Received ${signal}, starting graceful shutdown...`);

        try {
            await orchestrator.stop();
            await messageBroker.close();
            await database.close();
            await fastify.close();
            
            logger.info('Graceful shutdown completed');
            process.exit(0);
        } catch (error) {
            logger.error('Error during shutdown', error as Error);
            process.exit(1);
        }
    };

    process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));
    process.on('SIGINT', () => gracefulShutdown('SIGINT'));

    return fastify;
}

async function start() {
    try {
        const fastify = await createServer();
        const port = parseInt(process.env.PORT || '3001', 10);
        const host = process.env.HOST || '0.0.0.0';

        await fastify.listen({ port, host });
        console.log(`🚀 Orchestrator Service running on http://${host}:${port}`);
    } catch (error) {
        console.error('Failed to start server:', error);
        process.exit(1);
    }
}

// Start the server if this file is run directly
if (require.main === module) {
    start();
}

export { createServer, start };
