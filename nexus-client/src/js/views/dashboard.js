/**
 * Dashboard View Component
 * Main dashboard interface for the Nexus C2 Client
 */

export class Dashboard {
    constructor(app) {
        this.app = app;
        this.activities = [];
        this.maxActivities = 50;
        this.init();
    }

    init() {
        // Initialize dashboard components
        this.setupEventListeners();
    }

    setupEventListeners() {
        // Any dashboard-specific event listeners can be added here
    }

    render() {
        // Dashboard is rendered in HTML, this method handles dynamic updates
        this.updateStats();
        this.renderActivityFeed();
    }

    updateStats() {
        // Update dashboard statistics
        const activeAgents = this.app.agents ? Array.from(this.app.agents.values()).filter(agent =>
            agent.status?.toLowerCase() === 'active'
        ).length : 0;

        document.getElementById('dashboard-agents').textContent = activeAgents;
        document.getElementById('dashboard-tasks').textContent = this.app.taskCount || 0;
    }

    updateConnectionStatus(status) {
        // Update connection status display in dashboard
        const connectionCard = document.querySelector('.dashboard-card');
        if (connectionCard) {
            // Visual updates based on connection status
            if (status === 'connected') {
                connectionCard.classList.add('connected');
                connectionCard.classList.remove('disconnected', 'error');
            } else if (status.Error) {
                connectionCard.classList.add('error');
                connectionCard.classList.remove('connected', 'disconnected');
            } else {
                connectionCard.classList.add('disconnected');
                connectionCard.classList.remove('connected', 'error');
            }
        }
    }

    addActivity(type, description, level = 'info') {
        const activity = {
            id: Date.now().toString(),
            type: type,
            description: description,
            level: level,
            timestamp: new Date()
        };

        this.activities.unshift(activity);

        // Keep only the latest activities
        if (this.activities.length > this.maxActivities) {
            this.activities = this.activities.slice(0, this.maxActivities);
        }

        this.renderActivityFeed();
    }

    renderActivityFeed() {
        const feedElement = document.getElementById('activity-feed');
        if (!feedElement) return;

        if (this.activities.length === 0) {
            feedElement.innerHTML = `
                <div class="empty-state">
                    <i class="fas fa-clock"></i>
                    <p>No recent activity</p>
                </div>
            `;
            return;
        }

        feedElement.innerHTML = this.activities.map(activity => `
            <div class="activity-item ${activity.level}">
                <div class="activity-icon">
                    <i class="fas ${this.getActivityIcon(activity.type)}"></i>
                </div>
                <div class="activity-content">
                    <div class="activity-title">${activity.type}</div>
                    <div class="activity-description">${activity.description}</div>
                    <div class="activity-time">${this.formatTime(activity.timestamp)}</div>
                </div>
            </div>
        `).join('');
    }

    getActivityIcon(type) {
        const iconMap = {
            'Agent connected': 'fa-plug',
            'Agent disconnected': 'fa-unlink',
            'Task completed': 'fa-check-circle',
            'Task started': 'fa-play-circle',
            'File transfer': 'fa-exchange-alt',
            'Domain rotated': 'fa-sync-alt',
            'Certificate updated': 'fa-certificate',
            'Connection established': 'fa-link',
            'Connection lost': 'fa-exclamation-triangle'
        };
        return iconMap[type] || 'fa-info-circle';
    }

    formatTime(timestamp) {
        const now = new Date();
        const diff = now - timestamp;
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);

        if (days > 0) {
            return `${days}d ago`;
        } else if (hours > 0) {
            return `${hours}h ago`;
        } else if (minutes > 0) {
            return `${minutes}m ago`;
        } else {
            return 'Just now';
        }
    }

    onActivated() {
        // Called when dashboard tab is activated
        this.updateStats();
        this.renderActivityFeed();
    }

    destroy() {
        // Cleanup when dashboard is destroyed
        this.activities = [];
    }
}
