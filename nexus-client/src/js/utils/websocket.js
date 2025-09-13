/**
 * WebSocket Manager for Nexus C2 Client
 * Handles WebSocket connections and real-time communication
 */

export class WebSocketManager {
    constructor() {
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 1000;
        this.isConnected = false;
        this.config = null;
        this.heartbeatInterval = null;
        this.heartbeatTimeout = null;

        // Event listeners
        this.eventListeners = new Map();

        // Bind methods
        this.connect = this.connect.bind(this);
        this.disconnect = this.disconnect.bind(this);
        this.send = this.send.bind(this);
        this.handleOpen = this.handleOpen.bind(this);
        this.handleMessage = this.handleMessage.bind(this);
        this.handleError = this.handleError.bind(this);
        this.handleClose = this.handleClose.bind(this);
    }

    /**
     * Connect to WebSocket server
     * @param {Object} config - Connection configuration
     */
    async connect(config) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            console.log('WebSocket already connected');
            return;
        }

        this.config = config;
        const wsUrl = config.websocket_endpoint || `wss://${config.server_endpoint}:${config.server_port}/ws`;

        try {
            console.log('Connecting to WebSocket:', wsUrl);

            this.ws = new WebSocket(wsUrl);

            // Set up event handlers
            this.ws.addEventListener('open', this.handleOpen);
            this.ws.addEventListener('message', this.handleMessage);
            this.ws.addEventListener('error', this.handleError);
            this.ws.addEventListener('close', this.handleClose);

        } catch (error) {
            console.error('Failed to create WebSocket connection:', error);
            this.emit('websocket_error', error.message);
        }
    }

    /**
     * Disconnect from WebSocket server
     */
    disconnect() {
        if (this.ws) {
            this.ws.removeEventListener('open', this.handleOpen);
            this.ws.removeEventListener('message', this.handleMessage);
            this.ws.removeEventListener('error', this.handleError);
            this.ws.removeEventListener('close', this.handleClose);

            this.ws.close();
            this.ws = null;
        }

        this.clearHeartbeat();
        this.isConnected = false;
        this.reconnectAttempts = 0;

        this.emit('websocket_connected', false);
    }

    /**
     * Send message through WebSocket
     * @param {Object} message - Message to send
     */
    send(message) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            try {
                this.ws.send(JSON.stringify(message));
                return true;
            } catch (error) {
                console.error('Failed to send WebSocket message:', error);
                return false;
            }
        } else {
            console.warn('WebSocket is not connected');
            return false;
        }
    }

    /**
     * Handle WebSocket open event
     */
    handleOpen(event) {
        console.log('WebSocket connected');
        this.isConnected = true;
        this.reconnectAttempts = 0;

        // Start heartbeat
        this.startHeartbeat();

        // Emit connection event
        this.emit('websocket_connected', true);

        // Send authentication if configured
        if (this.config && this.config.username) {
            this.send({
                type: 'authenticate',
                username: this.config.username,
                team_name: this.config.team_name
            });
        }
    }

    /**
     * Handle WebSocket message event
     */
    handleMessage(event) {
        try {
            const message = JSON.parse(event.data);
            this.handleServerMessage(message);
        } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
        }
    }

    /**
     * Handle server messages
     * @param {Object} message - Parsed message from server
     */
    handleServerMessage(message) {
        switch (message.type) {
            case 'heartbeat':
                this.handleHeartbeat(message);
                break;

            case 'agent_connected':
                this.emit('agent_connected', message.data);
                break;

            case 'agent_disconnected':
                this.emit('agent_disconnected', message.data);
                break;

            case 'agent_status_update':
                this.emit('agent_status_update', message.data);
                break;

            case 'task_result':
                this.emit('task_result', message.data);
                break;

            case 'task_started':
                this.emit('task_started', message.data);
                break;

            case 'task_completed':
                this.emit('task_completed', message.data);
                break;

            case 'system_alert':
                this.emit('system_alert', message.data);
                break;

            case 'domain_rotation':
                this.emit('domain_rotation', message.data);
                break;

            case 'file_transfer_progress':
                this.emit('file_transfer_progress', message.data);
                break;

            case 'chat_message':
                this.emit('chat_message', message.data);
                break;

            case 'notification':
                this.emit('notification_received', message.data);
                break;

            default:
                console.log('Unknown WebSocket message type:', message.type);
        }
    }

    /**
     * Handle WebSocket error event
     */
    handleError(event) {
        console.error('WebSocket error:', event);
        this.emit('websocket_error', 'Connection error occurred');
    }

    /**
     * Handle WebSocket close event
     */
    handleClose(event) {
        console.log('WebSocket closed:', event.code, event.reason);
        this.isConnected = false;
        this.clearHeartbeat();

        this.emit('websocket_connected', false);

        // Attempt to reconnect if not a clean close
        if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
            this.scheduleReconnect();
        }
    }

    /**
     * Schedule a reconnection attempt
     */
    scheduleReconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('Max reconnection attempts reached');
            this.emit('websocket_error', 'Max reconnection attempts reached');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

        console.log(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

        setTimeout(() => {
            if (!this.isConnected && this.config) {
                this.connect(this.config);
            }
        }, delay);
    }

    /**
     * Start heartbeat mechanism
     */
    startHeartbeat() {
        this.clearHeartbeat();

        // Send heartbeat every 30 seconds
        this.heartbeatInterval = setInterval(() => {
            if (this.isConnected) {
                this.send({ type: 'heartbeat', timestamp: Date.now() });

                // Set timeout for heartbeat response
                this.heartbeatTimeout = setTimeout(() => {
                    console.warn('Heartbeat timeout - connection may be lost');
                    this.ws?.close();
                }, 10000);
            }
        }, 30000);
    }

    /**
     * Handle heartbeat response
     */
    handleHeartbeat(message) {
        if (this.heartbeatTimeout) {
            clearTimeout(this.heartbeatTimeout);
            this.heartbeatTimeout = null;
        }
    }

    /**
     * Clear heartbeat timers
     */
    clearHeartbeat() {
        if (this.heartbeatInterval) {
            clearInterval(this.heartbeatInterval);
            this.heartbeatInterval = null;
        }

        if (this.heartbeatTimeout) {
            clearTimeout(this.heartbeatTimeout);
            this.heartbeatTimeout = null;
        }
    }

    /**
     * Add event listener
     * @param {string} event - Event name
     * @param {Function} callback - Callback function
     */
    addEventListener(event, callback) {
        if (!this.eventListeners.has(event)) {
            this.eventListeners.set(event, []);
        }
        this.eventListeners.get(event).push(callback);
    }

    /**
     * Remove event listener
     * @param {string} event - Event name
     * @param {Function} callback - Callback function
     */
    removeEventListener(event, callback) {
        if (this.eventListeners.has(event)) {
            const listeners = this.eventListeners.get(event);
            const index = listeners.indexOf(callback);
            if (index > -1) {
                listeners.splice(index, 1);
            }
        }
    }

    /**
     * Emit event to all listeners
     * @param {string} event - Event name
     * @param {any} data - Event data
     */
    emit(event, data) {
        // Emit as custom DOM events for global listening
        const customEvent = new CustomEvent(event, { detail: data });
        document.dispatchEvent(customEvent);

        // Also emit to direct listeners
        if (this.eventListeners.has(event)) {
            this.eventListeners.get(event).forEach(callback => {
                try {
                    callback(data);
                } catch (error) {
                    console.error(`Error in WebSocket event listener for ${event}:`, error);
                }
            });
        }
    }

    /**
     * Get connection status
     * @returns {Object} Status object
     */
    getStatus() {
        return {
            connected: this.isConnected,
            readyState: this.ws?.readyState || WebSocket.CLOSED,
            reconnectAttempts: this.reconnectAttempts,
            maxReconnectAttempts: this.maxReconnectAttempts
        };
    }

    /**
     * Send command to agent through WebSocket
     * @param {string} agentId - Target agent ID
     * @param {string} command - Command to execute
     * @returns {Promise<string>} Task ID
     */
    async sendCommand(agentId, command) {
        return new Promise((resolve, reject) => {
            if (!this.isConnected) {
                reject(new Error('WebSocket not connected'));
                return;
            }

            const taskId = this.generateTaskId();
            const message = {
                type: 'execute_command',
                task_id: taskId,
                agent_id: agentId,
                command: command,
                timestamp: Date.now()
            };

            if (this.send(message)) {
                resolve(taskId);
            } else {
                reject(new Error('Failed to send command'));
            }
        });
    }

    /**
     * Upload file to agent through WebSocket
     * @param {string} agentId - Target agent ID
     * @param {string} localPath - Local file path
     * @param {string} remotePath - Remote file path
     * @returns {Promise<string>} Transfer ID
     */
    async uploadFile(agentId, localPath, remotePath) {
        return new Promise((resolve, reject) => {
            if (!this.isConnected) {
                reject(new Error('WebSocket not connected'));
                return;
            }

            const transferId = this.generateTaskId();
            const message = {
                type: 'upload_file',
                transfer_id: transferId,
                agent_id: agentId,
                local_path: localPath,
                remote_path: remotePath,
                timestamp: Date.now()
            };

            if (this.send(message)) {
                resolve(transferId);
            } else {
                reject(new Error('Failed to initiate file upload'));
            }
        });
    }

    /**
     * Download file from agent through WebSocket
     * @param {string} agentId - Target agent ID
     * @param {string} remotePath - Remote file path
     * @param {string} localPath - Local file path
     * @returns {Promise<string>} Transfer ID
     */
    async downloadFile(agentId, remotePath, localPath) {
        return new Promise((resolve, reject) => {
            if (!this.isConnected) {
                reject(new Error('WebSocket not connected'));
                return;
            }

            const transferId = this.generateTaskId();
            const message = {
                type: 'download_file',
                transfer_id: transferId,
                agent_id: agentId,
                remote_path: remotePath,
                local_path: localPath,
                timestamp: Date.now()
            };

            if (this.send(message)) {
                resolve(transferId);
            } else {
                reject(new Error('Failed to initiate file download'));
            }
        });
    }

    /**
     * Send chat message
     * @param {string} message - Chat message
     * @param {string} channel - Chat channel (optional)
     */
    sendChatMessage(message, channel = 'general') {
        if (this.isConnected) {
            this.send({
                type: 'chat_message',
                message: message,
                channel: channel,
                timestamp: Date.now(),
                username: this.config?.username || 'Anonymous'
            });
        }
    }

    /**
     * Generate unique task ID
     * @returns {string} Task ID
     */
    generateTaskId() {
        return 'task_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
    }
}

// Export singleton instance
export const websocketManager = new WebSocketManager();
