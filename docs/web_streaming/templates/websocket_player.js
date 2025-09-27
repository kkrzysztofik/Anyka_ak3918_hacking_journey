/**
 * WebSocket Video Player Template
 *
 * This template provides a complete WebSocket-based H.264 video player
 * with PTZ control integration for browser streaming.
 *
 * Usage:
 * const player = new WebSocketVideoPlayer('ws://camera-ip:8081/ws/stream', canvasElement);
 * player.connect();
 */

class WebSocketVideoPlayer {
    constructor(wsUrl, canvasElement) {
        this.wsUrl = wsUrl;
        this.canvas = canvasElement;
        this.ctx = this.canvas.getContext('2d');
        this.ws = null;
        this.decoder = null;
        this.isConnected = false;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.frameCount = 0;
        this.startTime = Date.now();

        // Performance monitoring
        this.performanceStats = {
            framesReceived: 0,
            bytesReceived: 0,
            averageFPS: 0,
            lastFrameTime: 0
        };

        this.initH264Decoder();
        this.setupPerformanceMonitoring();
    }

    /**
     * Initialize H.264 decoder (requires Broadway.js or similar)
     */
    initH264Decoder() {
        // Using Broadway H.264 decoder as example
        // Replace with your preferred H.264 decoder library
        this.decoder = new H264Decoder();

        this.decoder.onPictureDecoded = (buffer, width, height) => {
            this.displayFrame(buffer, width, height);
            this.updatePerformanceStats();
        };

        this.decoder.onDecoderReady = () => {
            console.log('H.264 decoder ready');
        };

        this.decoder.onError = (error) => {
            console.error('H.264 decoder error:', error);
        };
    }

    /**
     * Connect to WebSocket server
     */
    connect() {
        try {
            this.ws = new WebSocket(this.wsUrl);
            this.ws.binaryType = 'arraybuffer';

            this.ws.onopen = (event) => {
                console.log('WebSocket connected to:', this.wsUrl);
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.onConnectionStatusChanged(true);
                this.startTime = Date.now();
            };

            this.ws.onmessage = (event) => {
                if (event.data instanceof ArrayBuffer) {
                    this.handleVideoFrame(event.data);
                } else {
                    this.handleControlMessage(JSON.parse(event.data));
                }
            };

            this.ws.onclose = (event) => {
                console.log(`WebSocket disconnected: ${event.code} - ${event.reason}`);
                this.isConnected = false;
                this.onConnectionStatusChanged(false);

                if (event.code !== 1000) { // Not a normal closure
                    this.attemptReconnect();
                }
            };

            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
            };

        } catch (error) {
            console.error('Failed to create WebSocket:', error);
            this.attemptReconnect();
        }
    }

    /**
     * Handle incoming video frame data
     */
    handleVideoFrame(frameData) {
        const uint8Array = new Uint8Array(frameData);
        this.performanceStats.bytesReceived += uint8Array.length;

        try {
            // Decode H.264 frame
            this.decoder.decode(uint8Array);
            this.performanceStats.framesReceived++;
            this.performanceStats.lastFrameTime = Date.now();
        } catch (error) {
            console.error('H.264 decode error:', error);
        }
    }

    /**
     * Display decoded frame on canvas
     */
    displayFrame(yuvBuffer, width, height) {
        // Resize canvas if necessary
        if (this.canvas.width !== width || this.canvas.height !== height) {
            this.canvas.width = width;
            this.canvas.height = height;
            this.onVideoResolutionChanged(width, height);
        }

        // Convert YUV to RGB and display
        const imageData = this.ctx.createImageData(width, height);
        this.convertYUVToRGB(yuvBuffer, imageData.data, width, height);
        this.ctx.putImageData(imageData, 0, 0);

        this.frameCount++;
    }

    /**
     * Convert YUV420P to RGB
     */
    convertYUVToRGB(yuvBuffer, rgbBuffer, width, height) {
        const ySize = width * height;
        const uvSize = ySize / 4;

        const yPlane = new Uint8Array(yuvBuffer, 0, ySize);
        const uPlane = new Uint8Array(yuvBuffer, ySize, uvSize);
        const vPlane = new Uint8Array(yuvBuffer, ySize + uvSize, uvSize);

        let rgbIndex = 0;

        for (let y = 0; y < height; y++) {
            for (let x = 0; x < width; x++) {
                const yIndex = y * width + x;
                const uvIndex = Math.floor(y / 2) * Math.floor(width / 2) + Math.floor(x / 2);

                const Y = yPlane[yIndex];
                const U = uPlane[uvIndex] - 128;
                const V = vPlane[uvIndex] - 128;

                // YUV to RGB conversion
                let R = Y + 1.402 * V;
                let G = Y - 0.344136 * U - 0.714136 * V;
                let B = Y + 1.772 * U;

                // Clamp values
                R = Math.max(0, Math.min(255, Math.round(R)));
                G = Math.max(0, Math.min(255, Math.round(G)));
                B = Math.max(0, Math.min(255, Math.round(B)));

                rgbBuffer[rgbIndex++] = R;
                rgbBuffer[rgbIndex++] = G;
                rgbBuffer[rgbIndex++] = B;
                rgbBuffer[rgbIndex++] = 255; // Alpha
            }
        }
    }

    /**
     * Handle control messages from server
     */
    handleControlMessage(message) {
        switch (message.type) {
            case 'ptz_response':
                this.onPTZResponse(message.command, message.result);
                break;
            case 'config_response':
                this.onConfigResponse(message.config, message.result);
                break;
            case 'error':
                console.error('Server error:', message.error);
                break;
            case 'ping':
                this.sendPong();
                break;
            default:
                console.warn('Unknown message type:', message.type);
        }
    }

    /**
     * Send PTZ command to server
     */
    sendPTZCommand(command, params = {}) {
        if (this.isConnected) {
            const message = {
                type: 'ptz',
                command: command,
                params: params,
                timestamp: Date.now()
            };

            this.ws.send(JSON.stringify(message));
        } else {
            console.warn('Cannot send PTZ command - not connected');
        }
    }

    /**
     * Send configuration command to server
     */
    sendConfigCommand(config) {
        if (this.isConnected) {
            const message = {
                type: 'config',
                config: config,
                timestamp: Date.now()
            };

            this.ws.send(JSON.stringify(message));
        } else {
            console.warn('Cannot send config command - not connected');
        }
    }

    /**
     * Send pong response to ping
     */
    sendPong() {
        if (this.isConnected) {
            const message = {
                type: 'pong',
                timestamp: Date.now()
            };

            this.ws.send(JSON.stringify(message));
        }
    }

    /**
     * Disconnect from WebSocket server
     */
    disconnect() {
        if (this.ws) {
            this.ws.close(1000, 'User disconnect');
            this.ws = null;
        }
        this.isConnected = false;
    }

    /**
     * Attempt to reconnect with exponential backoff
     */
    attemptReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);

            console.log(`Attempting reconnect ${this.reconnectAttempts}/${this.maxReconnectAttempts} in ${delay}ms`);

            setTimeout(() => {
                if (!this.isConnected) {
                    this.connect();
                }
            }, delay);
        } else {
            console.error('Max reconnection attempts reached');
            this.onConnectionFailed();
        }
    }

    /**
     * Set up performance monitoring
     */
    setupPerformanceMonitoring() {
        setInterval(() => {
            this.calculatePerformanceStats();
        }, 5000); // Update every 5 seconds
    }

    /**
     * Calculate and update performance statistics
     */
    calculatePerformanceStats() {
        const elapsed = (Date.now() - this.startTime) / 1000;
        if (elapsed > 0) {
            this.performanceStats.averageFPS = this.performanceStats.framesReceived / elapsed;
        }

        this.onPerformanceUpdate(this.performanceStats);
    }

    /**
     * Update performance stats when frame is displayed
     */
    updatePerformanceStats() {
        // This can be extended to include frame processing time, etc.
    }

    /**
     * Get current performance statistics
     */
    getPerformanceStats() {
        return { ...this.performanceStats };
    }

    // Event handlers - override these in your implementation

    /**
     * Called when connection status changes
     */
    onConnectionStatusChanged(connected) {
        console.log(`Connection status: ${connected ? 'Connected' : 'Disconnected'}`);
    }

    /**
     * Called when connection permanently fails
     */
    onConnectionFailed() {
        console.error('WebSocket connection permanently failed');
    }

    /**
     * Called when video resolution changes
     */
    onVideoResolutionChanged(width, height) {
        console.log(`Video resolution: ${width}x${height}`);
    }

    /**
     * Called when PTZ response is received
     */
    onPTZResponse(command, result) {
        console.log(`PTZ ${command} result:`, result);
    }

    /**
     * Called when config response is received
     */
    onConfigResponse(config, result) {
        console.log(`Config result:`, result);
    }

    /**
     * Called when performance stats are updated
     */
    onPerformanceUpdate(stats) {
        console.log(`Performance: ${stats.averageFPS.toFixed(1)} FPS, ${(stats.bytesReceived / 1024 / 1024).toFixed(1)} MB received`);
    }
}

/**
 * PTZ Controller for WebSocket Video Player
 */
class WebSocketPTZController {
    constructor(videoPlayer) {
        this.player = videoPlayer;
        this.isMoving = false;
        this.currentCommand = null;
        this.moveStartTime = null;

        this.setupEventListeners();
    }

    /**
     * Set up keyboard and mouse event listeners
     */
    setupEventListeners() {
        // Prevent browser default behavior for arrow keys
        document.addEventListener('keydown', (event) => {
            if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home'].includes(event.key)) {
                event.preventDefault();
            }
            this.handleKeyDown(event);
        });

        document.addEventListener('keyup', (event) => {
            this.handleKeyUp(event);
        });

        // Stop movement when mouse is released anywhere
        document.addEventListener('mouseup', () => {
            if (this.isMoving) {
                this.stopPTZ();
            }
        });
    }

    /**
     * Start PTZ movement in specified direction
     */
    startPTZMove(direction, speed = 0.5) {
        if (this.isMoving && this.currentCommand === direction) {
            return; // Already moving in this direction
        }

        this.stopPTZ(); // Stop any current movement

        this.player.sendPTZCommand('move_' + direction, { speed: speed });
        this.isMoving = true;
        this.currentCommand = direction;
        this.moveStartTime = Date.now();

        console.log(`Started PTZ move: ${direction} at speed ${speed}`);
    }

    /**
     * Stop PTZ movement
     */
    stopPTZ() {
        if (this.isMoving) {
            this.player.sendPTZCommand('stop');

            const duration = this.moveStartTime ? Date.now() - this.moveStartTime : 0;
            console.log(`Stopped PTZ move: ${this.currentCommand} (${duration}ms)`);

            this.isMoving = false;
            this.currentCommand = null;
            this.moveStartTime = null;
        }
    }

    /**
     * Go to home position
     */
    goHome() {
        this.stopPTZ();
        this.player.sendPTZCommand('home');
        console.log('PTZ going to home position');
    }

    /**
     * Go to preset position
     */
    goToPreset(presetNumber) {
        this.stopPTZ();
        this.player.sendPTZCommand('goto_preset', { preset: presetNumber });
        console.log(`PTZ going to preset ${presetNumber}`);
    }

    /**
     * Set preset at current position
     */
    setPreset(presetNumber) {
        this.player.sendPTZCommand('set_preset', { preset: presetNumber });
        console.log(`Setting PTZ preset ${presetNumber}`);
    }

    /**
     * Zoom in/out
     */
    zoom(direction, speed = 0.5) {
        const zoomSpeed = direction === 'in' ? speed : -speed;
        this.player.sendPTZCommand('zoom', { speed: zoomSpeed });
        console.log(`Zoom ${direction} at speed ${speed}`);
    }

    /**
     * Handle keyboard input
     */
    handleKeyDown(event) {
        switch(event.key) {
            case 'ArrowUp':
                this.startPTZMove('up');
                break;
            case 'ArrowDown':
                this.startPTZMove('down');
                break;
            case 'ArrowLeft':
                this.startPTZMove('left');
                break;
            case 'ArrowRight':
                this.startPTZMove('right');
                break;
            case 'Home':
                this.goHome();
                break;
            case '+':
            case '=':
                this.zoom('in');
                break;
            case '-':
                this.zoom('out');
                break;
            // Preset keys (1-9)
            case '1': case '2': case '3': case '4': case '5':
            case '6': case '7': case '8': case '9':
                if (event.ctrlKey) {
                    this.setPreset(parseInt(event.key));
                } else {
                    this.goToPreset(parseInt(event.key));
                }
                break;
        }
    }

    /**
     * Handle keyboard key release
     */
    handleKeyUp(event) {
        if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) {
            this.stopPTZ();
        }
    }

    /**
     * Create PTZ control UI buttons
     */
    createControlButtons(container) {
        const buttons = [
            { id: 'ptz-up', text: '↑', action: () => this.startPTZMove('up') },
            { id: 'ptz-down', text: '↓', action: () => this.startPTZMove('down') },
            { id: 'ptz-left', text: '←', action: () => this.startPTZMove('left') },
            { id: 'ptz-right', text: '→', action: () => this.startPTZMove('right') },
            { id: 'ptz-home', text: 'Home', action: () => this.goHome() },
            { id: 'zoom-in', text: 'Zoom+', action: () => this.zoom('in') },
            { id: 'zoom-out', text: 'Zoom-', action: () => this.zoom('out') }
        ];

        buttons.forEach(button => {
            const btn = document.createElement('button');
            btn.id = button.id;
            btn.textContent = button.text;
            btn.className = 'ptz-button';

            btn.addEventListener('mousedown', button.action);
            btn.addEventListener('mouseup', () => {
                if (button.id.includes('ptz-') && button.id !== 'ptz-home') {
                    this.stopPTZ();
                }
            });

            container.appendChild(btn);
        });
    }
}

// Usage example:
/*
const canvas = document.getElementById('videoCanvas');
const player = new WebSocketVideoPlayer('ws://192.168.1.100:8081/ws/stream', canvas);
const ptzController = new WebSocketPTZController(player);

// Override event handlers as needed
player.onConnectionStatusChanged = (connected) => {
    document.getElementById('status').textContent = connected ? 'Connected' : 'Disconnected';
};

player.onPerformanceUpdate = (stats) => {
    document.getElementById('fps').textContent = stats.averageFPS.toFixed(1);
    document.getElementById('bandwidth').textContent = (stats.bytesReceived / 1024 / 1024).toFixed(1) + ' MB';
};

// Connect to camera
player.connect();

// Create PTZ control buttons
const controlContainer = document.getElementById('ptz-controls');
ptzController.createControlButtons(controlContainer);
*/