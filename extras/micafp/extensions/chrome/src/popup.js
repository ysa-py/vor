// Shield Chrome Extension Popup Controller
// Handles UI interactions and communicates with background service worker

document.addEventListener('DOMContentLoaded', () => {
    const statusCircle = document.getElementById('statusCircle');
    const statusText = document.getElementById('statusText');
    const connectBtn = document.getElementById('connectBtn');
    const settingsBtn = document.getElementById('settingsBtn');
    const settingsPanel = document.getElementById('settingsPanel');
    const serverSelect = document.getElementById('serverSelect');
    const uptimeEl = document.getElementById('uptime');
    const downloadEl = document.getElementById('download');
    const uploadEl = document.getElementById('upload');
    const latencyEl = document.getElementById('latency');

    let isConnected = false;
    let connectTime = null;

    // Initialize: get current state from background
    chrome.runtime.sendMessage({ type: 'getStatus' }, (response) => {
        if (response) {
            updateUI(response);
        }
    });

    // Connect/Disconnect button
    connectBtn.addEventListener('click', () => {
        if (isConnected) {
            chrome.runtime.sendMessage({ type: 'disconnect' }, (response) => {
                updateUI({ connected: false });
            });
        } else {
            const server = serverSelect.value;
            chrome.runtime.sendMessage({ type: 'connect', server }, (response) => {
                if (response && response.success) {
                    updateUI({ connected: true, server });
                }
            });
        }
    });

    // Settings toggle
    settingsBtn.addEventListener('click', () => {
        settingsPanel.style.display = settingsPanel.style.display === 'none' ? 'block' : 'none';
    });

    // Toggle switches
    document.querySelectorAll('.toggle').forEach(toggle => {
        toggle.addEventListener('click', () => {
            toggle.classList.toggle('active');
            const setting = toggle.id.replace('Toggle', '');
            const enabled = toggle.classList.contains('active');
            chrome.runtime.sendMessage({ type: 'setSetting', setting, enabled });
        });
    });

    // Server selection change
    serverSelect.addEventListener('change', () => {
        if (isConnected) {
            chrome.runtime.sendMessage({ type: 'reconnect', server: serverSelect.value });
        }
    });

    // Listen for state updates from background
    chrome.runtime.onMessage.addListener((message) => {
        if (message.type === 'stateUpdate') {
            updateUI(message);
        }
    });

    // Update UI based on state
    function updateUI(state) {
        isConnected = state.connected;

        if (isConnected) {
            statusCircle.className = 'status-circle status-connected';
            statusCircle.textContent = 'متصل';
            statusText.textContent = state.server || 'متصل';
            connectBtn.textContent = 'قطع اتصال';
            connectBtn.className = 'btn btn-disconnect';
            connectTime = state.connectTime || Date.now();
            startUptimeCounter();
        } else {
            statusCircle.className = 'status-circle status-disconnected';
            statusCircle.textContent = 'قطع';
            statusText.textContent = 'متصل نیست';
            connectBtn.textContent = 'اتصال';
            connectBtn.className = 'btn btn-connect';
            connectTime = null;
            uptimeEl.textContent = '--';
        }

        if (state.stats) {
            downloadEl.textContent = formatBytes(state.stats.download || 0);
            uploadEl.textContent = formatBytes(state.stats.upload || 0);
            latencyEl.textContent = state.stats.latency ? `${state.stats.latency}ms` : '--';
        }
    }

    // Uptime counter
    function startUptimeCounter() {
        setInterval(() => {
            if (connectTime) {
                const elapsed = Math.floor((Date.now() - connectTime) / 1000);
                const mins = Math.floor(elapsed / 60);
                const secs = elapsed % 60;
                uptimeEl.textContent = `${mins}:${secs.toString().padStart(2, '0')}`;
            }
        }, 1000);
    }

    // Format bytes to human-readable
    function formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }
});
