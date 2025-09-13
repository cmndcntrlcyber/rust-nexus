/**
 * Settings View Component
 * Configuration and settings interface for the Nexus C2 Client
 */

export class Settings {
    constructor(app) {
        this.app = app;
        this.modal = null;
        this.form = null;
        this.init();
    }

    init() {
        this.modal = document.getElementById('settings-modal');
        this.form = document.getElementById('settings-form');
        this.setupEventListeners();
    }

    setupEventListeners() {
        // Save settings button
        const saveBtn = document.getElementById('save-settings-btn');
        if (saveBtn) {
            saveBtn.addEventListener('click', () => {
                this.saveSettings();
            });
        }

        // Form submission
        if (this.form) {
            this.form.addEventListener('submit', (e) => {
                e.preventDefault();
                this.saveSettings();
            });
        }
    }

    show() {
        if (this.modal) {
            this.loadCurrentSettings();
            this.app.showModal('settings-modal');
        }
    }

    hide() {
        if (this.modal) {
            this.app.hideModal('settings-modal');
        }
    }

    loadCurrentSettings() {
        const config = this.app.config;
        if (!config) return;

        // Load current configuration into form
        const serverEndpointInput = document.getElementById('server-endpoint-input');
        const serverPortInput = document.getElementById('server-port-input');
        const useTlsInput = document.getElementById('use-tls-input');
        const usernameInput = document.getElementById('username-input');
        const teamNameInput = document.getElementById('team-name-input');

        if (serverEndpointInput) serverEndpointInput.value = config.server_endpoint || '';
        if (serverPortInput) serverPortInput.value = config.server_port || '';
        if (useTlsInput) useTlsInput.checked = config.use_tls || false;
        if (usernameInput) usernameInput.value = config.username || '';
        if (teamNameInput) teamNameInput.value = config.team_name || '';
    }

    async saveSettings() {
        const formData = this.getFormData();

        try {
            this.app.showProgress('Saving settings...');

            // Validate settings
            if (!this.validateSettings(formData)) {
                this.app.hideProgress();
                return;
            }

            // Create configuration object
            const config = {
                server_endpoint: formData.serverEndpoint,
                server_port: parseInt(formData.serverPort),
                use_tls: formData.useTls,
                username: formData.username,
                team_name: formData.teamName,
                auto_connect: false,
                websocket_endpoint: `${formData.useTls ? 'wss' : 'ws'}://${formData.serverEndpoint}:${formData.serverPort}/ws`,
                update_interval_ms: 5000,
                max_concurrent_tasks: 10,
                log_level: 'info'
            };

            // Save configuration
            await this.app.api.saveConfig(config);

            // Update app configuration
            this.app.config = config;
            this.app.updateServerInfo();

            this.app.hideProgress();
            this.hide();

            this.app.showNotification('Settings Saved', 'Configuration updated successfully', 'success');

        } catch (error) {
            this.app.hideProgress();
            this.app.showNotification('Save Failed', error.message, 'error');
        }
    }

    getFormData() {
        const serverEndpointInput = document.getElementById('server-endpoint-input');
        const serverPortInput = document.getElementById('server-port-input');
        const useTlsInput = document.getElementById('use-tls-input');
        const usernameInput = document.getElementById('username-input');
        const teamNameInput = document.getElementById('team-name-input');

        return {
            serverEndpoint: serverEndpointInput?.value?.trim() || '',
            serverPort: serverPortInput?.value?.trim() || '443',
            useTls: useTlsInput?.checked || false,
            username: usernameInput?.value?.trim() || '',
            teamName: teamNameInput?.value?.trim() || ''
        };
    }

    validateSettings(formData) {
        const errors = [];

        // Validate server endpoint
        if (!formData.serverEndpoint) {
            errors.push('Server endpoint is required');
        }

        // Validate port
        const port = parseInt(formData.serverPort);
        if (!port || port < 1 || port > 65535) {
            errors.push('Port must be between 1 and 65535');
        }

        // Validate username
        if (!formData.username) {
            errors.push('Username is required');
        }

        // Validate team name
        if (!formData.teamName) {
            errors.push('Team name is required');
        }

        if (errors.length > 0) {
            this.app.showNotification('Validation Error', errors.join('\n'), 'error');
            return false;
        }

        return true;
    }

    updateConnectionStatus(status) {
        // Update any connection-related settings display
        // This could show current connection status in the settings modal
    }

    destroy() {
        // Cleanup when settings component is destroyed
        this.modal = null;
        this.form = null;
    }
}
