/**
 * Chat Component
 * Team chat functionality for the Nexus C2 Client
 */

export class Chat {
    constructor(app) {
        this.app = app;
        this.messages = [];
        this.init();
    }

    init() {
        this.setupEventListeners();
    }

    setupEventListeners() {
        // Send message button
        const sendBtn = document.getElementById('send-message-btn');
        if (sendBtn) {
            sendBtn.addEventListener('click', () => {
                this.sendMessage();
            });
        }

        // Chat input enter key
        const chatInput = document.getElementById('chat-input');
        if (chatInput) {
            chatInput.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') {
                    this.sendMessage();
                }
            });
        }
    }

    addMessage(message) {
        this.messages.push(message);
        this.renderMessages();
    }

    sendMessage() {
        const input = document.getElementById('chat-input');
        if (!input || !input.value.trim()) return;

        const message = {
            id: Date.now().toString(),
            username: this.app.config?.username || 'Anonymous',
            message: input.value.trim(),
            timestamp: new Date(),
            type: 'user'
        };

        this.addMessage(message);

        // Send via websocket if connected
        if (this.app.websocket?.isConnected) {
            this.app.websocket.sendChatMessage(message.message);
        }

        input.value = '';
    }

    renderMessages() {
        const container = document.getElementById('chat-messages');
        if (!container) return;

        if (this.messages.length === 0) {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="fas fa-comment"></i>
                    <p>No messages</p>
                </div>
            `;
            return;
        }

        container.innerHTML = this.messages.map(msg => `
            <div class="chat-message">
                <div class="message-header">
                    <span class="username">${msg.username}</span>
                    <span class="timestamp">${msg.timestamp.toLocaleTimeString()}</span>
                </div>
                <div class="message-content">${msg.message}</div>
            </div>
        `).join('');

        // Auto-scroll to bottom
        container.scrollTop = container.scrollHeight;
    }

    destroy() {
        this.messages = [];
    }
}
