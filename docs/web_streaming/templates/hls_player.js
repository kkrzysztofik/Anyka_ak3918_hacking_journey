/**
 * HLS Video Player Template
 *
 * This template provides HLS (HTTP Live Streaming) video playback
 * with adaptive bitrate support using HLS.js for cross-browser compatibility.
 *
 * Usage:
 * const player = new HLSVideoPlayer(videoElement, 'http://camera-ip:8080/hls/playlist.m3u8');
 * player.init();
 */

class HLSVideoPlayer {
    constructor(videoElement, playlistUrl) {
        this.video = videoElement;
        this.playlistUrl = playlistUrl;
        this.hls = null;
        this.isSupported = this.checkSupport();
        this.isPlaying = false;
        this.retryCount = 0;
        this.maxRetries = 5;

        // Performance monitoring
        this.stats = {
            bytesLoaded: 0,
            segmentsLoaded: 0,
            currentBitrate: 0,
            droppedFrames: 0,
            bufferLength: 0
        };

        this.setupEventListeners();
    }

    /**
     * Check HLS support in current browser
     */
    checkSupport() {
        // Check for native HLS support (Safari)
        const nativeSupport = this.video.canPlayType('application/vnd.apple.mpegurl');

        // Check for HLS.js support (other browsers)
        const hlsJsSupport = typeof Hls !== 'undefined' && Hls.isSupported();

        return {
            native: nativeSupport !== '',
            hlsJs: hlsJsSupport,
            supported: nativeSupport !== '' || hlsJsSupport
        };
    }

    /**
     * Initialize HLS player
     */
    init() {
        if (!this.isSupported.supported) {
            this.onError('HLS not supported in this browser');
            return false;
        }

        if (this.isSupported.native) {
            console.log('Using native HLS support');
            this.initNativeHLS();
        } else if (this.isSupported.hlsJs) {
            console.log('Using HLS.js');
            this.initHLSJs();
        }

        return true;
    }

    /**
     * Initialize native HLS support (Safari)
     */
    initNativeHLS() {
        this.video.src = this.playlistUrl;

        this.video.addEventListener('loadstart', () => {
            console.log('Native HLS: Loading started');
            this.onLoadingStart();
        });

        this.video.addEventListener('canplay', () => {
            console.log('Native HLS: Can start playing');
            this.onCanPlay();
        });

        this.video.addEventListener('playing', () => {
            console.log('Native HLS: Playing');
            this.isPlaying = true;
            this.onPlayingStart();
        });

        this.video.addEventListener('pause', () => {
            console.log('Native HLS: Paused');
            this.isPlaying = false;
            this.onPlayingStop();
        });

        this.video.addEventListener('error', (event) => {
            console.error('Native HLS error:', event);
            this.handleError('Native HLS playback error');
        });

        // Monitor buffer and quality
        this.startNativeMonitoring();
    }

    /**
     * Initialize HLS.js for non-Safari browsers
     */
    initHLSJs() {
        this.hls = new Hls({
            // HLS.js configuration
            debug: false,
            enableWorker: true,
            lowLatencyMode: true,
            maxBufferLength: 30,        // 30 seconds buffer
            maxMaxBufferLength: 60,     // 60 seconds max buffer
            maxBufferSize: 60 * 1000 * 1000,  // 60MB max buffer size
            maxBufferHole: 0.5,         // 0.5 second buffer hole tolerance
            startLevel: -1,             // Auto start level
            capLevelToPlayerSize: true, // Cap quality to player size
            testBandwidth: true,        // Test bandwidth for quality selection

            // Retry configuration
            manifestLoadingMaxRetry: 3,
            manifestLoadingRetryDelay: 1000,
            levelLoadingMaxRetry: 3,
            levelLoadingRetryDelay: 1000,
            fragLoadingMaxRetry: 3,
            fragLoadingRetryDelay: 1000
        });

        // Load source and attach to video element
        this.hls.loadSource(this.playlistUrl);
        this.hls.attachMedia(this.video);

        // Set up HLS.js event handlers
        this.setupHLSJsEventHandlers();
    }

    /**
     * Set up HLS.js event handlers
     */
    setupHLSJsEventHandlers() {
        // Manifest loaded successfully
        this.hls.on(Hls.Events.MANIFEST_PARSED, (event, data) => {
            console.log('HLS manifest parsed:', data);
            console.log(`Found ${data.levels.length} quality levels`);

            this.onManifestParsed(data);

            // Auto-play if configured
            if (this.video.autoplay) {
                this.play();
            }
        });

        // Level switching
        this.hls.on(Hls.Events.LEVEL_SWITCHED, (event, data) => {
            const level = this.hls.levels[data.level];
            console.log(`Quality switched to: ${level.height}p, ${Math.round(level.bitrate / 1000)}kbps`);

            this.stats.currentBitrate = level.bitrate;
            this.onQualityChanged(level);
        });

        // Fragment loaded
        this.hls.on(Hls.Events.FRAG_LOADED, (event, data) => {
            this.stats.bytesLoaded += data.frag.byteLength || 0;
            this.stats.segmentsLoaded++;

            this.onFragmentLoaded(data);
        });

        // Buffer events
        this.hls.on(Hls.Events.BUFFER_APPENDED, (event, data) => {
            this.updateBufferStats();
        });

        this.hls.on(Hls.Events.BUFFER_EOS, (event, data) => {
            console.log('Buffer end of stream');
        });

        // Error handling
        this.hls.on(Hls.Events.ERROR, (event, data) => {
            this.handleHLSError(data);
        });

        // Statistics
        this.hls.on(Hls.Events.FRAG_PARSING_DATA, (event, data) => {
            // Track dropped frames, etc.
            this.updatePerformanceStats(data);
        });
    }

    /**
     * Set up video element event listeners
     */
    setupEventListeners() {
        this.video.addEventListener('play', () => {
            this.isPlaying = true;
            this.onPlayingStart();
        });

        this.video.addEventListener('pause', () => {
            this.isPlaying = false;
            this.onPlayingStop();
        });

        this.video.addEventListener('timeupdate', () => {
            this.onTimeUpdate();
        });

        this.video.addEventListener('seeking', () => {
            this.onSeeking();
        });

        this.video.addEventListener('seeked', () => {
            this.onSeeked();
        });

        this.video.addEventListener('waiting', () => {
            this.onBuffering(true);
        });

        this.video.addEventListener('playing', () => {
            this.onBuffering(false);
        });
    }

    /**
     * Play video
     */
    play() {
        if (this.video.paused) {
            const playPromise = this.video.play();

            if (playPromise !== undefined) {
                playPromise
                    .then(() => {
                        console.log('HLS playback started');
                    })
                    .catch(error => {
                        console.error('HLS playback failed:', error);
                        this.onError('Playback failed: ' + error.message);
                    });
            }
        }
    }

    /**
     * Pause video
     */
    pause() {
        if (!this.video.paused) {
            this.video.pause();
        }
    }

    /**
     * Seek to specific time
     */
    seek(time) {
        if (this.video.duration && time >= 0 && time <= this.video.duration) {
            this.video.currentTime = time;
        }
    }

    /**
     * Set volume (0.0 to 1.0)
     */
    setVolume(volume) {
        this.video.volume = Math.max(0, Math.min(1, volume));
    }

    /**
     * Set quality level
     */
    setQuality(levelIndex) {
        if (this.hls) {
            if (levelIndex === -1) {
                // Auto quality
                this.hls.currentLevel = -1;
                console.log('Set quality to auto');
            } else if (levelIndex >= 0 && levelIndex < this.hls.levels.length) {
                this.hls.currentLevel = levelIndex;
                const level = this.hls.levels[levelIndex];
                console.log(`Set quality to: ${level.height}p, ${Math.round(level.bitrate / 1000)}kbps`);
            }
        }
    }

    /**
     * Get available quality levels
     */
    getQualityLevels() {
        if (this.hls && this.hls.levels) {
            return this.hls.levels.map((level, index) => ({
                index: index,
                width: level.width,
                height: level.height,
                bitrate: level.bitrate,
                name: `${level.height}p (${Math.round(level.bitrate / 1000)}kbps)`
            }));
        }
        return [];
    }

    /**
     * Get current statistics
     */
    getStats() {
        const currentStats = { ...this.stats };

        if (this.hls) {
            const hlsStats = this.hls.stats;
            currentStats.totalBytesLoaded = hlsStats.total;
            currentStats.bufferLength = this.getBufferLength();
            currentStats.droppedFrames = hlsStats.dropped || 0;
        }

        return currentStats;
    }

    /**
     * Get current buffer length in seconds
     */
    getBufferLength() {
        if (this.video.buffered.length > 0) {
            const currentTime = this.video.currentTime;
            for (let i = 0; i < this.video.buffered.length; i++) {
                const start = this.video.buffered.start(i);
                const end = this.video.buffered.end(i);

                if (currentTime >= start && currentTime <= end) {
                    return end - currentTime;
                }
            }
        }
        return 0;
    }

    /**
     * Handle HLS.js errors
     */
    handleHLSError(data) {
        console.error('HLS.js error:', data);

        if (data.fatal) {
            switch (data.type) {
                case Hls.ErrorTypes.NETWORK_ERROR:
                    console.error('Fatal network error');
                    this.recoverNetworkError();
                    break;
                case Hls.ErrorTypes.MEDIA_ERROR:
                    console.error('Fatal media error');
                    this.recoverMediaError();
                    break;
                default:
                    console.error('Fatal error, cannot recover');
                    this.destroy();
                    this.onError('Fatal HLS error: ' + data.details);
                    break;
            }
        } else {
            console.warn('Non-fatal HLS error:', data.details);
        }
    }

    /**
     * Recover from network error
     */
    recoverNetworkError() {
        if (this.retryCount < this.maxRetries) {
            console.log(`Attempting network error recovery (${this.retryCount + 1}/${this.maxRetries})`);
            this.retryCount++;

            setTimeout(() => {
                if (this.hls) {
                    this.hls.startLoad();
                }
            }, 1000 * this.retryCount); // Exponential backoff
        } else {
            this.onError('Network error recovery failed');
        }
    }

    /**
     * Recover from media error
     */
    recoverMediaError() {
        if (this.retryCount < this.maxRetries) {
            console.log(`Attempting media error recovery (${this.retryCount + 1}/${this.maxRetries})`);
            this.retryCount++;

            if (this.hls) {
                this.hls.recoverMediaError();
            }
        } else {
            this.onError('Media error recovery failed');
        }
    }

    /**
     * Handle general errors
     */
    handleError(message) {
        console.error('HLS Player error:', message);
        this.onError(message);
    }

    /**
     * Start monitoring for native HLS
     */
    startNativeMonitoring() {
        setInterval(() => {
            this.updateBufferStats();
            this.onStatsUpdate(this.getStats());
        }, 1000);
    }

    /**
     * Update buffer statistics
     */
    updateBufferStats() {
        this.stats.bufferLength = this.getBufferLength();
    }

    /**
     * Update performance statistics
     */
    updatePerformanceStats(data) {
        // Extend this based on available data
        if (data && data.stats) {
            // Update dropped frames, etc.
        }
    }

    /**
     * Destroy player and clean up resources
     */
    destroy() {
        if (this.hls) {
            this.hls.destroy();
            this.hls = null;
        }

        if (this.video) {
            this.video.removeAttribute('src');
            this.video.load();
        }

        this.isPlaying = false;
        console.log('HLS player destroyed');
    }

    /**
     * Restart player (useful for error recovery)
     */
    restart() {
        console.log('Restarting HLS player');
        this.destroy();
        this.retryCount = 0;

        setTimeout(() => {
            this.init();
        }, 1000);
    }

    // Event handlers - override these in your implementation

    onLoadingStart() {
        console.log('Loading started');
    }

    onCanPlay() {
        console.log('Can play');
    }

    onPlayingStart() {
        console.log('Playing started');
    }

    onPlayingStop() {
        console.log('Playing stopped');
    }

    onManifestParsed(data) {
        console.log('Manifest parsed:', data);
    }

    onQualityChanged(level) {
        console.log('Quality changed:', level);
    }

    onFragmentLoaded(data) {
        // Fragment loaded
    }

    onTimeUpdate() {
        // Time updated
    }

    onSeeking() {
        console.log('Seeking');
    }

    onSeeked() {
        console.log('Seeked');
    }

    onBuffering(isBuffering) {
        console.log('Buffering:', isBuffering);
    }

    onStatsUpdate(stats) {
        // Stats updated
    }

    onError(message) {
        console.error('Player error:', message);
    }
}

/**
 * HLS Player UI Controller
 */
class HLSPlayerUI {
    constructor(player, container) {
        this.player = player;
        this.container = container;
        this.controls = {};

        this.createUI();
        this.setupEventListeners();
    }

    createUI() {
        this.container.innerHTML = `
            <div class="hls-player-controls">
                <button id="playPauseBtn" class="control-btn">Play</button>
                <div class="progress-container">
                    <div class="progress-bar">
                        <div id="progressFill" class="progress-fill"></div>
                        <div id="bufferFill" class="buffer-fill"></div>
                    </div>
                    <input type="range" id="progressSlider" class="progress-slider"
                           min="0" max="100" value="0">
                </div>
                <div class="time-display">
                    <span id="currentTime">0:00</span> / <span id="duration">0:00</span>
                </div>
                <div class="volume-container">
                    <button id="volumeBtn" class="control-btn">🔊</button>
                    <input type="range" id="volumeSlider" class="volume-slider"
                           min="0" max="100" value="100">
                </div>
                <select id="qualitySelect" class="quality-select">
                    <option value="-1">Auto</option>
                </select>
                <button id="fullscreenBtn" class="control-btn">Fullscreen</button>
            </div>
            <div class="stats-display" id="statsDisplay"></div>
        `;

        this.cacheElements();
    }

    cacheElements() {
        this.controls = {
            playPause: this.container.querySelector('#playPauseBtn'),
            progressSlider: this.container.querySelector('#progressSlider'),
            progressFill: this.container.querySelector('#progressFill'),
            bufferFill: this.container.querySelector('#bufferFill'),
            currentTime: this.container.querySelector('#currentTime'),
            duration: this.container.querySelector('#duration'),
            volumeBtn: this.container.querySelector('#volumeBtn'),
            volumeSlider: this.container.querySelector('#volumeSlider'),
            qualitySelect: this.container.querySelector('#qualitySelect'),
            fullscreenBtn: this.container.querySelector('#fullscreenBtn'),
            statsDisplay: this.container.querySelector('#statsDisplay')
        };
    }

    setupEventListeners() {
        // Play/Pause button
        this.controls.playPause.addEventListener('click', () => {
            if (this.player.isPlaying) {
                this.player.pause();
            } else {
                this.player.play();
            }
        });

        // Progress slider
        this.controls.progressSlider.addEventListener('input', (e) => {
            const time = (e.target.value / 100) * this.player.video.duration;
            this.player.seek(time);
        });

        // Volume controls
        this.controls.volumeSlider.addEventListener('input', (e) => {
            const volume = e.target.value / 100;
            this.player.setVolume(volume);
        });

        this.controls.volumeBtn.addEventListener('click', () => {
            this.player.video.muted = !this.player.video.muted;
        });

        // Quality selection
        this.controls.qualitySelect.addEventListener('change', (e) => {
            const levelIndex = parseInt(e.target.value);
            this.player.setQuality(levelIndex);
        });

        // Fullscreen
        this.controls.fullscreenBtn.addEventListener('click', () => {
            this.toggleFullscreen();
        });

        // Player event overrides
        this.player.onPlayingStart = () => {
            this.controls.playPause.textContent = 'Pause';
        };

        this.player.onPlayingStop = () => {
            this.controls.playPause.textContent = 'Play';
        };

        this.player.onManifestParsed = (data) => {
            this.populateQualitySelector();
        };

        this.player.onTimeUpdate = () => {
            this.updateProgress();
        };

        this.player.onStatsUpdate = (stats) => {
            this.updateStatsDisplay(stats);
        };
    }

    populateQualitySelector() {
        const levels = this.player.getQualityLevels();
        const select = this.controls.qualitySelect;

        // Clear existing options except auto
        while (select.children.length > 1) {
            select.removeChild(select.lastChild);
        }

        // Add quality options
        levels.forEach(level => {
            const option = document.createElement('option');
            option.value = level.index;
            option.textContent = level.name;
            select.appendChild(option);
        });
    }

    updateProgress() {
        const video = this.player.video;
        if (video.duration) {
            const progress = (video.currentTime / video.duration) * 100;
            this.controls.progressFill.style.width = progress + '%';
            this.controls.progressSlider.value = progress;

            // Update buffer display
            const bufferLength = this.player.getBufferLength();
            const bufferProgress = ((video.currentTime + bufferLength) / video.duration) * 100;
            this.controls.bufferFill.style.width = bufferProgress + '%';

            // Update time display
            this.controls.currentTime.textContent = this.formatTime(video.currentTime);
            this.controls.duration.textContent = this.formatTime(video.duration);
        }
    }

    updateStatsDisplay(stats) {
        const display = this.controls.statsDisplay;
        display.innerHTML = `
            Bitrate: ${Math.round(stats.currentBitrate / 1000)}kbps |
            Buffer: ${stats.bufferLength.toFixed(1)}s |
            Segments: ${stats.segmentsLoaded} |
            Data: ${(stats.bytesLoaded / 1024 / 1024).toFixed(1)}MB
        `;
    }

    formatTime(seconds) {
        const minutes = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${minutes}:${secs.toString().padStart(2, '0')}`;
    }

    toggleFullscreen() {
        if (document.fullscreenElement) {
            document.exitFullscreen();
        } else {
            this.player.video.requestFullscreen();
        }
    }
}

// Usage example:
/*
const video = document.getElementById('hlsVideo');
const controlsContainer = document.getElementById('hlsControls');

const player = new HLSVideoPlayer(video, 'http://192.168.1.100:8080/hls/playlist.m3u8');
const ui = new HLSPlayerUI(player, controlsContainer);

player.init();
*/