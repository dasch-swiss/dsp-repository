class PlaygroundController {
    constructor() {
        this.initializeControls();
        this.bindEvents();
        this.syncFromURL();
    }

    initializeControls() {
        this.variantSelect = document.getElementById('variant-select');
        this.themeSelect = document.getElementById('theme-select');
        this.iframe = document.getElementById('component-iframe');
        this.tabButtons = document.querySelectorAll('[data-tab-button]');
        this.tabContents = document.querySelectorAll('[data-tab-content]');
    }

    bindEvents() {
        // Parameter control events
        if (this.variantSelect) {
            this.variantSelect.addEventListener('change', (e) => {
                this.updateParameter('variant', e.target.value);
            });
        }

        if (this.themeSelect) {
            this.themeSelect.addEventListener('change', (e) => {
                this.updateParameter('theme', e.target.value);
            });
        }

        // Tab switching events
        this.tabButtons.forEach(button => {
            button.addEventListener('click', (e) => {
                const tabName = e.target.dataset.tab;
                this.switchTab(tabName, true);
            });
        });

        // Handle browser back/forward
        window.addEventListener('popstate', () => {
            this.syncFromURL();
        });
    }

    syncFromURL() {
        const urlParams = new URLSearchParams(window.location.search);
        const variant = urlParams.get('variant');
        const theme = urlParams.get('theme');
        const view = urlParams.get('view') || 'component';

        if (variant && this.variantSelect) {
            this.variantSelect.value = variant;
        }

        if (theme && this.themeSelect) {
            this.themeSelect.value = theme;
            // Update theme on the main HTML element
            document.documentElement.classList.toggle('dark', theme === 'dark');
        }

        // Sync tab state with URL (without triggering URL update)
        this.switchTab(view, false);

        this.updateIframe();
    }

    updateParameter(paramName, value) {
        const url = new URL(window.location);
        url.searchParams.set(paramName, value);
        window.history.pushState({}, '', url);
        
        // Update theme on the main HTML element
        if (paramName === 'theme') {
            document.documentElement.classList.toggle('dark', value === 'dark');
        }
        
        this.updateIframe();
    }

    updateIframe() {
        if (!this.iframe) return;

        const urlParams = new URLSearchParams(window.location.search);
        const iframeUrl = new URL('/iframe', window.location.origin);

        // Copy all parameters to iframe URL
        for (const [key, value] of urlParams) {
            iframeUrl.searchParams.set(key, value);
        }

        this.iframe.src = iframeUrl.toString();
    }

    switchTab(tabName, updateURL = true) {
        // Update tab button styling with semantic classes
        this.tabButtons.forEach(button => {
            if (button.dataset.tab === tabName) {
                button.classList.remove('tab-button-inactive');
                button.classList.add('tab-button-active');
            } else {
                button.classList.remove('tab-button-active');
                button.classList.add('tab-button-inactive');
            }
        });

        // Update tab contents - show the selected tab, hide others
        this.tabContents.forEach(content => {
            if (content.dataset.panel === tabName) {
                content.classList.remove('hidden');
                content.classList.add('flex', 'flex-col');
            } else {
                content.classList.add('hidden');
                content.classList.remove('flex', 'flex-col');
            }
        });

        // Update URL to persist tab state only when user interacts
        if (updateURL) {
            this.updateParameter('view', tabName);
        }
    }

    // Get current iframe URL with all parameters
    getCurrentIframeUrl() {
        const urlParams = new URLSearchParams(window.location.search);
        const iframeUrl = new URL('/iframe', window.location.origin);

        // Copy all parameters to iframe URL
        for (const [key, value] of urlParams) {
            iframeUrl.searchParams.set(key, value);
        }

        return iframeUrl.toString();
    }
}

// Initialize when DOM is ready, handling both cases where DOM is already loaded and not
function initializePlayground() {
    window.playgroundController = new PlaygroundController();
}

// Global function to open component in new tab with current parameters
function openComponentInNewTab() {
    if (window.playgroundController) {
        const iframeUrl = window.playgroundController.getCurrentIframeUrl();
        window.open(iframeUrl, '_blank');
    }
}

// Initialize immediately if DOM is already loaded, otherwise wait for DOMContentLoaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initializePlayground);
} else {
    initializePlayground();
}
