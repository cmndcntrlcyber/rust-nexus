/**
 * Nexus C2 Client - Main Application
 * Initializes the Tauri-based desktop client interface
 */

import { API } from './utils/api.js';
import { WebSocketManager } from './utils/websocket.js';
import { Dashboard } from './views/dashboard.js';
import { Settings } from './views/settings.js';
import { SessionTable } from './components/session-table.js';
import { Chat } from './components/chat.js';
import { Terminal } from './components/terminal.js';
import { FileBrowser } from './components/file-browser.js';
import { BOFManager } from './components/bof-manager.js';

class NexusClient {
    constructor() {
        this.api = new API();
        this.websocket = new WebSocketManager();
        this.dashboard = null;
        this.settings = null;
        this.sessionTable = null;
        this.chat = null;
        this.activeTerminals = new Map();
        this.activeFileBrowsers = new Map();
        this.currentTab = 'dashboard';
        this.tabs = new Map();
        this.config = null;
        this.connectionStatus = 'disconnected';
        this.agents = new Map();
        this.notifications = [];
        this.taskCount = 0;

        this.init();
    }

    async init() {
        console.log('Initializing Nexus Client...');

        try {
            // Load configuration
            await this.loadConfig();

            // Initialize components
            this.initializeComponents();

            // Setup event listeners
            this.setupEventListeners();

            // Initialize UI state
            this.initializeUI();

            // Setup periodic updates
            this.setupPeriodicUpdates();

            console.log('Nexus Client initialized successfully');

            // Show welcome notification
            this.showNotification('Welcome to Nexus C2 Client', 'Client initialized successfully', 'success');

        } catch (error) {
            console.error('Failed to initialize Nexus Client:', error);
            this.showNotification('Initialization Error', `Failed to initialize client: ${error.message}`, 'error');
        }
    }

    async loadConfig() {
        try {
            this.config = await this.api.getConfig();
            if (this.config) {
                this.updateServerInfo();
            }
        } catch (error) {
            console.warn('Failed to load config, using defaults:', error);
            this.config = null;
        }
    }

    initializeComponents() {
        // Initialize dashboard
        this.dashboard = new Dashboard(this);

        // Initialize settings
        this.settings = new Settings(this);

        // Initialize session table
        this.sessionTable = new SessionTable(this);

        // Initialize chat
        this.chat = new Chat(this);

        // Initialize BOF manager
        this.bofManager = new BOFManager(this);

        // Register default tab
        this.tabs.set('dashboard', {
            id: 'dashboard',
            title: 'Dashboard',
            icon: 'fas fa-tachometer-alt',
            component: this.dashboard,
            closeable: false
        });
    }

    setupEventListeners() {
        // Tab management
        document.getElementById('tab-nav').addEventListener('click', (e) => {
            if (e.target.closest('.tab-item')) {
                const tabId = e.target.closest('.tab-item').dataset.tab;
                if (e.target.closest('.close-btn')) {
                    this.closeTab(tabId);
                } else {
                    this.switchTab(tabId);
                }
            }
        });

        // New tab button
        document.getElementById('new-tab-btn').addEventListener('click', () => {
            this.showNewTabMenu();
        });

        // Connection controls
        document.getElementById('connect-server-btn').addEventListener('click', async () => {
            await this.connectToServer();
        });

        // Settings
        document.getElementById('settings-btn').addEventListener('click', () => {
            this.settings.show();
        });

        // Notifications
        document.getElementById('notifications-btn').addEventListener('click', () => {
            this.showNotificationsModal();
        });

        // Minimize button
        document.getElementById('minimize-btn').addEventListener('click', () => {
            if (window.__TAURI__) {
                window.__TAURI__.appWindow.minimize();
            }
        });

        // Infrastructure controls
        document.getElementById('rotate-domain-btn').addEventListener('click', async () => {
            await this.rotateDomain();
        });

        document.getElementById('cert-status-btn').addEventListener('click', () => {
            this.showCertificateModal();
        });

        // Tool buttons
        document.getElementById('bof-manager-btn').addEventListener('click', () => {
            this.bofManager.show();
        });

        document.getElementById('file-manager-btn').addEventListener('click', () => {
            this.showFileManagerDialog();
        });

        // Export/Import
        document.getElementById('export-data-btn').addEventListener('click', async () => {
            await this.exportSessionData();
        });

        document.getElementById('import-bof-btn').addEventListener('click', () => {
            this.bofManager.showImportDialog();
        });

        // Modal close handlers
        document.querySelectorAll('.modal-close').forEach(btn => {
            btn.addEventListener('click', () => {
                const modalId = btn.dataset.modal;
                this.hideModal(modalId);
            });
        });

        // Modal overlay click to close
        document.getElementById('modal-overlay').addEventListener('click', (e) => {
            if (e.target === e.currentTarget) {
                this.hideAllModals();
            }
        });

        // Sidebar agent selection
        document.getElementById('agents-list').addEventListener('click', (e) => {
            if (e.target.closest('.agent-item')) {
                const agentId = e.target.closest('.agent-item').dataset.agentId;
                this.selectAgent(agentId);
            }
        });

        // WebSocket events
        this.setupWebSocketListeners();

        // Tauri events
        this.setupTauriEventListeners();
    }

    setupWebSocketListeners() {
        // Connection status
        document.addEventListener('websocket_connected', (event) => {
            const connected = event.detail;
            this.updateWebSocketStatus(connected);
        });

        document.addEventListener('websocket_error', (event) => {
            console.error('WebSocket error:', event.detail);
            this.showNotification('Connection Error', `WebSocket error: ${event.detail}`, 'error');
        });

        // Agent events
        document.addEventListener('agent_connected', (event) => {
            this.handleAgentConnected(event.detail);
        });

        document.addEventListener('agent_disconnected', (event) => {
            this.handleAgentDisconnected(event.detail);
        });

        document.addEventListener('agent_status_update', (event) => {
            this.handleAgentStatusUpdate(event.detail);
        });

        // Task events
        document.addEventListener('task_result', (event) => {
            this.handleTaskResult(event.detail);
        });

        document.addEventListener('task_started', (event) => {
            this.taskCount++;
            this.updateStatusBar();
        });

        document.addEventListener('task_completed', (event) => {
            this.taskCount = Math.max(0, this.taskCount - 1);
            this.updateStatusBar();
        });

        // System events
        document.addEventListener('system_alert', (event) => {
            const alert = event.detail;
            this.showNotification('System Alert', alert.message, alert.level || 'warning');
        });

        document.addEventListener('domain_rotation', (event) => {
            this.handleDomainRotation(event.detail);
        });

        // File transfer progress
        document.addEventListener('file_transfer_progress', (event) => {
            this.handleFileTransferProgress(event.detail);
        });

        // Chat messages
        document.addEventListener('chat_message', (event) => {
            this.chat.addMessage(event.detail);
        });

        // Notifications
        document.addEventListener('notification_received', (event) => {
            this.handleNotificationReceived(event.detail);
        });
    }

    setupTauriEventListeners() {
        if (!window.__TAURI__) return;

        // Listen for Tauri events
        window.__TAURI__.listen('connection_status_changed', (event) => {
            this.updateConnectionStatus(event.payload);
        });

        window.__TAURI__.listen('agents_updated', (event) => {
            this.updateAgentsList(event.payload);
        });
    }

    initializeUI() {
        // Update current time
        this.updateCurrentTime();

        // Initialize dashboard
        this.dashboard.render();

        // Update connection status
        this.updateConnectionStatus(this.connectionStatus);

        // Update agent stats
        this.updateAgentStats();

        // Load initial data
        this.loadInitialData();
    }

    async loadInitialData() {
        try {
            // Load agents
            const agents = await this.api.listAgents();
            this.updateAgentsList(agents);

            // Load domains
            const domains = await this.api.getDomains();
            this.updateDomainStatus(domains);

            // Load system info
            const systemInfo = await this.api.getSystemInfo();
            this.updateSystemInfo(systemInfo);

        } catch (error) {
            console.warn('Failed to load initial data:', error);
        }
    }

    setupPeriodicUpdates() {
        // Update current time every second
        setInterval(() => {
            this.updateCurrentTime();
        }, 1000);

        // Update agent status every 5 seconds if connected
        setInterval(async () => {
            if (this.connectionStatus === 'connected') {
                try {
                    const agents = await this.api.listAgents();
                    this.updateAgentsList(agents);
                } catch (error) {
                    console.warn('Failed to update agents:', error);
                }
            }
        }, 5000);

        // Update domain status every 30 seconds
        setInterval(async () => {
            if (this.connectionStatus === 'connected') {
                try {
                    const domains = await this.api.getDomains();
                    this.updateDomainStatus(domains);
                } catch (error) {
                    console.warn('Failed to update domains:', error);
                }
            }
        }, 30000);
    }

    // Tab management
    switchTab(tabId) {
        if (!this.tabs.has(tabId)) return;

        // Hide current tab
        document.querySelectorAll('.tab-item').forEach(tab => {
            tab.classList.remove('active');
        });
        document.querySelectorAll('.tab-panel').forEach(panel => {
            panel.classList.remove('active');
        });

        // Show new tab
        const tabElement = document.querySelector(`[data-tab="${tabId}"]`);
        const panelElement = document.getElementById(`${tabId}-panel`);

        if (tabElement && panelElement) {
            tabElement.classList.add('active');
            panelElement.classList.add('active');
            this.currentTab = tabId;

            // Notify component
            const tab = this.tabs.get(tabId);
            if (tab.component && typeof tab.component.onActivated === 'function') {
                tab.component.onActivated();
            }
        }
    }

    openAgentTab(agentId, agentData) {
        const tabId = `agent-${agentId}`;

        if (this.tabs.has(tabId)) {
            this.switchTab(tabId);
            return;
        }

        // Create terminal for agent
        const terminal = new Terminal(this, agentId);
        this.activeTerminals.set(agentId, terminal);

        // Register tab
        this.tabs.set(tabId, {
            id: tabId,
            title: `${agentData.hostname || agentId}`,
            icon: 'fas fa-terminal',
            component: terminal,
            closeable: true,
            agentId: agentId
        });

        // Create tab UI
        this.createTabUI(tabId);

        // Create panel UI
        terminal.render();

        // Switch to new tab
        this.switchTab(tabId);
    }

    createTabUI(tabId) {
        const tab = this.tabs.get(tabId);
        if (!tab) return;

        // Create tab item
        const tabItem = document.createElement('div');
        tabItem.className = 'tab-item';
        tabItem.dataset.tab = tabId;
        tabItem.innerHTML = `
            <i class="${tab.icon}"></i>
            <span>${tab.title}</span>
            ${tab.closeable ? '<button class="close-btn"><i class="fas fa-times"></i></button>' : ''}
        `;

        document.getElementById('tab-nav').appendChild(tabItem);

        // Create tab panel
        const panel = document.createElement('div');
        panel.className = 'tab-panel';
        panel.id = `${tabId}-panel`;

        document.getElementById('tab-content').appendChild(panel);
    }

    closeTab(tabId) {
        if (!this.tabs.has(tabId)) return;

        const tab = this.tabs.get(tabId);
        if (!tab.closeable) return;

        // Clean up component
        if (tab.component && typeof tab.component.destroy === 'function') {
            tab.component.destroy();
        }

        // Clean up terminal if exists
        if (tab.agentId && this.activeTerminals.has(tab.agentId)) {
            this.activeTerminals.delete(tab.agentId);
        }

        // Remove UI elements
        const tabElement = document.querySelector(`[data-tab="${tabId}"]`);
        const panelElement = document.getElementById(`${tabId}-panel`);

        if (tabElement) tabElement.remove();
        if (panelElement) panelElement.remove();

        // Remove from tabs
        this.tabs.delete(tabId);

        // Switch to dashboard if current tab was closed
        if (this.currentTab === tabId) {
            this.switchTab('dashboard');
        }
    }

    // Connection management
    async connectToServer() {
        if (!this.config) {
            this.showNotification('Configuration Required', 'Please configure server settings first', 'warning');
            this.settings.show();
            return;
        }

        this.showProgress('Connecting to server...');

        try {
            await this.api.connectToServer();
            this.showNotification('Connected', 'Successfully connected to server', 'success');
        } catch (error) {
            console.error('Connection failed:', error);
            this.showNotification('Connection Failed', error.message, 'error');
        } finally {
            this.hideProgress();
        }
    }

    async disconnectFromServer() {
        try {
            await this.api.disconnectFromServer();
            this.showNotification('Disconnected', 'Disconnected from server', 'info');
        } catch (error) {
            console.error('Disconnect failed:', error);
        }
    }

    // Agent management
    selectAgent(agentId) {
        // Update sidebar selection
        document.querySelectorAll('.agent-item').forEach(item => {
            item.classList.remove('selected');
        });

        const agentElement = document.querySelector(`[data-agent-id="${agentId}"]`);
        if (agentElement) {
            agentElement.classList.add('selected');

            // Open agent tab
            const agentData = this.agents.get(agentId);
            if (agentData) {
                this.openAgentTab(agentId, agentData);
            }
        }
    }

    updateAgentsList(agents) {
        this.agents.clear();

        const agentsList = document.getElementById('agents-list');

        if (!agents || agents.length === 0) {
            agentsList.innerHTML = `
                <div class="empty-state">
                    <i class="fas fa-robot"></i>
                    <p>No agents connected</p>
                </div>
            `;
            return;
        }

        agentsList.innerHTML = '';

        agents.forEach(agent => {
            this.agents.set(agent.id, agent);

            const agentElement = document.createElement('div');
            agentElement.className = 'agent-item';
            agentElement.dataset.agentId = agent.id;

            const platform = agent.agent_info?.os?.toLowerCase() || 'unknown';
            const platformClass = platform.includes('windows') ? 'windows' :
                                 platform.includes('linux') ? 'linux' :
                                 platform.includes('macos') || platform.includes('darwin') ? 'macos' : 'unknown';

            agentElement.innerHTML = `
                <div class="agent-icon ${platformClass}">
                    ${this.getPlatformIcon(platformClass)}
                </div>
                <div class="agent-details">
                    <div class="agent-hostname">${agent.agent_info?.hostname || agent.id}</div>
                    <div class="agent-info">
                        ${agent.agent_info?.username || 'unknown'}@${agent.agent_info?.domain || 'unknown'}
                    </div>
                </div>
                <div class="agent-status ${agent.status?.toLowerCase() || 'inactive'}"></div>
            `;

            agentsList.appendChild(agentElement);
        });

        this.updateAgentStats();
    }

    getPlatformIcon(platform) {
        switch (platform) {
            case 'windows': return '<i class="fab fa-windows"></i>';
            case 'linux': return '<i class="fab fa-linux"></i>';
            case 'macos': return '<i class="fab fa-apple"></i>';
            default: return '<i class="fas fa-desktop"></i>';
        }
    }

    updateAgentStats() {
        const total = this.agents.size;
        const active = Array.from(this.agents.values()).filter(agent =>
            agent.status?.toLowerCase() === 'active'
        ).length;

        document.getElementById('active-agents').textContent = active;
        document.getElementById('total-agents').textContent = total;
        document.getElementById('dashboard-agents').textContent = active;
    }

    // UI updates
    updateConnectionStatus(status) {
        this.connectionStatus = status;

        const statusElement = document.getElementById('connection-status');
        const indicator = statusElement.querySelector('.status-indicator');
        const text = statusElement.querySelector('.status-text');

        // Remove all status classes
        indicator.className = 'status-indicator';

        switch (status.toLowerCase()) {
            case 'connected':
                indicator.classList.add('connected');
                text.textContent = 'Connected';
                break;
            case 'connecting':
                indicator.classList.add('connecting');
                text.textContent = 'Connecting...';
                break;
            case 'disconnected':
                indicator.classList.add('disconnected');
                text.textContent = 'Disconnected';
                break;
            default:
                if (status.Error) {
                    indicator.classList.add('error');
                    text.textContent = 'Error';
                } else {
                    indicator.classList.add('disconnected');
                    text.textContent = status;
                }
        }

        // Update dashboard
        this.dashboard?.updateConnectionStatus(status);
    }

    updateServerInfo() {
        if (!this.config) return;

        const serverInfo = document.getElementById('server-endpoint');
        const endpoint = `${this.config.server_endpoint}:${this.config.server_port}`;
        serverInfo.textContent = endpoint;
    }

    updateWebSocketStatus(connected) {
        const wsStatus = document.getElementById('websocket-status');
        wsStatus.textContent = `WS: ${connected ? 'Connected' : 'Disconnected'}`;
        wsStatus.className = connected ? 'text-success' : 'text-danger';
    }

    updateCurrentTime() {
        const timeElement = document.getElementById('current-time');
        const now = new Date();
        timeElement.textContent = now.toLocaleTimeString();
    }

    updateStatusBar() {
        const taskCountElement = document.getElementById('task-count');
        taskCountElement.textContent = `Tasks: ${this.taskCount}`;
    }

    updateDomainStatus(domains) {
        const domainStatus = document.getElementById('domain-status');

        if (!domains || domains.length === 0) {
            domainStatus.innerHTML = `
                <div class="domain-item">
                    <span class="domain-name">No domains</span>
                    <span class="domain-health unknown">?</span>
                </div>
            `;
            return;
        }

        domainStatus.innerHTML = '';

        domains.forEach(domain => {
            const domainElement = document.createElement('div');
            domainElement.className = 'domain-item';

            const healthClass = domain.certificate_valid ? 'healthy' : 'error';
            const healthText = domain.certificate_valid ? '✓' : '✗';

            domainElement.innerHTML = `
                <span class="domain-name">${domain.domain}</span>
                <span class="domain-health ${healthClass}">${healthText}</span>
            `;

            domainStatus.appendChild(domainElement);
        });

        // Update dashboard
        document.getElementById('dashboard-domains').textContent = domains.length;
    }

    // Event handlers
    handleAgentConnected(agentData) {
        this.showNotification(
            'Agent Connected',
            `${agentData.hostname || agentData.id} connected`,
            'success'
        );

        // Add to dashboard activity
        this.dashboard?.addActivity('Agent connected', agentData.hostname || agentData.id, 'success');
    }

    handleAgentDisconnected(agentId) {
        const agent = this.agents.get(agentId);
        this.showNotification(
            'Agent Disconnected',
            `${agent?.agent_info?.hostname || agentId} disconnected`,
            'warning'
        );

        // Close agent tab if open
        this.closeTab(`agent-${agentId}`);

        // Add to dashboard activity
        this.dashboard?.addActivity('Agent disconnected', agent?.agent_info?.hostname || agentId, 'warning');
    }

    handleTaskResult(result) {
        console.log('Task result received:', result);

        // Update terminal if task belongs to an active session
        if (result.agent_id && this.activeTerminals.has(result.agent_id)) {
            const terminal = this.activeTerminals.get(result.agent_id);
            terminal.displayTaskResult(result);
        }

        // Add to dashboard activity
        this.dashboard?.addActivity('Task completed', result.task_type || 'Unknown', 'success');
    }

    handleFileTransferProgress(progress) {
        this.showProgress(`File Transfer: ${Math.round(progress.percentage)}%`);

        if (progress.status === 'Completed') {
            setTimeout(() => {
                this.hideProgress();
                this.showNotification(
                    'Transfer Complete',
                    `File ${progress.file_name} transferred successfully`,
                    'success'
                );
            }, 1000);
        }
    }

    handleDomainRotation(data) {
        this.showNotification(
            'Domain Rotated',
            `Rotated from ${data.old_domain} to ${data.new_domain}`,
            'info'
        );
    }

    handleNotificationReceived(notification) {
        this.notifications.unshift(notification);

        // Update notification count
        const countElement = document.getElementById('notification-count');
        countElement.textContent = this.notifications.filter(n => !n.read).length;

        // Show system notification
        this.showNotification(notification.title, notification.message, notification.level);
    }

    // Infrastructure management
    async rotateDomain() {
        this.showProgress('Rotating domain...');

        try {
            const result = await this.api.rotateDomain();
            this.showNotification('Domain Rotation', result, 'success');
        } catch (error) {
            this.showNotification('Domain Rotation Failed', error.message, 'error');
        } finally {
            this.hideProgress();
        }
    }

    async exportSessionData() {
        try {
            if (window.__TAURI__) {
                const filePath = await window.__TAURI__.api.dialog.save({
                    defaultPath: 'nexus-session-export.json',
                    filters: [{ name: 'JSON', extensions: ['json'] }]
                });

                if (filePath) {
                    await this.api.exportSessionData(filePath);
                    this.showNotification('Export Complete', 'Session data exported successfully', 'success');
                }
            }
        } catch (error) {
            this.showNotification('Export Failed', error.message, 'error');
        }
    }

    // UI utilities
    showModal(modalId) {
        const overlay = document.getElementById('modal-overlay');
        const modal = document.getElementById(modalId);

        if (overlay && modal) {
            overlay.classList.add('active');
            modal.style.display = 'block';
        }
    }

    hideModal(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.style.display = 'none';
        }

        // Hide overlay if no modals are visible
        const visibleModals = document.querySelectorAll('.modal[style*="block"]');
        if (visibleModals.length === 0) {
            document.getElementById('modal-overlay').classList.remove('active');
        }
    }

    hideAllModals() {
        document.getElementById('modal-overlay').classList.remove('active');
        document.querySelectorAll('.modal').forEach(modal => {
            modal.style.display = 'none';
        });
    }

    showProgress(title) {
        const overlay = document.getElementById('progress-overlay');
        const titleElement = document.getElementById('progress-title');

        if (overlay && titleElement) {
            titleElement.textContent = title;
            overlay.classList.add('active');
        }
    }

    hideProgress() {
        const overlay = document.getElementById('progress-overlay');
        if (overlay) {
            overlay.classList.remove('active');
        }
    }

    showNotification(title, message, level = 'info') {
        // Create notification element
        const notification = document.createElement('div');
        notification.className = `notification ${level}`;

        notification.innerHTML = `
            <div class="notification-header">
                <div class="notification-title">${title}</div>
                <button class="notification-close">&times;</button>
            </div>
            <div class="notification-body">${message}</div>
            <div class="notification-time">${new Date().toLocaleTimeString()}</div>
        `;

        document.body.appendChild(notification);

        // Show notification
        setTimeout(() => {
            notification.classList.add('show');
        }, 100);

        // Close handler
        const closeBtn = notification.querySelector('.notification-close');
        const closeNotification = () => {
            notification.classList.remove('show');
            setTimeout(() => {
                if (notification.parentNode) {
                    notification.parentNode.removeChild(notification);
                }
            }, 300);
        };

        closeBtn.addEventListener('click', closeNotification);

        // Auto close after 5 seconds
        setTimeout(closeNotification, 5000);
    }

    showNotificationsModal() {
        const modal = document.getElementById('notifications-modal');
        const list = document.getElementById('notifications-list');

        if (this.notifications.length === 0) {
            list.innerHTML = `
                <div class="empty-state">
                    <i class="fas fa-bell-slash"></i>
                    <p>No notifications</p>
                </div>
            `;
        } else {
            list.innerHTML = '';
            this.notifications.forEach(notification => {
                const item = document.createElement('div');
                item.className = `notification-item ${notification.read ? 'read' : 'unread'}`;
                item.innerHTML = `
                    <div class="notification-header">
                        <strong>${notification.title}</strong>
                        <span class="notification-time">${new Date(notification.timestamp).toLocaleString()}</span>
                    </div>
                    <div class="notification-body">${notification.message}</div>
                    <div class="notification-level badge ${notification.level}">${notification.level}</div>
                `;
                list.appendChild(item);
            });
        }

        this.showModal('notifications-modal');
    }

    setStatusMessage(message) {
        const statusElement = document.getElementById('status-message');
        if (statusElement) {
            statusElement.textContent = message;
        }
    }

    // Certificate management
    showCertificateModal() {
        this.setupCertificateModal();
        this.showModal('certificate-modal');
    }

    setupCertificateModal() {
        // Setup file upload handlers
        this.setupFileUploadArea('client-cert', 'client.crt', async (filePath) => {
            try {
                const result = await this.api.uploadClientCertificate(filePath);
                this.updateUploadStatus('client-cert-status', 'success', result);
                this.validateCertificateFiles();
            } catch (error) {
                this.updateUploadStatus('client-cert-status', 'error', error.message);
            }
        });

        this.setupFileUploadArea('client-key', 'client.key', async (filePath) => {
            try {
                const result = await this.api.uploadClientKey(filePath);
                this.updateUploadStatus('client-key-status', 'success', result);
                this.validateCertificateFiles();
            } catch (error) {
                this.updateUploadStatus('client-key-status', 'error', error.message);
            }
        });

        this.setupFileUploadArea('ca-cert', 'ca.crt', async (filePath) => {
            try {
                const result = await this.api.uploadCaCertificate(filePath);
                this.updateUploadStatus('ca-cert-status', 'success', result);
                this.validateCertificateFiles();
            } catch (error) {
                this.updateUploadStatus('ca-cert-status', 'error', error.message);
            }
        });

        // Setup upload button handler
        document.getElementById('upload-certificates-btn').addEventListener('click', async () => {
            await this.validateCertificateFiles();
            this.hideModal('certificate-modal');
            this.showNotification('Certificates', 'Certificate validation complete', 'success');
        });
    }

    setupFileUploadArea(areaId, fileName, uploadHandler) {
        const uploadArea = document.getElementById(`${areaId}-upload`);
        const fileInput = document.getElementById(`${areaId}-input`);

        // Click to upload
        uploadArea.addEventListener('click', () => {
            fileInput.click();
        });

        // File selection handler
        fileInput.addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (file) {
                this.updateUploadStatus(`${areaId}-status`, 'uploading', `Uploading ${file.name}...`);
                try {
                    // In a real Tauri app, you'd get the file path differently
                    // For now, we'll simulate with the file name
                    await uploadHandler(file.path || file.name);
                } catch (error) {
                    this.updateUploadStatus(`${areaId}-status`, 'error', error.message);
                }
            }
        });

        // Drag and drop
        uploadArea.addEventListener('dragover', (e) => {
            e.preventDefault();
            uploadArea.classList.add('dragover');
        });

        uploadArea.addEventListener('dragleave', () => {
            uploadArea.classList.remove('dragover');
        });

        uploadArea.addEventListener('drop', async (e) => {
            e.preventDefault();
            uploadArea.classList.remove('dragover');

            const files = Array.from(e.dataTransfer.files);
            const file = files.find(f => f.name.endsWith(fileName.split('.')[1]));

            if (file) {
                this.updateUploadStatus(`${areaId}-status`, 'uploading', `Uploading ${file.name}...`);
                try {
                    await uploadHandler(file.path || file.name);
                } catch (error) {
                    this.updateUploadStatus(`${areaId}-status`, 'error', error.message);
                }
            } else {
                this.updateUploadStatus(`${areaId}-status`, 'error', `Please select a ${fileName} file`);
            }
        });
    }

    updateUploadStatus(statusId, type, message) {
        const statusElement = document.getElementById(statusId);
        if (statusElement) {
            statusElement.className = `upload-status ${type}`;
            statusElement.textContent = message;
        }

        // Update upload area appearance
        const areaId = statusId.replace('-status', '-upload');
        const uploadArea = document.getElementById(areaId);
        if (uploadArea) {
            uploadArea.classList.remove('uploaded', 'validation-error', 'validation-success');
            if (type === 'success') {
                uploadArea.classList.add('uploaded', 'validation-success');
            } else if (type === 'error') {
                uploadArea.classList.add('validation-error');
            }
        }
    }

    async validateCertificateFiles() {
        try {
            const validation = await this.api.validateCertificateFiles();
            const uploadBtn = document.getElementById('upload-certificates-btn');

            if (validation.client_cert_valid && validation.client_key_valid && validation.ca_cert_valid) {
                uploadBtn.disabled = false;
                this.showCertificateInfo(validation);
            } else {
                uploadBtn.disabled = true;
                console.log('Certificate validation errors:', validation.errors);
            }
        } catch (error) {
            console.error('Certificate validation failed:', error);
        }
    }

    showCertificateInfo(validation) {
        const infoDiv = document.getElementById('certificate-info');
        const detailsDiv = document.getElementById('cert-details');

        if (validation.client_cert_info || validation.ca_cert_info) {
            let html = '';

            if (validation.client_cert_info) {
                html += `
                    <div class="cert-detail-item">
                        <span class="cert-detail-label">Client Certificate:</span>
                        <span class="cert-detail-value">${validation.client_cert_info.domain}</span>
                    </div>
                    <div class="cert-detail-item">
                        <span class="cert-detail-label">Valid Until:</span>
                        <span class="cert-detail-value">${new Date(validation.client_cert_info.valid_to).toLocaleDateString()}</span>
                    </div>
                `;
            }

            if (validation.ca_cert_info) {
                html += `
                    <div class="cert-detail-item">
                        <span class="cert-detail-label">CA Certificate:</span>
                        <span class="cert-detail-value">${validation.ca_cert_info.issuer}</span>
                    </div>
                `;
            }

            detailsDiv.innerHTML = html;
            infoDiv.style.display = 'block';
        }
    }

    // Config file management
    showConfigModal() {
        this.setupConfigModal();
        this.showModal('config-modal');
    }

    setupConfigModal() {
        const uploadArea = document.getElementById('config-file-upload');
        const fileInput = document.getElementById('config-file-input');
        const connectBtn = document.getElementById('connect-with-config-btn');

        let selectedConfig = null;

        // Click to upload
        uploadArea.addEventListener('click', () => {
            fileInput.click();
        });

        // File selection handler
        fileInput.addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (file) {
                try {
                    this.updateUploadStatus('config-file-status', 'uploading', 'Loading configuration...');

                    // Read file content (in real Tauri app, you'd use the file path)
                    const text = await file.text();
                    selectedConfig = JSON.parse(text);

                    // Validate config
                    const validation = await this.api.validateConfig(selectedConfig);

                    if (validation.is_valid) {
                        this.updateUploadStatus('config-file-status', 'success', 'Configuration loaded successfully');
                        this.showConfigPreview(selectedConfig);
                        connectBtn.disabled = false;
                    } else {
                        this.updateUploadStatus('config-file-status', 'error', 'Invalid configuration: ' + validation.errors.join(', '));
                        connectBtn.disabled = true;
                    }
                } catch (error) {
                    this.updateUploadStatus('config-file-status', 'error', 'Failed to load configuration: ' + error.message);
                    connectBtn.disabled = true;
                }
            }
        });

        // Connect button handler
        connectBtn.addEventListener('click', async () => {
            if (selectedConfig) {
                this.hideModal('config-modal');
                this.showProgress('Connecting with configuration...');

                try {
                    await this.api.connectWithConfig(selectedConfig);
                    this.showNotification('Connected', 'Successfully connected to server with provided configuration', 'success');
                    this.config = selectedConfig;
                    this.updateServerInfo();
                } catch (error) {
                    this.showNotification('Connection Failed', error.message, 'error');
                } finally {
                    this.hideProgress();
                }
            }
        });
    }

    showConfigPreview(config) {
        const previewDiv = document.getElementById('config-preview');
        const detailsDiv = document.getElementById('config-details');

        const sensitiveKeys = ['cert_path', 'key_path', 'ca_cert_path'];
        let html = '';

        Object.entries(config).forEach(([key, value]) => {
            const isSensitive = sensitiveKeys.includes(key);
            const displayValue = isSensitive ? '***' : String(value);

            html += `
                <div class="config-item">
                    <span class="config-key">${key}:</span>
                    <span class="config-value ${isSensitive ? 'sensitive' : ''}">${displayValue}</span>
                </div>
            `;
        });

        detailsDiv.innerHTML = html;
        previewDiv.style.display = 'block';
    }

    // Updated connection method - show new connection config modal
    async connectToServer() {
        this.showConnectionConfigModal();
    }

    // Connection Configuration Modal
    showConnectionConfigModal() {
        this.setupConnectionConfigModal();
        this.showModal('connection-config-modal');
    }

    setupConnectionConfigModal() {
        // Form elements
        const form = document.getElementById('connection-config-form');
        const domainInput = document.getElementById('domain-input');
        const portInput = document.getElementById('port-input');
        const wsEndpointDisplay = document.getElementById('websocket-endpoint-display');
        const usernameInput = document.getElementById('username-config-input');
        const teamNameInput = document.getElementById('team-name-input');
        const updateIntervalInput = document.getElementById('update-interval-input');
        const maxTasksInput = document.getElementById('max-tasks-input');
        const logLevelInput = document.getElementById('log-level-input');

        // Buttons
        const saveOnlyBtn = document.getElementById('save-config-only-btn');
        const connectBtn = document.getElementById('connect-with-new-config-btn');

        // Track uploaded certificates
        let uploadedCerts = {
            clientCert: null,
            clientKey: null,
            caCert: null
        };

        // Configuration preview
        let configPreview = null;

        // Auto-populate websocket endpoint and update preview
        const updateWebSocketEndpoint = () => {
            const domain = domainInput.value.trim();
            const port = portInput.value.trim();

            if (domain && port) {
                wsEndpointDisplay.value = `wss://${domain}:${port}/ws`;
            } else {
                wsEndpointDisplay.value = '';
            }

            updateConfigurationPreview();
            validateForm();
        };

        // Update configuration preview in real-time
        const updateConfigurationPreview = () => {
            const previewConfig = this.buildConfigFromForm(uploadedCerts, true);

            // Create or update preview panel
            let previewPanel = document.getElementById('config-preview-panel');
            if (!previewPanel) {
                previewPanel = document.createElement('div');
                previewPanel.id = 'config-preview-panel';
                previewPanel.className = 'config-section';
                previewPanel.innerHTML = `
                    <h4><i class="fas fa-eye"></i> Configuration Preview</h4>
                    <div class="config-preview-content">
                        <pre id="config-preview-json"></pre>
                    </div>
                `;

                const configSections = document.querySelector('.config-sections');
                configSections.appendChild(previewPanel);
            }

            // Update JSON preview
            const jsonPreview = document.getElementById('config-preview-json');
            jsonPreview.textContent = JSON.stringify(previewConfig, null, 2);
        };

        // Form validation with visual feedback
        const validateForm = () => {
            const domain = domainInput.value.trim();
            const port = portInput.value.trim();
            const username = usernameInput.value.trim();
            const teamName = teamNameInput.value.trim();

            const isValid = domain &&
                           port &&
                           username &&
                           teamName &&
                           uploadedCerts.clientCert &&
                           uploadedCerts.clientKey &&
                           uploadedCerts.caCert;

            saveOnlyBtn.disabled = !isValid;
            connectBtn.disabled = !isValid;

            // Update form field validation states
            this.updateInputValidation(domainInput, domain);
            this.updateInputValidation(portInput, port && port > 0 && port <= 65535);
            this.updateInputValidation(usernameInput, username);
            this.updateInputValidation(teamNameInput, teamName);

            // Update certificate validation states
            this.updateCertificateValidation('config-client-cert', uploadedCerts.clientCert);
            this.updateCertificateValidation('config-client-key', uploadedCerts.clientKey);
            this.updateCertificateValidation('config-ca-cert', uploadedCerts.caCert);

            // Update buttons text based on validation
            if (isValid) {
                saveOnlyBtn.innerHTML = '<i class="fas fa-save"></i> Save Configuration';
                connectBtn.innerHTML = '<i class="fas fa-plug"></i> Connect to Server';
            } else {
                saveOnlyBtn.innerHTML = '<i class="fas fa-exclamation-triangle"></i> Complete Required Fields';
                connectBtn.innerHTML = '<i class="fas fa-exclamation-triangle"></i> Complete Required Fields';
            }

            updateConfigurationPreview();
        };

        // Event listeners
        domainInput.addEventListener('input', updateWebSocketEndpoint);
        portInput.addEventListener('input', updateWebSocketEndpoint);

        // Validate other fields and update preview
        [usernameInput, teamNameInput, updateIntervalInput, maxTasksInput, logLevelInput].forEach(input => {
            input.addEventListener('input', () => {
                updateConfigurationPreview();
                validateForm();
            });
        });

        // Setup certificate file uploads
        this.setupConfigFileUpload('config-client-cert', 'client.crt', (filePath) => {
            uploadedCerts.clientCert = filePath;
            validateForm();
        });

        this.setupConfigFileUpload('config-client-key', 'client.key', (filePath) => {
            uploadedCerts.clientKey = filePath;
            validateForm();
        });

        this.setupConfigFileUpload('config-ca-cert', 'ca.crt', (filePath) => {
            uploadedCerts.caCert = filePath;
            validateForm();
        });

        // Button handlers
        saveOnlyBtn.addEventListener('click', async () => {
            const config = this.buildConfigFromForm(uploadedCerts);

            try {
                await this.api.saveConfig(config);
                this.hideModal('connection-config-modal');
                this.showNotification('Configuration Saved', 'Configuration saved successfully', 'success');
                this.config = config;
                this.updateServerInfo();
            } catch (error) {
                this.showNotification('Save Failed', error.message, 'error');
            }
        });

        connectBtn.addEventListener('click', async () => {
            const config = this.buildConfigFromForm(uploadedCerts);

            this.hideModal('connection-config-modal');
            this.showProgress('Connecting to server...');

            try {
                // First save the config
                await this.api.saveConfig(config);

                // Then connect with it
                await this.api.connectWithConfig(config);

                this.showNotification('Connected', 'Successfully connected to server', 'success');
                this.config = config;
                this.updateServerInfo();
            } catch (error) {
                this.showNotification('Connection Failed', error.message, 'error');
            } finally {
                this.hideProgress();
            }
        });

        // Initialize form validation and preview
        updateConfigurationPreview();
        validateForm();
    }

    setupConfigFileUpload(areaId, fileName, onUpload) {
        const uploadArea = document.getElementById(`${areaId}-upload`);
        const fileInput = document.getElementById(`${areaId}-input`);
        const statusElement = document.getElementById(`${areaId}-status`);

        if (!uploadArea || !fileInput || !statusElement) {
            console.warn(`File upload setup failed: missing elements for ${areaId}`);
            return;
        }

        // Click to upload
        uploadArea.addEventListener('click', () => {
            fileInput.click();
        });

        // File selection handler
        fileInput.addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (file) {
                this.updateConfigUploadStatus(areaId, 'uploading', `Uploading ${file.name}...`);

                try {
                    // In a real Tauri app, you'd get the file path
                    // For now, simulate the upload process
                    await this.simulateFileUpload(file, fileName);

                    const filePath = `./certs/${fileName}`;
                    this.updateConfigUploadStatus(areaId, 'success', `${file.name} uploaded successfully`);

                    onUpload(filePath);
                } catch (error) {
                    this.updateConfigUploadStatus(areaId, 'error', error.message);
                }
            }
        });

        // Drag and drop
        uploadArea.addEventListener('dragover', (e) => {
            e.preventDefault();
            uploadArea.classList.add('dragover');
        });

        uploadArea.addEventListener('dragleave', () => {
            uploadArea.classList.remove('dragover');
        });

        uploadArea.addEventListener('drop', async (e) => {
            e.preventDefault();
            uploadArea.classList.remove('dragover');

            const files = Array.from(e.dataTransfer.files);
            const file = files.find(f => f.name.endsWith(fileName.split('.')[1]));

            if (file) {
                this.updateConfigUploadStatus(areaId, 'uploading', `Uploading ${file.name}...`);

                try {
                    await this.simulateFileUpload(file, fileName);
                    const filePath = `./certs/${fileName}`;
                    this.updateConfigUploadStatus(areaId, 'success', `${file.name} uploaded successfully`);

                    onUpload(filePath);
                } catch (error) {
                    this.updateConfigUploadStatus(areaId, 'error', error.message);
                }
            } else {
                this.updateConfigUploadStatus(areaId, 'error', `Please select a ${fileName} file`);
            }
        });
    }

    updateConfigUploadStatus(areaId, type, message) {
        const statusElement = document.getElementById(`${areaId}-status`);
        const uploadArea = document.getElementById(`${areaId}-upload`);

        if (statusElement) {
            statusElement.className = `upload-status ${type}`;
            statusElement.textContent = message;
        }

        if (uploadArea) {
            uploadArea.classList.remove('uploaded', 'validation-error', 'validation-success');

            if (type === 'success') {
                uploadArea.classList.add('uploaded', 'validation-success');
            } else if (type === 'error') {
                uploadArea.classList.add('validation-error');
            }
        }
    }

    updateInputValidation(input, isValid) {
        input.classList.remove('valid', 'invalid');

        if (input.value.trim()) {
            input.classList.add(isValid ? 'valid' : 'invalid');
        }
    }

    updateCertificateValidation(areaId, isValid) {
        const uploadArea = document.getElementById(`${areaId}-upload`);
        if (!uploadArea) return;

        uploadArea.classList.remove('validation-success', 'validation-error', 'validation-pending');

        if (isValid) {
            uploadArea.classList.add('validation-success');
        } else {
            uploadArea.classList.add('validation-pending');
        }
    }

    async simulateFileUpload(file, expectedFileName) {
        // Simulate file upload process
        return new Promise((resolve, reject) => {
            setTimeout(() => {
                if (file.name.endsWith(expectedFileName.split('.')[1])) {
                    resolve();
                } else {
                    reject(new Error(`Expected ${expectedFileName} file`));
                }
            }, 500);
        });
    }

    buildConfigFromForm(uploadedCerts) {
        const domainInput = document.getElementById('domain-input');
        const portInput = document.getElementById('port-input');
        const usernameInput = document.getElementById('username-config-input');
        const teamNameInput = document.getElementById('team-name-input');
        const updateIntervalInput = document.getElementById('update-interval-input');
        const maxTasksInput = document.getElementById('max-tasks-input');
        const logLevelInput = document.getElementById('log-level-input');

        const domain = domainInput ? domainInput.value.trim() : '';
        const port = portInput ? parseInt(portInput.value.trim()) || 443 : 443;
        const username = usernameInput ? usernameInput.value.trim() : '';
        const teamName = teamNameInput ? teamNameInput.value.trim() : '';

        return {
            server_endpoint: domain,
            server_port: port,
            use_tls: true,
            cert_path: uploadedCerts.clientCert || "./certs/client.crt",
            key_path: uploadedCerts.clientKey || "./certs/client.key",
            ca_cert_path: uploadedCerts.caCert || "./certs/ca.crt",
            username: username,
            team_name: teamName,
            auto_connect: false,
            websocket_endpoint: `wss://${domain}:${port}/ws`,
            update_interval_ms: updateIntervalInput ? parseInt(updateIntervalInput.value) || 5000 : 5000,
            max_concurrent_tasks: maxTasksInput ? parseInt(maxTasksInput.value) || 10 : 10,
            log_level: logLevelInput ? logLevelInput.value || 'info' : 'info'
        };
    }
}

// Initialize application when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.nexusClient = new NexusClient();
});

// Export for debugging
window.NexusClient = NexusClient;
