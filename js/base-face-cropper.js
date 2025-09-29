class BaseFaceCropper {
    constructor() {
        this.detector = null;
        this.aspectRatioLocked = false;
        this.currentAspectRatio = 1;
        this.isDarkMode = false;
        this.processingStartTime = null;

        this.statistics = {
            totalFacesDetected: 0,
            imagesProcessed: 0,
            successfulProcessing: 0,
            processingTimes: [],
            startTime: null
        };
    }

    convertBoundingBoxToPixels(bbox, imageWidth, imageHeight) {
        if (!bbox) {
            return null;
        }

        const originX = Number.isFinite(bbox.originX) ? bbox.originX : 0;
        const originY = Number.isFinite(bbox.originY) ? bbox.originY : 0;
        const boxWidth = Number.isFinite(bbox.width) ? bbox.width : 0;
        const boxHeight = Number.isFinite(bbox.height) ? bbox.height : 0;

        const isNormalized = originX >= 0 && originX <= 1 &&
            originY >= 0 && originY <= 1 &&
            boxWidth > 0 && boxWidth <= 1 &&
            boxHeight > 0 && boxHeight <= 1;

        const rawX = isNormalized ? originX * imageWidth : originX;
        const rawY = isNormalized ? originY * imageHeight : originY;
        const rawWidth = isNormalized ? boxWidth * imageWidth : boxWidth;
        const rawHeight = isNormalized ? boxHeight * imageHeight : boxHeight;

        if (!Number.isFinite(rawX) || !Number.isFinite(rawY) ||
            !Number.isFinite(rawWidth) || !Number.isFinite(rawHeight)) {
            return null;
        }

        const clampedX = Math.min(Math.max(rawX, 0), imageWidth);
        const clampedY = Math.min(Math.max(rawY, 0), imageHeight);
        const maxWidth = imageWidth - clampedX;
        const maxHeight = imageHeight - clampedY;

        const width = Math.min(Math.max(rawWidth, 1), maxWidth);
        const height = Math.min(Math.max(rawHeight, 1), maxHeight);

        if (width <= 0 || height <= 0) {
            return null;
        }

        return {
            x: clampedX,
            y: clampedY,
            width,
            height
        };
    }

    async waitForMediaPipe() {
        const maxWaitTime = 10000;
        const checkInterval = 100;
        let elapsed = 0;

        while (elapsed < maxWaitTime) {
            if (typeof window.vision !== 'undefined' &&
                window.vision.FilesetResolver &&
                window.vision.FaceDetector) {
                return;
            }
            await new Promise(resolve => setTimeout(resolve, checkInterval));
            elapsed += checkInterval;
        }

        throw new Error('MediaPipe Tasks Vision library failed to load within timeout');
    }

    async loadModel() {
        try {
            this.updateStatus('Loading face detection model...');

            await this.waitForMediaPipe();

            if (!window.vision || !window.vision.FilesetResolver || !window.vision.FaceDetector) {
                throw new Error('MediaPipe Tasks Vision library not loaded');
            }

            const visionFileset = await window.vision.FilesetResolver.forVisionTasks(
                "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm"
            );

            this.detector = await window.vision.FaceDetector.createFromOptions(visionFileset, {
                baseOptions: {
                    modelAssetPath: `https://storage.googleapis.com/mediapipe-models/face_detector/blaze_face_short_range/float16/1/blaze_face_short_range.tflite`,
                    delegate: "GPU"
                },
                runningMode: "IMAGE"
            });

            this.updateStatus('Model loaded successfully. Ready to process images.');
            this.addToLog('Face detection model loaded successfully');
        } catch (error) {
            console.error('Error loading model:', error);
            this.updateStatus('Error loading model. Please refresh the page.');
            this.addToLog('Error loading face detection model: ' + error.message, 'error');
        }
    }

    async cropFace(image, face, settings) {
        if (!settings) {
            settings = this.getSettings();
        }

        const box = face.box;

        const faceHeight = settings.outputHeight * (settings.faceHeightPct / 100);
        const scale = faceHeight / box.height;

        let cropWidth = settings.outputWidth / scale;
        let cropHeight = settings.outputHeight / scale;

        let centerX = box.xMin + (box.width / 2);
        let centerY = box.yMin + (box.height / 2);

        if (settings.positioningMode === 'rule-of-thirds') {
            centerY = box.yMin + (box.height * 0.33);
        } else if (settings.positioningMode === 'custom') {
            centerX += (settings.horizontalOffset / 100) * cropWidth;
            centerY += (settings.verticalOffset / 100) * cropHeight;
        }

        let cropX = centerX - (cropWidth / 2);
        let cropY = centerY - (cropHeight / 2);

        cropX = Math.max(0, Math.min(cropX, image.width - cropWidth));
        cropY = Math.max(0, Math.min(cropY, image.height - cropHeight));

        const canvas = document.createElement('canvas');
        canvas.width = settings.outputWidth;
        canvas.height = settings.outputHeight;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });

        ctx.drawImage(
            image,
            cropX, cropY, cropWidth, cropHeight,
            0, 0, settings.outputWidth, settings.outputHeight
        );

        await this.applyEnhancements(ctx, canvas);

        return canvas;
    }

    async applyEnhancements(ctx, canvas) {
        if (this.autoColorCorrection?.checked) {
            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
            ctx.putImageData(imageData, 0, 0);
        }
    }

    getSettings() {
        return {
            outputWidth: parseInt(this.outputWidth.value),
            outputHeight: parseInt(this.outputHeight.value),
            faceHeightPct: parseInt(this.faceHeightPct.value),
            positioningMode: this.positioningMode.value,
            verticalOffset: parseInt(this.verticalOffset.value),
            horizontalOffset: parseInt(this.horizontalOffset.value),
            format: this.outputFormat.value,
            quality: this.outputFormat.value === 'jpeg' ? (parseInt(this.jpegQuality.value) / 100) : 1
        };
    }

    generateFilename(imageData, index, template) {
        const settings = this.getSettings();
        const filenameTemplate = template || this.namingTemplate?.value || 'face_{original}_{index}';
        const originalName = imageData.file ? imageData.file.name.replace(/\.[^/.]+$/, '') : 'image';
        const extension = settings.format === 'jpeg' ? 'jpg' : settings.format;

        return filenameTemplate
            .replace('{original}', originalName)
            .replace('{csv_name}', imageData.csvOutputName || originalName)
            .replace('{index}', index + 1)
            .replace('{timestamp}', Date.now())
            .replace('{width}', settings.outputWidth)
            .replace('{height}', settings.outputHeight) + '.' + extension;
    }

    updateStatus(message) {
        if (this.status) {
            this.status.textContent = message;
        }
        if (this.processingStatus) {
            this.processingStatus.textContent = message.split('.')[0];
        }
    }

    addToLog(message, type = 'info') {
        const logElement = this.processingLogElement || this.loadingLogElement;
        if (logElement && logElement.appendChild) {
            const logEntry = document.createElement('div');
            logEntry.className = `log-entry ${type}`;
            logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
            logElement.appendChild(logEntry);
            logElement.scrollTop = logElement.scrollHeight;
        }
    }

    addToLoadingLog(message) {
        if (this.loadingLogElement && this.loadingLogElement.appendChild) {
            const logEntry = document.createElement('div');
            logEntry.className = 'log-entry loading';
            logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
            this.loadingLogElement.appendChild(logEntry);
            this.loadingLogElement.scrollTop = this.loadingLogElement.scrollHeight;
        }

        // Also add to array if it exists (for CSV processor compatibility)
        if (Array.isArray(this.loadingLog)) {
            this.loadingLog.push(message);
        }
    }

    addToProcessingLog(message) {
        if (this.processingLogElement && this.processingLogElement.appendChild) {
            const logEntry = document.createElement('div');
            logEntry.className = 'log-entry processing';
            logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
            this.processingLogElement.appendChild(logEntry);
            this.processingLogElement.scrollTop = this.processingLogElement.scrollHeight;
        }

        // Also add to array if it exists (for CSV processor compatibility)
        if (Array.isArray(this.processingLog)) {
            this.processingLog.push(message);
        }
    }

    addToErrorLog(message) {
        if (this.errorLogElement && this.errorLogElement.appendChild) {
            const logEntry = document.createElement('div');
            logEntry.className = 'log-entry error';
            logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
            this.errorLogElement.appendChild(logEntry);
            this.errorLogElement.scrollTop = this.errorLogElement.scrollHeight;
        }

        // Also add to array if it exists (for CSV processor compatibility)
        if (Array.isArray(this.errorLog)) {
            this.errorLog.push(message);
        }
    }

    showProgress() {
        if (this.progressSection) {
            this.progressSection.classList.remove('hidden');
        }
    }

    hideProgress() {
        if (this.progressSection) {
            setTimeout(() => {
                this.progressSection.classList.add('hidden');
            }, 1000);
        }
    }

    updateProgress(percent, text) {
        if (this.progressFill) {
            this.progressFill.style.width = percent + '%';
        }
        if (this.progressText) {
            this.progressText.textContent = text;
        }
    }

    updatePreview() {
        const width = this.outputWidth.value;
        const height = this.outputHeight.value;
        const faceHeight = this.faceHeightPct.value;
        const format = this.outputFormat.value.toUpperCase();

        const previewElement = document.getElementById('previewText');
        if (previewElement) {
            previewElement.textContent = `${width}×${height}px, face at ${faceHeight}% height, ${format} format`;
        }

        const ratio = width / height;
        const aspectRatioElement = document.getElementById('aspectRatioText');
        if (aspectRatioElement) {
            aspectRatioElement.textContent = `${ratio.toFixed(2)}:1 ratio`;
        }

        if (this.aspectRatioLocked) {
            this.maintainAspectRatio();
        }
    }

    updateAdvancedPositioning() {
        const mode = this.positioningMode.value;
        const advancedPositioning = document.getElementById('advancedPositioning');

        if (advancedPositioning) {
            if (mode === 'custom') {
                advancedPositioning.style.display = 'block';
            } else {
                advancedPositioning.style.display = 'none';
            }
        }
    }

    updateOffsetDisplay(type) {
        const value = type === 'vertical' ? this.verticalOffset.value : this.horizontalOffset.value;
        const displayElement = document.getElementById(`${type}OffsetValue`);
        if (displayElement) {
            displayElement.textContent = value + '%';
        }
    }

    toggleAspectRatioLock() {
        this.aspectRatioLocked = !this.aspectRatioLocked;
        if (this.aspectRatioLock) {
            this.aspectRatioLock.textContent = this.aspectRatioLocked ? '🔒' : '🔓';
        }

        if (this.aspectRatioLocked) {
            this.currentAspectRatio = this.outputWidth.value / this.outputHeight.value;
        }
    }

    maintainAspectRatio() {
        if (!this.aspectRatioLocked) return;

        const currentRatio = this.outputWidth.value / this.outputHeight.value;
        if (Math.abs(currentRatio - this.currentAspectRatio) > 0.01) {
            this.outputHeight.value = Math.round(this.outputWidth.value / this.currentAspectRatio);
        }
    }

    applySizePreset() {
        const preset = this.sizePreset.value;
        const presets = {
            linkedin: { width: 400, height: 400 },
            passport: { width: 413, height: 531 },
            instagram: { width: 1080, height: 1080 },
            idcard: { width: 332, height: 498 },
            avatar: { width: 512, height: 512 },
            headshot: { width: 600, height: 800 }
        };

        if (presets[preset]) {
            this.outputWidth.value = presets[preset].width;
            this.outputHeight.value = presets[preset].height;
            this.updatePreview();
        }
    }

    updateFormatSettings() {
        const format = this.outputFormat.value;
        const jpegQualityGroup = document.getElementById('jpegQualityGroup');

        if (jpegQualityGroup) {
            if (format === 'jpeg') {
                jpegQualityGroup.classList.remove('hidden');
            } else {
                jpegQualityGroup.classList.add('hidden');
            }
        }

        this.updatePreview();
    }

    updateSliderValue(type) {
        const sliderMap = {
            exposure: { element: this.exposureAdjustment, display: 'exposureValue' },
            contrast: { element: this.contrastAdjustment, display: 'contrastValue' },
            sharpness: { element: this.sharpnessControl, display: 'sharpnessValue' },
            skinSmoothing: { element: this.skinSmoothing, display: 'skinSmoothingValue' },
            backgroundBlur: { element: this.backgroundBlur, display: 'backgroundBlurValue' }
        };

        const config = sliderMap[type];
        if (config && config.element) {
            const value = config.element.value;
            const displayElement = document.getElementById(config.display);
            if (displayElement) {
                if (type === 'backgroundBlur') {
                    displayElement.textContent = value + 'px';
                } else {
                    displayElement.textContent = value;
                }
            }
        }
    }

    toggleDarkMode() {
        this.isDarkMode = !this.isDarkMode;
        document.body.classList.toggle('dark-mode', this.isDarkMode);

        const darkModeBtn = document.getElementById('darkModeBtn');
        if (darkModeBtn) {
            darkModeBtn.textContent = this.isDarkMode ? '☀️' : '🌙';
            darkModeBtn.setAttribute('aria-pressed', this.isDarkMode.toString());
        }
    }

    async downloadAsZip(canvases, filename = 'cropped_faces.zip') {
        const zip = new JSZip();
        const settings = this.getSettings();

        for (let i = 0; i < canvases.length; i++) {
            const canvas = canvases[i];
            const filenameForCanvas = this.generateFilename({ file: { name: 'face' } }, i);

            const blob = await new Promise(resolve => {
                canvas.toBlob(resolve, `image/${settings.format}`, settings.quality);
            });

            zip.file(filenameForCanvas, blob);
        }

        const zipBlob = await zip.generateAsync({ type: 'blob' });
        const url = URL.createObjectURL(zipBlob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        a.click();
        URL.revokeObjectURL(url);
    }

    extractFileName(filePath) {
        return filePath.split(/[\\/]/).pop();
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    async loadImageFile(file) {
        return new Promise((resolve, reject) => {
            const img = new Image();
            img.onload = () => {
                URL.revokeObjectURL(img.src);
                resolve(img);
            };
            img.onerror = () => {
                URL.revokeObjectURL(img.src);
                reject(new Error(`Failed to load image: ${file.name}`));
            };
            img.src = URL.createObjectURL(file);
        });
    }

    updateStatistics() {
        if (this.totalFacesDetected) {
            this.totalFacesDetected.textContent = this.statistics.totalFacesDetected;
        }
        if (this.imagesProcessed) {
            this.imagesProcessed.textContent = this.statistics.imagesProcessed;
        }

        const successRate = this.statistics.imagesProcessed > 0 ?
            Math.round((this.statistics.successfulProcessing / this.statistics.imagesProcessed) * 100) : 0;

        if (this.successRate) {
            this.successRate.textContent = successRate + '%';
        }

        const avgTime = this.statistics.processingTimes.length > 0 ?
            Math.round(this.statistics.processingTimes.reduce((a, b) => a + b, 0) / this.statistics.processingTimes.length) : 0;

        if (this.avgProcessingTime) {
            this.avgProcessingTime.textContent = avgTime + 'ms';
        }
    }

    resetStatistics() {
        this.statistics = {
            totalFacesDetected: 0,
            imagesProcessed: 0,
            successfulProcessing: 0,
            processingTimes: [],
            startTime: null
        };
        this.updateStatistics();
    }

    // Specialized logging methods
    addToLoadingLog(message) {
        if (this.loadingLogElement) {
            this.addLogEntry(this.loadingLogElement, message);
        } else {
            this.addToLog(message, 'info');
        }
    }

    addToProcessingLog(message, type = 'info') {
        if (this.processingLogElement) {
            this.addLogEntry(this.processingLogElement, message, type);
        } else {
            this.addToLog(message, type);
        }
    }

    addToErrorLog(message, type = 'error') {
        if (this.errorLogElement) {
            this.addLogEntry(this.errorLogElement, message, type);
        } else {
            this.addToLog(message, type);
        }
    }

    addLogEntry(logElement, message, type = 'info') {
        if (!logElement) return;

        const logEntry = document.createElement('div');
        logEntry.className = `log-entry ${type}`;
        logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
        logElement.appendChild(logEntry);
        logElement.scrollTop = logElement.scrollHeight;
    }

    // Common selection methods for images
    selectAll() {
        if (this.images) {
            this.images.forEach(imageData => {
                imageData.selected = true;
            });
            this.refreshImageDisplay();
        }
    }

    selectNone() {
        if (this.images) {
            this.images.forEach(imageData => {
                imageData.selected = false;
            });
            this.refreshImageDisplay();
        }
    }

    // Common selection methods for faces
    selectAllFaces() {
        // This will be implemented by subclasses as they have different face storage methods
        console.log('selectAllFaces should be implemented by subclass');
    }

    selectNoneFaces() {
        // This will be implemented by subclasses as they have different face storage methods
        console.log('selectNoneFaces should be implemented by subclass');
    }

    updateSelectionCount() {
        if (this.selectedCount && this.totalCount && this.images) {
            const selected = Array.from(this.images.values()).filter(img => img.selected).length;
            const total = this.images.size;
            this.selectedCount.textContent = selected;
            this.totalCount.textContent = total;
        }
    }

    // Helper method for subclasses to call when they need to refresh display
    refreshImageDisplay() {
        if (typeof this.displayImageGallery === 'function') {
            this.displayImageGallery();
        }
        if (typeof this.updateSelectionCount === 'function') {
            this.updateSelectionCount();
        }
    }

    // File stats updating (common pattern)
    updateFileStats() {
        // Default implementation - subclasses can override
        if (this.totalCount && this.images) {
            this.totalCount.textContent = this.images.size;
        }
    }

    // Common clear all pattern
    clearAll() {
        // Base implementation - subclasses should extend this
        if (this.images) {
            this.images.clear();
        }
        this.resetStatistics();
        this.updateStatus('Ready');

        // Clear UI elements common to all classes
        if (this.progressSection) {
            this.progressSection.classList.add('hidden');
        }

        this.refreshImageDisplay();
    }

    // Common update UI pattern
    updateUI() {
        // Base implementation - subclasses should extend this
        const hasImages = this.images && this.images.size > 0;
        const hasSelectedImages = hasImages && Array.from(this.images.values()).some(img => img.selected);

        if (this.processAllBtn) {
            this.processAllBtn.disabled = !hasImages;
        }
        if (this.processSelectedBtn) {
            this.processSelectedBtn.disabled = !hasSelectedImages;
        }
        if (this.clearAllBtn) {
            this.clearAllBtn.disabled = !hasImages;
        }
        if (this.downloadAllBtn) {
            this.downloadAllBtn.disabled = true; // Enabled after processing
        }

        this.updateFileStats();
    }
}