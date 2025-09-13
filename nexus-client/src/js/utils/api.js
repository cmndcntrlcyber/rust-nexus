/**
 * API wrapper for Tauri commands
 * Provides a clean interface to backend functionality
 */

export class API {
    constructor() {
        this.invoke = window.__TAURI__?.invoke || this.mockInvoke;
    }

    // Mock invoke for development without Tauri
    async mockInvoke(command, args) {
        console.warn(`Mock API call: ${command}`, args);

        // Return mock data for development
        switch (command) {
            case 'get_config':
                return null;
            case 'list_agents':
                return [];
            case 'get_domains':
                return [];
            case 'get_system_info':
                return {
                    version: '0.1.0',
                    platform: 'mock',
                    architecture: 'x64',
                    uptime: new Date().toISOString()
                };
            default:
                return null;
        }
    }

    // Configuration methods
    async loadConfig() {
        return this.invoke('load_config');
    }

    async saveConfig(config) {
        return this.invoke('save_config', { config });
    }

    async getConfig() {
        return this.invoke('get_config');
    }

    // Connection methods
    async connectToServer() {
        return this.invoke('connect_to_server');
    }

    async disconnectFromServer() {
        return this.invoke('disconnect_from_server');
    }

    async getConnectionStatus() {
        return this.invoke('get_connection_status');
    }

    // Agent management methods
    async listAgents() {
        return this.invoke('list_agents');
    }

    async getAgentDetails(agentId) {
        return this.invoke('get_agent_details', { agentId });
    }

    async interactWithAgent(agentId) {
        return this.invoke('interact_with_agent', { agentId });
    }

    async executeCommand(agentId, command) {
        return this.invoke('execute_command', { agentId, command });
    }

    // File management methods
    async listAgentFiles(agentId, path) {
        return this.invoke('list_agent_files', { agentId, path });
    }

    async uploadFileToAgent(agentId, localPath, remotePath) {
        return this.invoke('upload_file_to_agent', { agentId, localPath, remotePath });
    }

    async downloadFileFromAgent(agentId, remotePath, localPath) {
        return this.invoke('download_file_from_agent', { agentId, remotePath, localPath });
    }

    // Task management methods
    async executeTask(agentId, taskType, parameters) {
        return this.invoke('execute_task', { agentId, taskType, parameters });
    }

    async getTaskResults(taskId) {
        return this.invoke('get_task_results', { taskId });
    }

    async listTaskHistory(agentId, limit) {
        return this.invoke('list_task_history', { agentId, limit });
    }

    // BOF management methods
    async listAvailableBofs() {
        return this.invoke('list_available_bofs');
    }

    async executeBof(agentId, bofId, bofArguments) {
        return this.invoke('execute_bof', { agentId, bofId, arguments: bofArguments });
    }

    async uploadBof(filePath, metadata) {
        return this.invoke('upload_bof', { filePath, metadata });
    }

    // Infrastructure methods
    async getDomains() {
        return this.invoke('get_domains');
    }

    async rotateDomain() {
        return this.invoke('rotate_domain');
    }

    async getCertificates() {
        return this.invoke('get_certificates');
    }

    // Certificate management methods
    async uploadClientCertificate(filePath) {
        return this.invoke('upload_client_certificate', { filePath });
    }

    async uploadClientKey(filePath) {
        return this.invoke('upload_client_key', { filePath });
    }

    async uploadCaCertificate(filePath) {
        return this.invoke('upload_ca_certificate', { filePath });
    }

    async validateCertificateFiles() {
        return this.invoke('validate_certificate_files');
    }

    async getCertificateInfo(certPath) {
        return this.invoke('get_certificate_info', { certPath });
    }

    // Config file management methods
    async loadConfigFromFile(filePath) {
        return this.invoke('load_config_from_file', { filePath });
    }

    async validateConfig(config) {
        return this.invoke('validate_config', { config });
    }

    async connectWithConfig(config) {
        return this.invoke('connect_with_config', { config });
    }

    // Notification methods
    async showNotification(title, message, level) {
        return this.invoke('show_notification', { title, message, level });
    }

    // System methods
    async getSystemInfo() {
        return this.invoke('get_system_info');
    }

    async exportSessionData(filePath) {
        return this.invoke('export_session_data', { filePath });
    }

    async importSessionData(filePath) {
        return this.invoke('import_session_data', { filePath });
    }

    // Utility methods
    async selectFile(options = {}) {
        if (window.__TAURI__?.dialog) {
            return window.__TAURI__.dialog.open({
                multiple: false,
                directory: false,
                ...options
            });
        }
        return null;
    }

    async selectFolder(options = {}) {
        if (window.__TAURI__?.dialog) {
            return window.__TAURI__.dialog.open({
                multiple: false,
                directory: true,
                ...options
            });
        }
        return null;
    }

    async saveFile(options = {}) {
        if (window.__TAURI__?.dialog) {
            return window.__TAURI__.dialog.save(options);
        }
        return null;
    }

    // Helper for batch operations
    async executeBatch(operations) {
        const results = [];

        for (const operation of operations) {
            try {
                const result = await this.invoke(operation.command, operation.args);
                results.push({ success: true, result, operation: operation.command });
            } catch (error) {
                results.push({ success: false, error: error.message, operation: operation.command });
            }
        }

        return results;
    }

    // Error handling wrapper
    async safeInvoke(command, args = {}) {
        try {
            return { success: true, data: await this.invoke(command, args) };
        } catch (error) {
            console.error(`API call failed: ${command}`, error);
            return { success: false, error: error.message };
        }
    }
}

// Export singleton instance
export const api = new API();
