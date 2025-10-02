import {
    BoundingBox,
    PixelBoundingBox,
    FaceData,
    ImageData,
    Statistics,
    CropSettings
} from './types.js';

// Additional interfaces for internal use
interface SizePreset {
    width: number;
    height: number;
}

interface SizePresets {
    [key: string]: SizePreset;
}

interface SliderConfig {
    element: HTMLInputElement | null;
    display: string;
}

interface SliderMap {
    [key: string]: SliderConfig;
}

export class BaseFaceCropper {
    protected detector: FaceDetector | null;
    protected aspectRatioLocked: boolean;
    protected currentAspectRatio: number;
    protected isDarkMode: boolean;
    protected processingStartTime: number | null;
    protected statistics: Statistics;

    // DOM Elements - marked as optional since they may not exist in all implementations
    protected status?: HTMLElement | null;
    protected processingStatus?: HTMLElement | null;
    protected outputWidth!: HTMLInputElement;
    protected outputHeight!: HTMLInputElement;
    protected faceHeightPct!: HTMLInputElement;
    protected positioningMode!: HTMLSelectElement;
    protected verticalOffset!: HTMLInputElement;
    protected horizontalOffset!: HTMLInputElement;
    protected outputFormat!: HTMLSelectElement;
    protected jpegQuality!: HTMLInputElement;
    protected namingTemplate?: HTMLInputElement;
    protected autoColorCorrection?: HTMLInputElement;
    protected progressSection?: HTMLElement;
    protected progressFill?: HTMLElement;
    protected progressText?: HTMLElement;
    protected aspectRatioLock?: HTMLElement;
    protected sizePreset!: HTMLSelectElement;
    protected exposureAdjustment?: HTMLInputElement;
    protected contrastAdjustment?: HTMLInputElement;
    protected sharpnessControl?: HTMLInputElement;
    protected skinSmoothing?: HTMLInputElement;
    protected backgroundBlur?: HTMLInputElement;
    protected processingLogElement?: HTMLElement;
    protected loadingLogElement?: HTMLElement;
    protected errorLogElement?: HTMLElement;
    protected totalFacesDetected?: HTMLElement;
    protected imagesProcessed?: HTMLElement;
    protected successRate?: HTMLElement;
    protected avgProcessingTime?: HTMLElement;
    protected selectedCount?: HTMLElement;
    protected totalCount?: HTMLElement;
    protected processAllBtn?: HTMLButtonElement;
    protected processSelectedBtn?: HTMLButtonElement;
    protected clearAllBtn?: HTMLButtonElement;
    protected downloadAllBtn?: HTMLButtonElement;

    // Arrays for logging (used by CSV processor)
    protected loadingLog?: any[];
    protected processingLog?: any[];
    protected errorLog?: any[];

    // Images map (used by subclasses)
    protected images?: Map<string, ImageData>;

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

    convertBoundingBoxToPixels(
        bbox: BoundingBox | null | undefined,
        imageWidth: number,
        imageHeight: number
    ): PixelBoundingBox | null {
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

    async waitForMediaPipe(): Promise<void> {
        const maxWaitTime = 10000;
        const checkInterval = 100;
        let elapsed = 0;

        while (elapsed < maxWaitTime) {
            if (typeof window.vision !== 'undefined' &&
                window.vision.FilesetResolver &&
                window.vision.FaceDetector) {
                return;
            }
            await new Promise<void>(resolve => setTimeout(resolve, checkInterval));
            elapsed += checkInterval;
        }

        throw new Error('MediaPipe Tasks Vision library failed to load within timeout');
    }

    async loadModel(): Promise<void> {
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
            this.addToLog('Error loading face detection model: ' + (error instanceof Error ? error.message : String(error)), 'error');
        }
    }

    async cropFace(
        image: HTMLImageElement,
        face: { box: { xMin: number; yMin: number; width: number; height: number } },
        settings?: CropSettings
    ): Promise<HTMLCanvasElement> {
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

        if (!ctx) {
            throw new Error('Failed to get canvas 2d context');
        }

        ctx.drawImage(
            image,
            cropX, cropY, cropWidth, cropHeight,
            0, 0, settings.outputWidth, settings.outputHeight
        );

        await this.applyEnhancements(ctx, canvas);

        return canvas;
    }

    async applyEnhancements(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement): Promise<void> {
        if (this.autoColorCorrection?.checked) {
            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
            ctx.putImageData(imageData, 0, 0);
        }
    }

    getSettings(): CropSettings {
        return {
            outputWidth: parseInt(this.outputWidth.value),
            outputHeight: parseInt(this.outputHeight.value),
            faceHeightPct: parseInt(this.faceHeightPct.value),
            positioningMode: this.positioningMode.value,
            verticalOffset: parseInt(this.verticalOffset.value),
            horizontalOffset: parseInt(this.horizontalOffset.value),
            outputFormat: this.outputFormat.value,
            jpegQuality: this.outputFormat.value === 'jpeg' ? (parseInt(this.jpegQuality.value) / 100) : 1,
            namingTemplate: this.namingTemplate?.value || 'face_{original}_{index}'
        };
    }

    generateFilename(imageData: Partial<ImageData>, index: number, template?: string): string {
        const settings = this.getSettings();
        const filenameTemplate = template || this.namingTemplate?.value || 'face_{original}_{index}';
        const originalName = imageData.file ? imageData.file.name.replace(/\.[^/.]+$/, '') : 'image';
        const extension = settings.outputFormat === 'jpeg' ? 'jpg' : settings.outputFormat;

        return filenameTemplate
            .replace('{original}', originalName)
            .replace('{csv_name}', imageData.csvOutputName || originalName)
            .replace('{index}', String(index + 1))
            .replace('{timestamp}', String(Date.now()))
            .replace('{width}', String(settings.outputWidth))
            .replace('{height}', String(settings.outputHeight)) + '.' + extension;
    }

    updateStatus(message: string): void {
        if (this.status) {
            this.status.textContent = message;
        }
        if (this.processingStatus) {
            this.processingStatus.textContent = message.split('.')[0];
        }
    }

    addToLog(message: string, type: 'info' | 'error' | 'warning' = 'info'): void {
        const logElement = this.processingLogElement || this.loadingLogElement;
        if (logElement && logElement.appendChild) {
            const logEntry = document.createElement('div');
            logEntry.className = `log-entry ${type}`;
            logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
            logElement.appendChild(logEntry);
            logElement.scrollTop = logElement.scrollHeight;
        }
    }

    addToLoadingLog(message: string): void {
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

    addToProcessingLog(message: string, type: 'info' | 'error' | 'warning' = 'info'): void {
        if (this.processingLogElement && this.processingLogElement.appendChild) {
            const logEntry = document.createElement('div');
            logEntry.className = `log-entry processing`;
            logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
            this.processingLogElement.appendChild(logEntry);
            this.processingLogElement.scrollTop = this.processingLogElement.scrollHeight;
        }

        // Also add to array if it exists (for CSV processor compatibility)
        if (Array.isArray(this.processingLog)) {
            this.processingLog.push(message);
        }
    }

    addToErrorLog(message: string): void {
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

    showProgress(): void {
        if (this.progressSection) {
            this.progressSection.classList.remove('hidden');
        }
    }

    hideProgress(): void {
        if (this.progressSection) {
            setTimeout(() => {
                this.progressSection?.classList.add('hidden');
            }, 1000);
        }
    }

    updateProgress(percent: number, text: string): void {
        if (this.progressFill) {
            this.progressFill.style.width = percent + '%';
        }
        if (this.progressText) {
            this.progressText.textContent = text;
        }
    }

    updatePreview(): void {
        const width = this.outputWidth.value;
        const height = this.outputHeight.value;
        const faceHeight = this.faceHeightPct.value;
        const format = this.outputFormat.value.toUpperCase();

        const previewElement = document.getElementById('previewText');
        if (previewElement) {
            previewElement.textContent = `${width}×${height}px, face at ${faceHeight}% height, ${format} format`;
        }

        const ratio = parseInt(width) / parseInt(height);
        const aspectRatioElement = document.getElementById('aspectRatioText');
        if (aspectRatioElement) {
            aspectRatioElement.textContent = `${ratio.toFixed(2)}:1 ratio`;
        }

        if (this.aspectRatioLocked) {
            this.maintainAspectRatio();
        }
    }

    updateAdvancedPositioning(): void {
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

    updateOffsetDisplay(type: 'vertical' | 'horizontal'): void {
        const value = type === 'vertical' ? this.verticalOffset.value : this.horizontalOffset.value;
        const displayElement = document.getElementById(`${type}OffsetValue`);
        if (displayElement) {
            displayElement.textContent = value + '%';
        }
    }

    toggleAspectRatioLock(): void {
        this.aspectRatioLocked = !this.aspectRatioLocked;
        if (this.aspectRatioLock) {
            this.aspectRatioLock.textContent = this.aspectRatioLocked ? '🔒' : '🔓';
        }

        if (this.aspectRatioLocked) {
            this.currentAspectRatio = parseInt(this.outputWidth.value) / parseInt(this.outputHeight.value);
        }
    }

    maintainAspectRatio(): void {
        if (!this.aspectRatioLocked) return;

        const currentRatio = parseInt(this.outputWidth.value) / parseInt(this.outputHeight.value);
        if (Math.abs(currentRatio - this.currentAspectRatio) > 0.01) {
            this.outputHeight.value = String(Math.round(parseInt(this.outputWidth.value) / this.currentAspectRatio));
        }
    }

    applySizePreset(): void {
        const preset = this.sizePreset.value;
        const presets: SizePresets = {
            linkedin: { width: 400, height: 400 },
            passport: { width: 413, height: 531 },
            instagram: { width: 1080, height: 1080 },
            idcard: { width: 332, height: 498 },
            avatar: { width: 512, height: 512 },
            headshot: { width: 600, height: 800 }
        };

        if (presets[preset]) {
            this.outputWidth.value = String(presets[preset].width);
            this.outputHeight.value = String(presets[preset].height);
            this.updatePreview();
        }
    }

    updateFormatSettings(): void {
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

    updateSliderValue(type: 'exposure' | 'contrast' | 'sharpness' | 'skinSmoothing' | 'backgroundBlur'): void {
        const sliderMap: SliderMap = {
            exposure: { element: this.exposureAdjustment || null, display: 'exposureValue' },
            contrast: { element: this.contrastAdjustment || null, display: 'contrastValue' },
            sharpness: { element: this.sharpnessControl || null, display: 'sharpnessValue' },
            skinSmoothing: { element: this.skinSmoothing || null, display: 'skinSmoothingValue' },
            backgroundBlur: { element: this.backgroundBlur || null, display: 'backgroundBlurValue' }
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

    toggleDarkMode(): void {
        this.isDarkMode = !this.isDarkMode;
        document.body.classList.toggle('dark-mode', this.isDarkMode);

        const darkModeBtn = document.getElementById('darkModeBtn');
        if (darkModeBtn) {
            darkModeBtn.textContent = this.isDarkMode ? '☀️' : '🌙';
            darkModeBtn.setAttribute('aria-pressed', this.isDarkMode.toString());
        }
    }

    async downloadAsZip(canvases: HTMLCanvasElement[], filename: string = 'cropped_faces.zip'): Promise<void> {
        // JSZip is loaded globally
        const JSZip = (window as any).JSZip;
        const zip = new JSZip();
        const settings = this.getSettings();

        for (let i = 0; i < canvases.length; i++) {
            const canvas = canvases[i];
            const filenameForCanvas = this.generateFilename({ file: { name: 'face' } as File }, i);

            const blob = await new Promise<Blob | null>(resolve => {
                canvas.toBlob(resolve, `image/${settings.outputFormat}`, settings.jpegQuality);
            });

            if (blob) {
                zip.file(filenameForCanvas, blob);
            }
        }

        const zipBlob = await zip.generateAsync({ type: 'blob' });
        const url = URL.createObjectURL(zipBlob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        a.click();
        URL.revokeObjectURL(url);
    }

    extractFileName(filePath: string): string {
        return filePath.split(/[\\/]/).pop() || '';
    }

    escapeHtml(text: string): string {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    async loadImageFile(file: File): Promise<HTMLImageElement> {
        return new Promise((resolve, reject) => {
            const img = new Image();
            img.onload = () => {
                // Don't revoke the URL - we need it for gallery display
                // The URL will be automatically cleaned up when the page unloads
                resolve(img);
            };
            img.onerror = () => {
                URL.revokeObjectURL(img.src);
                reject(new Error(`Failed to load image: ${file.name}`));
            };
            img.src = URL.createObjectURL(file);
        });
    }

    updateStatistics(): void {
        if (this.totalFacesDetected) {
            this.totalFacesDetected.textContent = String(this.statistics.totalFacesDetected);
        }
        if (this.imagesProcessed) {
            this.imagesProcessed.textContent = String(this.statistics.imagesProcessed);
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

    resetStatistics(): void {
        this.statistics = {
            totalFacesDetected: 0,
            imagesProcessed: 0,
            successfulProcessing: 0,
            processingTimes: [],
            startTime: null
        };
        this.updateStatistics();
    }

    addLogEntry(logElement: HTMLElement | null | undefined, message: string, type: 'info' | 'error' | 'warning' = 'info'): void {
        if (!logElement) return;

        const logEntry = document.createElement('div');
        logEntry.className = `log-entry ${type}`;
        logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
        logElement.appendChild(logEntry);
        logElement.scrollTop = logElement.scrollHeight;
    }

    createGalleryItem(imageData: ImageData): HTMLDivElement {
        const item = document.createElement('div');
        item.className = 'gallery-item';
        item.dataset.imageId = imageData.id;

        if (imageData.selected) {
            item.classList.add('selected');
        }
        if (imageData.processed) {
            item.classList.add('processed');
        }
        if (imageData.status === 'processing') {
            item.classList.add('processing');
        }

        const img = document.createElement('img');
        try {
            if (imageData.image instanceof HTMLImageElement && imageData.image.src) {
                // Use the existing image source
                img.src = imageData.image.src;
            } else if (imageData.file) {
                // Create blob URL from file
                img.src = URL.createObjectURL(imageData.file);
            } else {
                // Fallback: show placeholder
                img.src = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2Y0ZjRmNCIvPjx0ZXh0IHg9IjUwIiB5PSI1MCIgZm9udC1zaXplPSIxNCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZHk9Ii4zZW0iIGZpbGw9IiM5OTkiPkltYWdlPC90ZXh0Pjwvc3ZnPg==';
            }
        } catch (error) {
            // Fallback: show placeholder
            img.src = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2Y0ZjRmNCIvPjx0ZXh0IHg9IjUwIiB5PSI1MCIgZm9udC1zaXplPSIxNCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZHk9Ii4zZW0iIGZpbGw9IiM5OTkiPkltYWdlPC90ZXh0Pjwvc3ZnPg==';
        }
        img.alt = imageData.file ? imageData.file.name : 'Image';

        const info = document.createElement('div');
        info.className = 'gallery-item-info';

        const name = document.createElement('div');
        name.className = 'gallery-item-name';
        name.textContent = imageData.csvOutputName || imageData.file.name;

        const status = document.createElement('div');
        status.className = 'gallery-item-status';
        if (imageData.processed) {
            status.textContent = `${imageData.results.length} faces`;
            status.classList.add('processed');
        } else if (imageData.status === 'processing') {
            status.textContent = 'Processing...';
            status.classList.add('processing');
        } else {
            status.textContent = 'Ready';
        }

        info.appendChild(name);
        info.appendChild(status);
        item.appendChild(img);
        item.appendChild(info);

        return item;
    }

    // Common selection methods for images
    selectAll(): void {
        if (this.images) {
            this.images.forEach(imageData => {
                imageData.selected = true;
            });
            this.refreshImageDisplay();
        }
    }

    selectNone(): void {
        if (this.images) {
            this.images.forEach(imageData => {
                imageData.selected = false;
            });
            this.refreshImageDisplay();
        }
    }

    // Common selection methods for faces
    selectAllFaces(): void {
        // This will be implemented by subclasses as they have different face storage methods
        console.log('selectAllFaces should be implemented by subclass');
    }

    selectNoneFaces(): void {
        // This will be implemented by subclasses as they have different face storage methods
        console.log('selectNoneFaces should be implemented by subclass');
    }

    updateSelectionCount(): void {
        if (this.selectedCount && this.totalCount && this.images) {
            const selected = Array.from(this.images.values()).filter(img => img.selected).length;
            const total = this.images.size;
            this.selectedCount.textContent = String(selected);
            this.totalCount.textContent = String(total);
        }
    }

    // Helper method for subclasses to call when they need to refresh display
    refreshImageDisplay(): void {
        if (typeof (this as any).displayImageGallery === 'function') {
            (this as any).displayImageGallery();
        }
        if (typeof this.updateSelectionCount === 'function') {
            this.updateSelectionCount();
        }
    }

    // File stats updating (common pattern)
    updateFileStats(): void {
        // Default implementation - subclasses can override
        if (this.totalCount && this.images) {
            this.totalCount.textContent = String(this.images.size);
        }
    }

    // Common clear all pattern
    clearAll(): void {
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
    updateUI(): void {
        // Base implementation - subclasses should extend this
        const hasImages = this.images && this.images.size > 0;
        const hasSelectedImages = hasImages && Array.from(this.images!.values()).some(img => img.selected);

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
