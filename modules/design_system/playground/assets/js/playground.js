class PlaygroundController {
    constructor() {
        this.initializeControls();
        this.bindEvents();
        this.syncFromURL();
    }

    initializeControls() {
        this.variantSelect = document.getElementById('variant-select');
        this.themeSelect = document.getElementById('theme-select');
        this.componentStoreIframe = document.getElementById('component-store-iframe');
        this.examplesIframe = document.getElementById('examples-iframe');
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

        // Intercept sidebar component links to preserve theme
        document.querySelectorAll('nav a[href^="/?component="]').forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const href = new URL(link.href);
                const currentTheme = this.themeSelect ? this.themeSelect.value : 'light';
                href.searchParams.set('theme', currentTheme);
                window.location.href = href.toString();
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
        const theme = urlParams.get('theme') || 'light';
        const view = urlParams.get('view') || 'component-store';

        if (variant && this.variantSelect) {
            this.variantSelect.value = variant;
        }

        if (this.themeSelect) {
            this.themeSelect.value = theme;
        }

        // Sync tab state with URL (without triggering URL update)
        this.switchTab(view, false);

        this.updateIframes();
    }

    updateParameter(paramName, value) {
        const url = new URL(window.location);
        url.searchParams.set(paramName, value);
        window.history.pushState({}, '', url);

        this.updateIframes();
    }

    updateIframes() {
        const urlParams = new URLSearchParams(window.location.search);
        const component = urlParams.get('component');
        const variant = urlParams.get('variant');
        const theme = urlParams.get('theme') || 'light';

        // Update component-store iframe
        if (this.componentStoreIframe) {
            const componentStoreUrl = new URL('/iframe', window.location.origin);
            componentStoreUrl.searchParams.set('component', component);
            if (variant) componentStoreUrl.searchParams.set('variant', variant);
            componentStoreUrl.searchParams.set('theme', theme);
            componentStoreUrl.searchParams.set('view', 'component-store');
            this.componentStoreIframe.src = componentStoreUrl.toString();
        }

        // Update examples iframe
        if (this.examplesIframe) {
            const examplesUrl = new URL('/iframe', window.location.origin);
            examplesUrl.searchParams.set('component', component);
            examplesUrl.searchParams.set('variant', 'default');
            examplesUrl.searchParams.set('theme', theme);
            examplesUrl.searchParams.set('view', 'examples');
            this.examplesIframe.src = examplesUrl.toString();
        }
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
        const view = urlParams.get('view') || 'component-store';
        const component = urlParams.get('component');
        const variant = urlParams.get('variant');
        const theme = urlParams.get('theme') || 'light';

        const iframeUrl = new URL('/iframe', window.location.origin);
        iframeUrl.searchParams.set('component', component);
        if (variant) iframeUrl.searchParams.set('variant', variant);
        iframeUrl.searchParams.set('theme', theme);
        iframeUrl.searchParams.set('view', view);

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
