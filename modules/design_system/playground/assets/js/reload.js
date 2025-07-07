class ReloadController {
    constructor() {
        this.connect();
    }

    connect() {
        const socket = new WebSocket('ws://localhost:3400/reload-ws');

        socket.onopen = () => {
            console.log('WebSocket connected');
        };

        socket.onmessage = (event) => {
            if (event.data === 'reload') {
                console.log('Reload message received, reloading...');
                location.reload();
            }
        };

        socket.onclose = () => {
            console.warn('WebSocket disconnected, retrying...');
            setTimeout(() => this.connect(), 1000);
        };

        socket.onerror = (err) => {
            socket.close();
        };
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new ReloadController();
});