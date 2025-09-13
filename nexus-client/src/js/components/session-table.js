/**
 * Session Table Component
 * Displays and manages agent sessions in a table format
 */

export class SessionTable {
    constructor(app) {
        this.app = app;
        this.tableContainer = null;
        this.selectedAgent = null;
        this.sortColumn = 'hostname';
        this.sortDirection = 'asc';
        this.init();
    }

    init() {
        this.setupEventListeners();
    }

    setupEventListeners() {
        // Listen for agent updates
        document.addEventListener('agents_updated', (event) => {
            this.updateTable(event.detail);
        });
    }

    render(containerId = 'sessions-table') {
        this.tableContainer = document.getElementById(containerId);
        if (!this.tableContainer) return;

        this.tableContainer.innerHTML = `
            <div class="table-controls">
                <div class="search-container">
                    <input type="text" id="session-search" placeholder="Search sessions..." class="search-input">
                    <i class="fas fa-search search-icon"></i>
                </div>
                <div class="table-actions">
                    <button class="btn btn-small" id="refresh-sessions-btn">
                        <i class="fas fa-sync-alt"></i>
                        Refresh
                    </button>
                    <button class="btn btn-small" id="select-all-btn">
                        <i class="fas fa-check-square"></i>
                        Select All
                    </button>
                </div>
            </div>
            <div class="table-wrapper">
                <table class="table" id="sessions-table-data">
                    <thead>
                        <tr>
                            <th data-sort="select">
                                <input type="checkbox" id="select-all-checkbox">
                            </th>
                            <th data-sort="hostname" class="sortable">
                                Hostname
                                <i class="fas fa-sort sort-icon"></i>
                            </th>
                            <th data-sort="username" class="sortable">
                                User
                                <i class="fas fa-sort sort-icon"></i>
                            </th>
                            <th data-sort="os" class="sortable">
                                OS
                                <i class="fas fa-sort sort-icon"></i>
                            </th>
                            <th data-sort="ip" class="sortable">
                                IP Address
                                <i class="fas fa-sort sort-icon"></i>
                            </th>
                            <th data-sort="status" class="sortable">
                                Status
                                <i class="fas fa-sort sort-icon"></i>
                            </th>
                            <th data-sort="last_seen" class="sortable">
                                Last Seen
                                <i class="fas fa-sort sort-icon"></i>
                            </th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td colspan="8" class="empty-state">
                                <i class="fas fa-robot"></i>
                                <p>No active sessions</p>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        `;

        this.setupTableEventListeners();
    }

    setupTableEventListeners() {
        // Search functionality
        const searchInput = document.getElementById('session-search');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterTable(e.target.value);
            });
        }

        // Refresh button
        const refreshBtn = document.getElementById('refresh-sessions-btn');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', async () => {
                await this.refreshSessions();
            });
        }

        // Select all functionality
        const selectAllCheckbox = document.getElementById('select-all-checkbox');
        if (selectAllCheckbox) {
            selectAllCheckbox.addEventListener('change', (e) => {
                this.selectAllSessions(e.target.checked);
            });
        }

        // Table sorting
        const sortableHeaders = document.querySelectorAll('.sortable');
        sortableHeaders.forEach(header => {
            header.addEventListener('click', () => {
                const column = header.dataset.sort;
                this.sortTable(column);
            });
        });

        // Row selection
        const table = document.getElementById('sessions-table-data');
        if (table) {
            table.addEventListener('click', (e) => {
                const row = e.target.closest('tbody tr');
                if (row && !e.target.closest('input[type="checkbox"]') && !e.target.closest('button')) {
                    this.selectRow(row);
                }
            });

            // Double-click to interact
            table.addEventListener('dblclick', (e) => {
                const row = e.target.closest('tbody tr');
                if (row) {
                    const agentId = row.dataset.agentId;
                    if (agentId) {
                        this.app.selectAgent(agentId);
                    }
                }
            });
        }
    }

    updateTable(agents) {
        const tbody = document.querySelector('#sessions-table-data tbody');
        if (!tbody) return;

        if (!agents || agents.length === 0) {
            tbody.innerHTML = `
                <tr>
                    <td colspan="8" class="empty-state">
                        <i class="fas fa-robot"></i>
                        <p>No active sessions</p>
                    </td>
                </tr>
            `;
            return;
        }

        tbody.innerHTML = agents.map(agent => this.createRowHTML(agent)).join('');
        this.updateSortIcons();
    }

    createRowHTML(agent) {
        const info = agent.agent_info || {};
        const status = agent.status || 'unknown';
        const lastSeen = agent.last_seen ? new Date(agent.last_seen).toLocaleString() : 'Never';

        return `
            <tr data-agent-id="${agent.id}" class="session-row">
                <td>
                    <input type="checkbox" class="session-checkbox" value="${agent.id}">
                </td>
                <td class="hostname-cell">
                    <div class="agent-identity">
                        <div class="platform-icon ${this.getPlatformClass(info.os)}">
                            ${this.getPlatformIcon(info.os)}
                        </div>
                        <span class="hostname">${info.hostname || agent.id}</span>
                    </div>
                </td>
                <td class="user-cell">
                    <span class="username">${info.username || 'unknown'}</span>
                    <span class="domain">@${info.domain || 'unknown'}</span>
                </td>
                <td class="os-cell">
                    <span class="os-name">${info.os || 'Unknown'}</span>
                    <span class="os-version">${info.arch || ''}</span>
                </td>
                <td class="ip-cell">
                    <span class="ip-address">${info.internal_ip || 'N/A'}</span>
                    ${info.external_ip ? `<br><small class="external-ip">${info.external_ip}</small>` : ''}
                </td>
                <td class="status-cell">
                    <span class="status-badge ${status.toLowerCase()}">${status}</span>
                </td>
                <td class="last-seen-cell">
                    <span class="last-seen-time">${lastSeen}</span>
                </td>
                <td class="actions-cell">
                    <div class="action-buttons">
                        <button class="btn btn-small action-btn" onclick="window.nexusClient.selectAgent('${agent.id}')" title="Interact">
                            <i class="fas fa-terminal"></i>
                        </button>
                        <button class="btn btn-small action-btn" onclick="window.nexusClient.openFileBrowser('${agent.id}')" title="Files">
                            <i class="fas fa-folder"></i>
                        </button>
                        <button class="btn btn-small btn-danger action-btn" onclick="window.nexusClient.disconnectAgent('${agent.id}')" title="Disconnect">
                            <i class="fas fa-unlink"></i>
                        </button>
                    </div>
                </td>
            </tr>
        `;
    }

    getPlatformClass(os) {
        if (!os) return 'unknown';
        const osLower = os.toLowerCase();
        if (osLower.includes('windows')) return 'windows';
        if (osLower.includes('linux')) return 'linux';
        if (osLower.includes('macos') || osLower.includes('darwin')) return 'macos';
        return 'unknown';
    }

    getPlatformIcon(os) {
        const platformClass = this.getPlatformClass(os);
        switch (platformClass) {
            case 'windows': return '<i class="fab fa-windows"></i>';
            case 'linux': return '<i class="fab fa-linux"></i>';
            case 'macos': return '<i class="fab fa-apple"></i>';
            default: return '<i class="fas fa-desktop"></i>';
        }
    }

    selectRow(row) {
        // Remove previous selection
        document.querySelectorAll('.session-row').forEach(r => r.classList.remove('selected'));

        // Add selection to current row
        row.classList.add('selected');

        // Update selected agent
        this.selectedAgent = row.dataset.agentId;

        // Update checkbox
        const checkbox = row.querySelector('.session-checkbox');
        if (checkbox) {
            checkbox.checked = true;
        }
    }

    selectAllSessions(checked) {
        const checkboxes = document.querySelectorAll('.session-checkbox');
        checkboxes.forEach(checkbox => {
            checkbox.checked = checked;
        });
    }

    filterTable(searchTerm) {
        const rows = document.querySelectorAll('.session-row');
        const term = searchTerm.toLowerCase();

        rows.forEach(row => {
            const hostname = row.querySelector('.hostname')?.textContent.toLowerCase() || '';
            const username = row.querySelector('.username')?.textContent.toLowerCase() || '';
            const os = row.querySelector('.os-name')?.textContent.toLowerCase() || '';
            const ip = row.querySelector('.ip-address')?.textContent.toLowerCase() || '';

            const matches = hostname.includes(term) ||
                          username.includes(term) ||
                          os.includes(term) ||
                          ip.includes(term);

            row.style.display = matches ? '' : 'none';
        });
    }

    sortTable(column) {
        if (this.sortColumn === column) {
            this.sortDirection = this.sortDirection === 'asc' ? 'desc' : 'asc';
        } else {
            this.sortColumn = column;
            this.sortDirection = 'asc';
        }

        const tbody = document.querySelector('#sessions-table-data tbody');
        const rows = Array.from(tbody.querySelectorAll('.session-row'));

        rows.sort((a, b) => {
            let aValue = this.getCellValue(a, column);
            let bValue = this.getCellValue(b, column);

            // Handle different data types
            if (!isNaN(aValue) && !isNaN(bValue)) {
                aValue = parseFloat(aValue);
                bValue = parseFloat(bValue);
            }

            if (this.sortDirection === 'asc') {
                return aValue > bValue ? 1 : -1;
            } else {
                return aValue < bValue ? 1 : -1;
            }
        });

        // Re-append sorted rows
        rows.forEach(row => tbody.appendChild(row));
        this.updateSortIcons();
    }

    getCellValue(row, column) {
        const cellMap = {
            'hostname': '.hostname',
            'username': '.username',
            'os': '.os-name',
            'ip': '.ip-address',
            'status': '.status-badge',
            'last_seen': '.last-seen-time'
        };

        const selector = cellMap[column];
        const cell = row.querySelector(selector);
        return cell ? cell.textContent.trim() : '';
    }

    updateSortIcons() {
        // Reset all sort icons
        document.querySelectorAll('.sort-icon').forEach(icon => {
            icon.className = 'fas fa-sort sort-icon';
        });

        // Update current sort icon
        const currentHeader = document.querySelector(`[data-sort="${this.sortColumn}"] .sort-icon`);
        if (currentHeader) {
            currentHeader.className = `fas fa-sort-${this.sortDirection === 'asc' ? 'up' : 'down'} sort-icon active`;
        }
    }

    async refreshSessions() {
        try {
            const refreshBtn = document.getElementById('refresh-sessions-btn');
            if (refreshBtn) {
                refreshBtn.disabled = true;
                refreshBtn.innerHTML = '<i class="fas fa-spin fa-sync-alt"></i> Refreshing...';
            }

            const agents = await this.app.api.listAgents();
            this.updateTable(agents);

            if (refreshBtn) {
                refreshBtn.disabled = false;
                refreshBtn.innerHTML = '<i class="fas fa-sync-alt"></i> Refresh';
            }

        } catch (error) {
            console.error('Failed to refresh sessions:', error);
            this.app.showNotification('Refresh Failed', error.message, 'error');
        }
    }

    getSelectedSessions() {
        const checkboxes = document.querySelectorAll('.session-checkbox:checked');
        return Array.from(checkboxes).map(cb => cb.value);
    }

    destroy() {
        this.tableContainer = null;
        this.selectedAgent = null;
    }
}
