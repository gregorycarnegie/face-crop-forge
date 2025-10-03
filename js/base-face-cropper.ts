import {
    BoundingBox,
    PixelBoundingBox,
    FaceData,
    ImageData,
    Statistics,
    CropSettings,
    CropResult
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
    protected inputCanvas?: HTMLCanvasElement;
    protected faceOverlays?: HTMLElement;
    protected faceCount?: HTMLElement;
    protected selectedFaceCount?: HTMLElement;
    protected ctx?: CanvasRenderingContext2D;

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
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            currentImage.faces.forEach(face => face.selected = true);

            // Only call these methods if they're implemented in the subclass
            if (typeof (this as any).displayImageWithFaceOverlays === 'function') {
                (this as any).displayImageWithFaceOverlays(currentImage);
            }
            if (typeof (this as any).updateFaceCounter === 'function') {
                (this as any).updateFaceCounter();
            }
        }
    }

    selectNoneFaces(): void {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            currentImage.faces.forEach(face => face.selected = false);

            // Only call these methods if they're implemented in the subclass
            if (typeof (this as any).displayImageWithFaceOverlays === 'function') {
                (this as any).displayImageWithFaceOverlays(currentImage);
            }
            if (typeof (this as any).updateFaceCounter === 'function') {
                (this as any).updateFaceCounter();
            }
        }
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

    // Generate unique image ID
    generateImageId(): string {
        return 'img_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
    }

    // Update selection counter display
    updateSelectionCounter(): void {
        if (this.selectedCount && this.totalCount && this.images) {
            const selected = Array.from(this.images.values()).filter(img => img.selected).length;
            const total = this.images.size;
            this.selectedCount.textContent = String(selected);
            this.totalCount.textContent = String(total);
        }
    }

    // Face detection with quality analysis
    async detectFacesWithQuality(image: HTMLImageElement): Promise<FaceData[]> {
        if (!this.detector) {
            throw new Error('Face detection model not loaded. Please wait for model to load.');
        }

        const detectionResult = await this.detector.detect(image);
        const detectedFaces: FaceData[] = [];

        if (detectionResult.detections && detectionResult.detections.length > 0) {
            for (let i = 0; i < detectionResult.detections.length; i++) {
                const detection = detectionResult.detections[i];
                const bbox = detection.boundingBox;
                const box = this.convertBoundingBoxToPixels(bbox, image.width, image.height);
                if (!box) {
                    continue;
                }
                const { x, y, width, height } = box;

                // Use detection confidence from MediaPipe
                const confidence = detection.categories && detection.categories.length > 0
                    ? detection.categories[0].score
                    : 0.8; // Default confidence

                // Calculate face quality using blur detection
                const quality = await this.calculateFaceQuality(image, x, y, width, height);

                detectedFaces.push({
                    bbox: box,
                    id: `face_${i}`,
                    x: x,
                    y: y,
                    width: width,
                    height: height,
                    box: {
                        xMin: x,
                        yMin: y,
                        width: width,
                        height: height
                    },
                    confidence: confidence,
                    quality: quality,
                    selected: true, // Default to selected
                    index: i + 1
                });
            }
        }

        return detectedFaces;
    }

    // Calculate face quality score
    async calculateFaceQuality(image: HTMLImageElement, x: number, y: number, width: number, height: number): Promise<{ score: number; level: string }> {
        const safeX = Math.max(0, Math.min(image.width, Math.floor(x)));
        const safeY = Math.max(0, Math.min(image.height, Math.floor(y)));
        const maxWidth = image.width - safeX;
        const maxHeight = image.height - safeY;
        const safeWidth = Math.max(1, Math.min(Math.floor(width), maxWidth));
        const safeHeight = Math.max(1, Math.min(Math.floor(height), maxHeight));

        if (safeWidth <= 0 || safeHeight <= 0) {
            return { score: 0, level: 'unknown' };
        }

        const maxDimension = 1024;
        const downscale = Math.min(1, maxDimension / Math.max(safeWidth, safeHeight));
        const targetWidth = Math.max(1, Math.floor(safeWidth * downscale));
        const targetHeight = Math.max(1, Math.floor(safeHeight * downscale));

        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        canvas.width = targetWidth;
        canvas.height = targetHeight;

        ctx!.drawImage(
            image,
            safeX, safeY, safeWidth, safeHeight,
            0, 0, targetWidth, targetHeight
        );

        let imageData;
        try {
            imageData = ctx!.getImageData(0, 0, targetWidth, targetHeight);
        } catch (error: unknown) {
            console.warn('Face quality analysis skipped due to canvas limits', error);
            return { score: 0, level: 'unknown' };
        }

        const data = imageData.data;

        // Calculate Laplacian variance for blur detection
        const laplacianVariance = this.calculateLaplacianVariance(data, targetWidth, targetHeight);

        // Classify quality based on variance
        if (laplacianVariance > 1000) return { score: laplacianVariance, level: 'high' };
        if (laplacianVariance > 300) return { score: laplacianVariance, level: 'medium' };
        return { score: laplacianVariance, level: 'low' };
    }

    // Calculate Laplacian variance for blur detection
    calculateLaplacianVariance(data: Uint8ClampedArray, width: number, height: number): number {
        // Convert to grayscale and calculate Laplacian variance
        const gray = [];
        for (let i = 0; i < data.length; i += 4) {
            gray.push(0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2]);
        }

        let variance = 0;
        let count = 0;

        for (let y = 1; y < height - 1; y++) {
            for (let x = 1; x < width - 1; x++) {
                const idx = y * width + x;
                const laplacian =
                    -gray[idx - width - 1] - gray[idx - width] - gray[idx - width + 1] +
                    -gray[idx - 1] + 8 * gray[idx] - gray[idx + 1] +
                    -gray[idx + width - 1] - gray[idx + width] - gray[idx + width + 1];

                variance += laplacian * laplacian;
                count++;
            }
        }

        return count > 0 ? variance / count : 0;
    }

    // Calculate smart crop position based on positioning mode
    calculateSmartCropPosition(face: FaceData, cropWidth: number, cropHeight: number, imageWidth: number, imageHeight: number): { cropX: number; cropY: number } {
        const mode = this.positioningMode.value;
        const vOffset = parseInt(this.verticalOffset.value) / 100;
        const hOffset = parseInt(this.horizontalOffset.value) / 100;

        const faceX = face.x || 0;
        const faceY = face.y || 0;
        const faceWidth = face.width || 0;
        const faceHeight = face.height || 0;

        const faceCenterX = faceX + faceWidth / 2;
        const faceCenterY = faceY + faceHeight / 2;

        let targetX, targetY;

        switch (mode) {
            case 'rule-of-thirds':
                // Position eyes at rule of thirds points
                // Eyes are typically at about 65% from top of face bounding box
                const eyesY = faceY + faceHeight * 0.35;

                // Place eyes at 1/3 from top of crop
                targetX = faceCenterX + (hOffset * cropWidth / 2);
                targetY = eyesY - (cropHeight / 3);
                break;

            case 'custom':
                // Use manual offsets from center
                targetX = faceCenterX + (hOffset * cropWidth / 2);
                targetY = faceCenterY + (vOffset * cropHeight / 2);
                break;

            case 'center':
            default:
                // Center the face
                targetX = faceCenterX;
                targetY = faceCenterY;
                break;
        }

        // Calculate crop position (top-left corner of crop area)
        let cropX = targetX - cropWidth / 2;
        let cropY = targetY - cropHeight / 2;

        // Clamp to image boundaries
        cropX = Math.max(0, Math.min(imageWidth - cropWidth, cropX));
        cropY = Math.max(0, Math.min(imageHeight - cropHeight, cropY));

        return { cropX, cropY };
    }

    // Generate batch filename from template
    generateBatchFilename(originalName: string, faceIndex: number, width: number, height: number): string {
        const template = this.namingTemplate?.value || 'face_{original}_{index}';
        const format = this.outputFormat.value;
        const baseName = originalName.replace(/\.[^/.]+$/, ''); // Remove extension
        const timestamp = new Date().toISOString().slice(0, 19).replace(/[:.]/g, '-');

        return template
            .replace(/{original}/g, baseName)
            .replace(/{index}/g, String(faceIndex))
            .replace(/{timestamp}/g, timestamp)
            .replace(/{width}/g, String(width))
            .replace(/{height}/g, String(height)) + '.' + format;
    }

    // Download all results (either as ZIP or individually)
    async downloadAllResults(): Promise<void> {
        // This will be implemented by subclasses as they have different result storage
        console.log('downloadAllResults should be implemented by subclass');
    }

    // Download results individually
    downloadIndividually(results: CropResult[]): void {
        let totalDownloads = 0;

        results.forEach((result) => {
            const link = document.createElement('a');
            link.download = result.filename || 'face.png';
            link.href = result.dataUrl || '';
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            totalDownloads++;
        });

        this.updateStatus(`Downloaded ${totalDownloads} cropped faces!`);
    }

    // Record processing timing
    recordProcessingStart(): void {
        this.processingStartTime = Date.now();
    }

    recordProcessingEnd(success: boolean, facesDetected: number = 0): void {
        if (this.processingStartTime) {
            const processingTime = Date.now() - this.processingStartTime;
            this.statistics.processingTimes.push(processingTime);

            // Keep only last 50 processing times for average calculation
            if (this.statistics.processingTimes.length > 50) {
                this.statistics.processingTimes.shift();
            }
        }

        this.statistics.imagesProcessed++;
        this.statistics.totalFacesDetected += facesDetected;

        if (success) {
            this.statistics.successfulProcessing++;
        }

        this.updateStatistics();
    }

    // Display image with face overlays
    displayImageWithFaceOverlays(imageData: ImageData): void {
        if (!this.inputCanvas || !this.faceOverlays) return;

        const maxWidth = 800;
        const maxHeight = 600;

        let { width, height } = imageData.image;
        const scale = Math.min(maxWidth / width, maxHeight / height, 1);

        const displayWidth = width * scale;
        const displayHeight = height * scale;

        this.inputCanvas.width = displayWidth;
        this.inputCanvas.height = displayHeight;

        if (!this.ctx && this.inputCanvas) {
            this.ctx = this.inputCanvas.getContext('2d')!;
        }

        if (this.ctx) {
            this.ctx.clearRect(0, 0, displayWidth, displayHeight);
            this.ctx.drawImage(imageData.image, 0, 0, displayWidth, displayHeight);
        }

        // Clear and recreate overlays
        this.faceOverlays.innerHTML = '';
        this.faceOverlays.style.width = displayWidth + 'px';
        this.faceOverlays.style.height = displayHeight + 'px';

        // Create face overlays
        if (imageData.faces) {
            imageData.faces.forEach((face: FaceData) => {
                this.createFaceOverlay(face, scale);
            });
        }
    }

    // Create face overlay element
    createFaceOverlay(face: FaceData, scale: number): void {
        if (!this.faceOverlays) return;

        const faceBox = document.createElement('div');
        faceBox.className = 'face-box';
        faceBox.dataset.faceId = String(face.id || '');
        faceBox.style.left = ((face.x || 0) * scale) + 'px';
        faceBox.style.top = ((face.y || 0) * scale) + 'px';
        faceBox.style.width = ((face.width || 0) * scale) + 'px';
        faceBox.style.height = ((face.height || 0) * scale) + 'px';

        if (face.selected) {
            faceBox.classList.add('selected');
        } else {
            faceBox.classList.add('unselected');
        }

        // Create checkbox
        const checkbox = document.createElement('div');
        checkbox.className = 'face-checkbox';
        if (face.selected) {
            checkbox.classList.add('selected');
        }
        checkbox.addEventListener('click', (e) => {
            e.stopPropagation();
            this.toggleFaceSelection(String(face.id || ''));
        });

        // Create index number
        const index = document.createElement('div');
        index.className = 'face-index';
        index.textContent = String(face.index || '');

        // Create confidence score
        const confidence = document.createElement('div');
        confidence.className = 'face-confidence';
        confidence.textContent = `${((face.confidence || 0) * 100).toFixed(0)}%`;

        // Create quality indicator
        const quality = document.createElement('div');
        quality.className = `face-quality ${face.quality?.level || 'unknown'}`;
        quality.textContent = (face.quality?.level || 'unknown').toUpperCase();

        // Add click to select
        faceBox.addEventListener('click', () => this.toggleFaceSelection(String(face.id || '')));

        // Append all elements
        faceBox.appendChild(checkbox);
        faceBox.appendChild(index);
        faceBox.appendChild(confidence);
        faceBox.appendChild(quality);

        this.faceOverlays.appendChild(faceBox);
    }

    // Toggle face selection
    toggleFaceSelection(faceId: string): void {
        // This will be implemented by subclasses as they have different image storage
        console.log('toggleFaceSelection should be implemented by subclass');
    }

    // Update face counter
    updateFaceCounter(): void {
        // This will be implemented by subclasses as they have different ways of tracking current image
        console.log('updateFaceCounter should be implemented by subclass');
    }

    // Crop faces from image data
    async cropFacesFromImageData(imageData: ImageData): Promise<CropResult[]> {
        if (!imageData.faces || imageData.faces.length === 0) return [];

        // Only crop selected faces
        const selectedFaces = imageData.faces.filter((face: FaceData) => face.selected);
        if (selectedFaces.length === 0) return [];

        const results: CropResult[] = [];
        const outputWidth = parseInt(this.outputWidth.value);
        const outputHeight = parseInt(this.outputHeight.value);
        const faceHeightPct = parseInt(this.faceHeightPct.value) / 100;

        // Use enhanced image if available, otherwise use original
        let sourceImage = imageData.image;

        // Create temporary canvas for processing
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        tempCanvas.width = sourceImage.width;
        tempCanvas.height = sourceImage.height;
        tempCtx!.drawImage(sourceImage, 0, 0);

        // Create crop canvas
        const cropCanvas = document.createElement('canvas');
        const cropCtx = cropCanvas.getContext('2d');
        cropCanvas.width = outputWidth;
        cropCanvas.height = outputHeight;

        for (let i = 0; i < selectedFaces.length; i++) {
            const face = selectedFaces[i];

            // Calculate target face height and scale
            const targetFaceHeight = outputHeight * faceHeightPct;
            const scale = targetFaceHeight / (face.height || 1);

            // Calculate crop dimensions
            const cropWidthSrc = outputWidth / scale;
            const cropHeightSrc = outputHeight / scale;

            // Calculate face position based on positioning mode
            const { cropX, cropY } = this.calculateSmartCropPosition(
                face, cropWidthSrc, cropHeightSrc, sourceImage.width, sourceImage.height
            );

            const finalCropWidth = Math.min(cropWidthSrc, sourceImage.width - cropX);
            const finalCropHeight = Math.min(cropHeightSrc, sourceImage.height - cropY);

            // Crop and resize
            cropCtx!.drawImage(
                tempCanvas,
                cropX, cropY, finalCropWidth, finalCropHeight,
                0, 0, outputWidth, outputHeight
            );

            // Generate image with selected format and quality
            const format = this.outputFormat.value;
            const quality = format === 'jpeg' ? parseInt(this.jpegQuality.value) / 100 : 1.0;

            let mimeType = 'image/png';
            if (format === 'jpeg') mimeType = 'image/jpeg';
            if (format === 'webp') mimeType = 'image/webp';

            const croppedDataUrl = cropCanvas.toDataURL(mimeType, quality);

            // Generate filename using template
            const filename = this.generateBatchFilename(imageData.file.name, face.index || 0, outputWidth, outputHeight);

            results.push({
                dataUrl: croppedDataUrl,
                faceIndex: face.index,
                faceId: String(face.id || ''),
                sourceImage: imageData.file.name,
                filename: filename,
                format: format,
                quality: format === 'jpeg' ? parseInt(this.jpegQuality.value) : 100
            });
        }

        return results;
    }
}
