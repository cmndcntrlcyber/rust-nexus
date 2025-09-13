/**
 * Terminal Component
 * Interactive terminal interface for agent sessions
 */

export class Terminal {
    constructor(app, agentId) {
        this.app = app;
        this.agentId = agentId;
        this.terminal = null;
        this.fitAddon = null;
        this.commandHistory = [];
        this.historyIndex = -1;
        this.init();
    }

    init() {
        // Terminal initialization - simplified for now
    }

    render() {
        const panelId = `agent-${this.agentId}-panel`;
        const panel = document.getElementById(panelId);

        if (panel) {
            panel.innerHTML = `
                <div class="terminal-container">
                    <div class="terminal-header">
                        <span>Terminal - Agent ${this.agentId}</span>
                        <button class="btn btn-small" onclick="this.clear()">Clear</button>
                    </div>
                    <div class="terminal-content" id="terminal-${this.agentId}">
                        <div class="terminal-output">Ready for commands...</div>
                        <div class="terminal-input">
                            <span class="prompt">$ </span>
                            <input type="text" class="command-input" placeholder="Enter command..." />
                        </div>
                    </div>
                </div>
            `;
        }
    }

    displayTaskResult(result) {
        const output = document.querySelector(`#terminal-${this.agentId} .terminal-output`);
        if (output) {
            output.innerHTML += `<div class="command-result">${result.result || result.error}</div>`;
        }
    }

    onActivated() {
        // Called when terminal tab is activated
    }

    destroy() {
        // Cleanup terminal instance
        this.terminal = null;
    }
}
