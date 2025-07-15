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
        this.tabButtons = document.querySelectorAll('.tab-button');
        this.tabContents = document.querySelectorAll('.tab-content');
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
        // Update tab buttons
        this.tabButtons.forEach(button => {
            if (button.dataset.tab === tabName) {
                button.classList.add('active');
            } else {
                button.classList.remove('active');
            }
        });

        // Update tab contents
        this.tabContents.forEach(content => {
            if (content.id === `${tabName}-tab`) {
                content.classList.add('active');
            } else {
                content.classList.remove('active');
            }
        });

        // Update URL to persist tab state only when user interacts
        if (updateURL) {
            this.updateParameter('view', tabName);
        }
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new PlaygroundController();
});
