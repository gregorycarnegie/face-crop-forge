import {
    BoundingBox,
    PixelBoundingBox,
    FaceData,
    ProcessorImageData,
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

// Face mesh contour indices (from MediaPipe FACEMESH_FACE_OVAL)
const FACE_OVAL_INDICES = [
    10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288,
    397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150, 136,
    172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109
];

export class BaseFaceCropper {
    protected detector: FaceDetector | null;
    protected faceLandmarker: FaceLandmarker | null;
    protected useSegmentation: boolean;
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
    protected redEyeRemoval?: HTMLInputElement;
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
    protected images?: Map<string, ProcessorImageData>;

    // Memory management
    protected memoryUsage?: { images: number; processed: number };
    protected memoryManagement?: HTMLSelectElement;

    // Lazy loading
    protected galleryPage?: number;
    protected galleryPageSize?: number;
    protected imageLoadQueue?: Array<{ files: any[]; page: number }>;
    protected isLoadingImages?: boolean;

    // Web Worker support
    protected faceDetectionWorker?: Worker | null;
    protected workerInitialized?: boolean;
    protected workerCallbacks?: Map<string, { resolve: (faces: any[]) => void; reject: (e: Error) => void }>;

    // Production optimization elements
    protected continueOnError?: HTMLInputElement;
    protected reducedResolution?: HTMLInputElement;
    protected enableWebWorkers?: HTMLInputElement;
    protected retryAttempts?: HTMLInputElement;
    protected galleryGrid?: HTMLElement;

    // Undo/Redo functionality
    protected undoStack: Array<any>;
    protected redoStack: Array<any>;

    // Navigation state
    protected currentImageIndex: number;
    protected currentFaceIndex: number;

    constructor() {
        this.detector = null;
        this.faceLandmarker = null;
        this.useSegmentation = true; // Use face segmentation by default
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

        // Initialize undo/redo stacks
        this.undoStack = [];
        this.redoStack = [];

        // Initialize navigation indices
        this.currentImageIndex = 0;
        this.currentFaceIndex = 0;
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
            this.updateStatus('Loading face detection models...');

            await this.waitForMediaPipe();

            if (!window.vision || !window.vision.FilesetResolver || !window.vision.FaceDetector) {
                throw new Error('MediaPipe Tasks Vision library not loaded');
            }

            const visionFileset = await window.vision.FilesetResolver.forVisionTasks(
                "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm"
            );

            // Load Face Detector (for bounding boxes)
            this.detector = await window.vision.FaceDetector.createFromOptions(visionFileset, {
                baseOptions: {
                    modelAssetPath: `https://storage.googleapis.com/mediapipe-models/face_detector/blaze_face_short_range/float16/1/blaze_face_short_range.tflite`,
                    delegate: "GPU"
                },
                runningMode: "IMAGE"
            });

            // Load Face Landmarker (for segmentation with 478 landmarks)
            if (window.vision.FaceLandmarker) {
                try {
                    this.faceLandmarker = await window.vision.FaceLandmarker.createFromOptions(visionFileset, {
                        baseOptions: {
                            modelAssetPath: `https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task`,
                            delegate: "GPU"
                        },
                        runningMode: "IMAGE",
                        numFaces: 10,
                        outputFaceBlendshapes: false,
                        outputFacialTransformationMatrixes: false
                    });
                    this.addToLog('Face Landmarker loaded successfully for segmentation');
                } catch (landmarkerError) {
                    console.warn('Face Landmarker failed to load, falling back to bounding box detection:', landmarkerError);
                    this.useSegmentation = false;
                }
            } else {
                this.useSegmentation = false;
            }

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

    /**
     * Create a face contour mask from Face Landmarker landmarks
     */
    protected createFaceContourMask(
        landmarks: Array<{x: number, y: number, z?: number}>,
        imageWidth: number,
        imageHeight: number
    ): HTMLCanvasElement {
        const maskCanvas = document.createElement('canvas');
        maskCanvas.width = imageWidth;
        maskCanvas.height = imageHeight;
        const ctx = maskCanvas.getContext('2d');

        if (!ctx) {
            throw new Error('Failed to get canvas context for face mask');
        }

        // Clear canvas
        ctx.clearRect(0, 0, imageWidth, imageHeight);

        // Create path from face oval landmarks
        ctx.beginPath();
        FACE_OVAL_INDICES.forEach((index, i) => {
            const landmark = landmarks[index];
            if (!landmark) return;

            const x = landmark.x * imageWidth;
            const y = landmark.y * imageHeight;

            if (i === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        });
        ctx.closePath();

        // Fill the face contour
        ctx.fillStyle = 'white';
        ctx.fill();

        return maskCanvas;
    }

    /**
     * Crop face using segmentation mask instead of bounding box
     */
    async cropFaceWithSegmentation(
        image: HTMLImageElement,
        landmarks: Array<{x: number, y: number, z?: number}>,
        boundingBox: { xMin: number; yMin: number; width: number; height: number },
        settings?: CropSettings
    ): Promise<HTMLCanvasElement> {
        if (!settings) {
            settings = this.getSettings();
        }

        // Create face contour mask
        const mask = this.createFaceContourMask(landmarks, image.width, image.height);

        // Calculate crop region similar to cropFace but using the mask
        const box = boundingBox;
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

        // Create output canvas
        const canvas = document.createElement('canvas');
        canvas.width = settings.outputWidth;
        canvas.height = settings.outputHeight;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });

        if (!ctx) {
            throw new Error('Failed to get canvas 2d context');
        }

        // Draw the image
        ctx.drawImage(
            image,
            cropX, cropY, cropWidth, cropHeight,
            0, 0, settings.outputWidth, settings.outputHeight
        );

        // Apply mask to create transparent background outside face contour
        const imageData = ctx.getImageData(0, 0, settings.outputWidth, settings.outputHeight);
        const maskCtx = mask.getContext('2d');
        if (maskCtx) {
            // Scale and position mask to match crop region
            const tempMaskCanvas = document.createElement('canvas');
            tempMaskCanvas.width = settings.outputWidth;
            tempMaskCanvas.height = settings.outputHeight;
            const tempMaskCtx = tempMaskCanvas.getContext('2d');

            if (tempMaskCtx) {
                tempMaskCtx.drawImage(
                    mask,
                    cropX, cropY, cropWidth, cropHeight,
                    0, 0, settings.outputWidth, settings.outputHeight
                );

                const maskData = tempMaskCtx.getImageData(0, 0, settings.outputWidth, settings.outputHeight);

                // Apply mask to alpha channel
                for (let i = 0; i < imageData.data.length; i += 4) {
                    const maskAlpha = maskData.data[i]; // Use red channel as mask
                    imageData.data[i + 3] = maskAlpha; // Set alpha channel
                }

                ctx.putImageData(imageData, 0, 0);
            }
        }

        await this.applyEnhancements(ctx, canvas);

        return canvas;
    }

    async applyEnhancements(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement): Promise<void> {
        // Get image data
        let imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

        // Apply auto color correction
        if (this.autoColorCorrection?.checked) {
            imageData = this.applyAutoColorCorrection(imageData);
        }

        // Apply exposure adjustment
        if (this.exposureAdjustment) {
            const exposure = parseFloat(this.exposureAdjustment.value);
            if (exposure !== 0) {
                imageData = this.applyExposureAdjustment(imageData, exposure);
            }
        }

        // Apply contrast adjustment
        if (this.contrastAdjustment) {
            const contrast = parseFloat(this.contrastAdjustment.value);
            if (contrast !== 1) {
                imageData = this.applyContrastAdjustment(imageData, contrast);
            }
        }

        // Apply sharpness
        if (this.sharpnessControl) {
            const sharpness = parseFloat(this.sharpnessControl.value);
            if (sharpness > 0) {
                imageData = this.applySharpness(imageData, sharpness);
            }
        }

        // Apply skin smoothing
        if (this.skinSmoothing) {
            const skinSmoothingAmount = parseFloat(this.skinSmoothing.value);
            if (skinSmoothingAmount > 0) {
                imageData = await this.applySkinSmoothing(imageData, skinSmoothingAmount);
            }
        }

        // Apply red-eye removal
        if (this.redEyeRemoval?.checked) {
            imageData = await this.applyRedEyeRemoval(imageData);
        }

        // Apply background blur
        if (this.backgroundBlur) {
            const blurAmount = parseFloat(this.backgroundBlur.value);
            if (blurAmount > 0) {
                imageData = await this.applyBackgroundBlur(imageData, blurAmount);
            }
        }

        // Put enhanced data back to canvas
        ctx.putImageData(imageData, 0, 0);
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

    generateFilename(imageData: Partial<ProcessorImageData>, index: number, template?: string): string {
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

    // toggleDarkMode implemented later in file with full functionality

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

    createGalleryItem(imageData: ProcessorImageData): HTMLDivElement {
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

    updateSelectionCount(): void {
        if (this.selectedCount && this.totalCount && this.images) {
            const selected = Array.from(this.images.values()).filter(img => img.selected).length;
            const total = this.images.size;
            this.selectedCount.textContent = String(selected);
            this.totalCount.textContent = String(total);
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
        if (!this.detector && !this.faceLandmarker) {
            throw new Error('Face detection model not loaded. Please wait for model to load.');
        }

        // Use detectFacesWithLandmarks which handles both Face Landmarker and Face Detector
        const detectedFaces = await this.detectFacesWithLandmarks(image);

        // Add quality analysis to each detected face
        for (const face of detectedFaces) {
            if (face.x !== undefined && face.y !== undefined && face.width && face.height) {
                const quality = await this.calculateFaceQuality(image, face.x, face.y, face.width, face.height);
                face.quality = quality;
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
    displayImageWithFaceOverlays(imageData: ProcessorImageData): void {
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

    // toggleFaceSelection and updateFaceCounter are implemented later in file with proper method signatures

    // Crop faces from image data
    /**
     * Detect faces using Face Landmarker (with 478 landmarks) for segmentation
     * Falls back to Face Detector if Face Landmarker is not available
     */
    async detectFacesWithLandmarks(image: HTMLImageElement): Promise<FaceData[]> {
        const faces: FaceData[] = [];

        // Try Face Landmarker first for segmentation
        if (this.useSegmentation && this.faceLandmarker) {
            try {
                const landmarkerResult = await this.faceLandmarker.detect(image);

                if (landmarkerResult.faceLandmarks && landmarkerResult.faceLandmarks.length > 0) {
                    landmarkerResult.faceLandmarks.forEach((landmarks, index) => {
                        // Calculate bounding box from landmarks
                        const xs = landmarks.map(l => l.x);
                        const ys = landmarks.map(l => l.y);
                        const xMin = Math.min(...xs);
                        const xMax = Math.max(...xs);
                        const yMin = Math.min(...ys);
                        const yMax = Math.max(...ys);

                        const width = xMax - xMin;
                        const height = yMax - yMin;

                        const bbox: PixelBoundingBox = {
                            x: xMin * image.width,
                            y: yMin * image.height,
                            width: width * image.width,
                            height: height * image.height
                        };

                        faces.push({
                            id: index,
                            bbox: bbox,
                            box: {
                                xMin: bbox.x,
                                yMin: bbox.y,
                                width: bbox.width,
                                height: bbox.height
                            },
                            selected: true,
                            confidence: 0.95, // Face Landmarker doesn't provide confidence
                            x: bbox.x,
                            y: bbox.y,
                            width: bbox.width,
                            height: bbox.height,
                            landmarks: landmarks // Store all 478 landmarks
                        });
                    });

                    return faces;
                }
            } catch (error) {
                console.warn('Face Landmarker detection failed, falling back to Face Detector:', error);
            }
        }

        // Fallback to Face Detector
        if (!this.detector) {
            throw new Error('No face detection model available');
        }

        const detectionResult = await this.detector.detect(image);
        if (detectionResult.detections && detectionResult.detections.length > 0) {
            detectionResult.detections.forEach((detection, index) => {
                const bbox = detection.boundingBox;
                const box = this.convertBoundingBoxToPixels(bbox, image.width, image.height);
                if (!box) return;

                const { x, y, width, height } = box;

                faces.push({
                    id: index,
                    bbox: box,
                    box: {
                        xMin: x,
                        yMin: y,
                        width: width,
                        height: height
                    },
                    confidence: detection.categories && detection.categories.length > 0
                        ? detection.categories[0].score
                        : 0.8,
                    selected: true,
                    x,
                    y,
                    width,
                    height
                });
            });
        }

        return faces;
    }

    async cropFacesFromImageData(imageData: ProcessorImageData): Promise<CropResult[]> {
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

            let croppedCanvas: HTMLCanvasElement;

            // Use segmentation if landmarks are available
            if (this.useSegmentation && face.landmarks && face.landmarks.length > 0) {
                try {
                    croppedCanvas = await this.cropFaceWithSegmentation(
                        sourceImage,
                        face.landmarks,
                        face.box!,
                        this.getSettings()
                    );
                } catch (error) {
                    console.warn('Segmentation crop failed, using bounding box:', error);
                    // Fallback to regular crop
                    croppedCanvas = await this.cropFace(sourceImage, face as any, this.getSettings());
                }
            } else {
                // Use regular bounding box crop
                croppedCanvas = await this.cropFace(sourceImage, face as any, this.getSettings());
            }

            // Generate image with selected format and quality
            const format = this.outputFormat.value;
            const quality = format === 'jpeg' ? parseInt(this.jpegQuality.value) / 100 : 1.0;

            let mimeType = 'image/png';
            if (format === 'jpeg') mimeType = 'image/jpeg';
            if (format === 'webp') mimeType = 'image/webp';

            const croppedDataUrl = croppedCanvas.toDataURL(mimeType, quality);

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

    // ============================================================================
    // ADVANCED FEATURES - Ported from batch-processor.ts
    // ============================================================================

    // Enhanced Error Logging with Severity Levels
    addToDetailedErrorLog(title: string, details: string, severity: 'error' | 'critical' | 'warning' | 'info' = 'error'): void {
        const timestamp = new Date().toLocaleTimeString();
        const entry = {
            timestamp,
            title,
            details,
            severity
        };

        if (!this.errorLog) {
            this.errorLog = [];
        }

        this.errorLog.push(entry);

        // Keep only last 50 error entries
        if (this.errorLog.length > 50) {
            this.errorLog.shift();
        }

        this.updateErrorLogDisplay();
    }

    updateErrorLogDisplay(): void {
        if (!this.errorLogElement) return;

        const logHtml = this.errorLog!
            .slice(-10) // Show last 10 entries
            .map(entry => `
                <div class="log-entry ${entry.severity}">
                    <span class="log-timestamp">${entry.timestamp}</span>
                    <strong>${entry.title}</strong>: ${entry.details}
                </div>
            `)
            .join('');

        this.errorLogElement.innerHTML = logHtml || '<div class="log-entry">No errors detected</div>';

        // Auto-scroll to bottom
        this.errorLogElement.scrollTop = this.errorLogElement.scrollHeight;
    }

    clearErrorLog(): void {
        this.errorLog = [];
        this.updateErrorLogDisplay();
        this.addToProcessingLog('Error log cleared', 'info');
        this.updateStatus('Error log cleared successfully');
    }

    exportErrorLog(): void {
        const errorData = {
            exportedAt: new Date().toISOString(),
            totalErrors: this.errorLog!.length,
            errors: this.errorLog!
        };

        const blob = new Blob([JSON.stringify(errorData, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const link = document.createElement('a');
        link.download = `face-cropper-errors-${new Date().toISOString().slice(0, 10)}.json`;
        link.href = url;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);

        URL.revokeObjectURL(url);
        this.addToProcessingLog('Error log exported', 'info');
        this.updateStatus('Error log exported successfully');
    }

    // Production-grade Image Loading with Validation
    async loadImageFromFileWithErrorHandling(file: File): Promise<HTMLImageElement> {
        return new Promise<HTMLImageElement>((resolve, reject) => {
            // Validate file type
            if (!file.type.startsWith('image/')) {
                reject(new Error('Invalid file type. Please select an image file.'));
                return;
            }

            // Validate file size (max 50MB)
            if (file.size > 50 * 1024 * 1024) {
                reject(new Error('File too large. Maximum size is 50MB.'));
                return;
            }

            const img = new Image();

            img.onload = () => {
                // Validate image dimensions
                if (img.width < 50 || img.height < 50) {
                    reject(new Error('Image too small. Minimum size is 50x50 pixels.'));
                    return;
                }

                if (img.width > 8192 || img.height > 8192) {
                    reject(new Error('Image too large. Maximum size is 8192x8192 pixels.'));
                    return;
                }

                resolve(img);
            };

            img.onerror = () => {
                reject(new Error('Corrupted or invalid image file.'));
            };

            try {
                img.src = URL.createObjectURL(file);
            } catch (error: unknown) {
                reject(new Error('Failed to create object URL for image.'));
            }
        });
    }

    // Memory Management
    createMemoryIndicator(): void {
        const indicator = document.createElement('div');
        indicator.className = 'memory-indicator';
        indicator.id = 'memoryIndicator';
        document.body.appendChild(indicator);

        // Update memory indicator periodically
        setInterval(() => this.updateMemoryIndicator(), 5000);
    }

    updateMemoryIndicator(): void {
        const indicator = document.getElementById('memoryIndicator');
        if (!indicator || !this.images) return;

        const memoryInfo = {
            images: this.images.size,
            processed: Array.from(this.images.values()).filter((img: any) => img.processed).length,
            errors: this.errorLog?.length || 0
        };

        const memoryScore = memoryInfo.images * 10 + memoryInfo.processed * 5;
        indicator.textContent = `Memory: ${memoryInfo.images} images, ${memoryInfo.processed} processed`;

        indicator.classList.remove('warning', 'critical', 'show');

        if (memoryScore > 200) {
            indicator.classList.add('critical', 'show');
        } else if (memoryScore > 100) {
            indicator.classList.add('warning', 'show');
        } else if (memoryInfo.images > 0) {
            indicator.classList.add('show');
        }
    }

    updateMemorySettings(): void {
        if (!this.memoryManagement) return;

        const mode = this.memoryManagement.value;
        switch (mode) {
            case 'aggressive':
                this.cleanupMemoryAggressive();
                break;
            case 'auto':
                this.cleanupMemoryAuto();
                break;
            case 'manual':
                // Do nothing, user handles cleanup
                break;
        }
    }

    cleanupMemoryAuto(): void {
        if (!this.images) return;

        // Clean up processed images older than 5 minutes
        const fiveMinutesAgo = Date.now() - (5 * 60 * 1000);

        for (const [id, imageData] of this.images) {
            const imgEntry = imageData as any;
            if (imgEntry.processed && imgEntry.processedAt && imgEntry.processedAt < fiveMinutesAgo) {
                this.cleanupImageData(imgEntry);
            }
        }
    }

    cleanupMemoryAggressive(): void {
        if (!this.images) return;

        // Clean up all processed images immediately
        for (const [id, imageData] of this.images) {
            if (imageData.processed) {
                this.cleanupImageData(imageData as any);
            }
        }
    }

    cleanupImageData(imageData: any): void {
        // Clean up blob URLs
        if (imageData.image && imageData.image.src && imageData.image.src.startsWith('blob:')) {
            URL.revokeObjectURL(imageData.image.src);
        }

        if (imageData.enhancedImage && imageData.enhancedImage.src && imageData.enhancedImage.src.startsWith('blob:')) {
            URL.revokeObjectURL(imageData.enhancedImage.src);
        }

        // Clear large data
        imageData.image = null;
        imageData.enhancedImage = null;

        // Mark as cleaned
        imageData.memoryCleanedUp = true;
    }

    // Lazy Loading Support
    protected async handleLargeImageBatch(files: any[]): Promise<void> {
        if (!this.galleryPageSize) this.galleryPageSize = 20;
        if (!this.imageLoadQueue) this.imageLoadQueue = [];

        this.addToLoadingLog(`Loading large batch: ${files.length} images`);

        // Load first page immediately
        const firstPage = files.slice(0, this.galleryPageSize);
        await this.loadImagePage(firstPage, 0);

        // Queue remaining files
        for (let i = this.galleryPageSize; i < files.length; i += this.galleryPageSize) {
            const page = files.slice(i, i + this.galleryPageSize);
            this.imageLoadQueue.push({ files: page, page: Math.floor(i / this.galleryPageSize) });
        }

        this.setupLazyLoading();
        this.updateStatus(`Loaded first ${firstPage.length} images. ${files.length - firstPage.length} queued for lazy loading.`);
    }

    protected async loadImagePage(files: any[], pageIndex: number): Promise<void> {
        if (!this.images) this.images = new Map();

        for (const fileOrWrapper of files) {
            const imageId = this.generateImageId();
            // Handle both File objects and wrapped objects like { file: File, outputName: string }
            const file = fileOrWrapper.file || fileOrWrapper;
            const outputName = fileOrWrapper.outputName;

            try {
                const image = await this.loadImageFromFileWithErrorHandling(file);
                this.images.set(imageId, {
                    id: imageId,
                    file: file,
                    image: image,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    ...(outputName && { csvOutputName: outputName })
                } as any);
            } catch (error: unknown) {
                this.addToDetailedErrorLog(`Failed to load image: ${file.name}`, (error as Error).message, 'error');
            }
        }

        this.refreshImageDisplay();
    }

    protected setupLazyLoading(): void {
        if (!this.galleryGrid) return;

        // Add lazy loading indicator to gallery
        const indicator = document.createElement('div');
        indicator.className = 'gallery-lazy-loading';
        indicator.innerHTML = 'Scroll to load more images...';
        this.galleryGrid.appendChild(indicator);

        // Setup intersection observer for lazy loading
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting && !this.isLoadingImages && this.imageLoadQueue && this.imageLoadQueue.length > 0) {
                    this.loadNextImagePage();
                }
            });
        });

        observer.observe(indicator);
    }

    protected async loadNextImagePage(): Promise<void> {
        if (!this.imageLoadQueue || this.imageLoadQueue.length === 0) return;

        this.isLoadingImages = true;
        const indicator = this.galleryGrid?.querySelector('.gallery-lazy-loading');

        if (indicator) {
            indicator.className = 'gallery-lazy-loading loading';
            indicator.innerHTML = '<span class="spinner"></span>Loading more images...';
        }

        const batch = this.imageLoadQueue.shift();
        if (!batch) return;

        const { files, page } = batch;
        await this.loadImagePage(files, page);

        this.isLoadingImages = false;

        if (indicator) {
            if (this.imageLoadQueue.length > 0) {
                indicator.className = 'gallery-lazy-loading';
                indicator.innerHTML = `Scroll to load more images... (${this.imageLoadQueue.length} pages remaining)`;
            } else {
                indicator.remove();
            }
        }
    }

    // Web Worker Support for Face Detection
    async initializeWebWorker(): Promise<void> {
        if (!this.enableWebWorkers?.checked) return;

        try {
            this.faceDetectionWorker = new Worker('dist/face-detection-worker.js');

            this.faceDetectionWorker.onmessage = (e) => {
                const { type, data, id, success, faces, error } = e.data;

                switch (type) {
                    case 'initialized':
                        this.workerInitialized = success;
                        if (success) {
                            this.addToProcessingLog('Web Worker initialized successfully', 'info');
                        } else {
                            this.addToDetailedErrorLog('Web Worker initialization failed', (error as Error).message, 'critical');
                            if (this.enableWebWorkers) this.enableWebWorkers.checked = false;
                        }
                        break;

                    case 'faceDetectionResult':
                        this.handleWorkerDetectionResult(id, faces);
                        break;

                    case 'error':
                        this.addToDetailedErrorLog(`Worker error for task ${id}`, (error as Error).message, 'critical');
                        break;
                }
            };

            this.faceDetectionWorker.onerror = (error) => {
                this.addToDetailedErrorLog('Web Worker error', error.message, 'critical');
                this.workerInitialized = false;
                if (this.enableWebWorkers) this.enableWebWorkers.checked = false;
            };

            // Initialize the worker
            this.faceDetectionWorker.postMessage({ type: 'initialize' });

        } catch (error: unknown) {
            this.addToDetailedErrorLog('Failed to create Web Worker', (error as Error).message, 'critical');
            if (this.enableWebWorkers) this.enableWebWorkers.checked = false;
        }
    }

    toggleWebWorkers(): void {
        if (this.enableWebWorkers?.checked) {
            this.initializeWebWorker();
        } else {
            if (this.faceDetectionWorker) {
                this.faceDetectionWorker.terminate();
                this.faceDetectionWorker = null;
                this.workerInitialized = false;
                this.addToProcessingLog('Web Worker disabled', 'info');
            }
        }
    }

    async detectFacesWithWorker(image: HTMLImageElement, imageId: string): Promise<any[]> {
        return new Promise(async (resolve, reject) => {
            if (!this.workerInitialized) {
                reject(new Error('Worker not initialized'));
                return;
            }

            try {
                // Create ImageBitmap for zero-copy transfer
                const bitmap = await createImageBitmap(image);

                // Store callback for this request
                const requestId = `${imageId}_${Date.now()}`;
                if (!this.workerCallbacks) {
                    this.workerCallbacks = new Map();
                }
                this.workerCallbacks.set(requestId, { resolve, reject });

                // Send to worker using transferable ImageBitmap
                this.faceDetectionWorker!.postMessage({
                    type: 'detectFaces',
                    id: requestId,
                    data: {
                        imageBitmap: bitmap,
                        options: { includeQuality: true }
                    }
                }, [bitmap]);  // Transfer ownership to worker

                // Set timeout
                setTimeout(() => {
                    if (this.workerCallbacks!.has(requestId)) {
                        this.workerCallbacks!.delete(requestId);
                        reject(new Error('Worker detection timeout'));
                    }
                }, 30000); // 30 second timeout
            } catch (error) {
                reject(error);
            }
        });
    }

    handleWorkerDetectionResult(requestId: string, faces: any[]): void {
        if (this.workerCallbacks && this.workerCallbacks.has(requestId)) {
            const callback = this.workerCallbacks.get(requestId);
            if (callback) {
                const { resolve } = callback;
                this.workerCallbacks.delete(requestId);
                resolve(faces);
            }
        }
    }

    // Enhanced Face Detection with Retry Logic
    async detectFacesWithQualityProduction(image: HTMLImageElement, imageId: string): Promise<any[]> {
        const maxRetries = parseInt(this.retryAttempts?.value || '0') || 0;
        let lastError = null;

        for (let attempt = 0; attempt <= maxRetries; attempt++) {
            try {
                if (attempt > 0) {
                    this.addToProcessingLog(`Retry attempt ${attempt} for ${imageId}`, 'warning');
                    await this.delay(1000 * attempt); // Progressive delay
                }

                let processedImage = image;

                // Apply reduced resolution if enabled
                if (this.reducedResolution?.checked) {
                    processedImage = await this.createReducedResolutionImage(image);
                }

                let faces;

                if (this.enableWebWorkers?.checked && this.workerInitialized) {
                    faces = await this.detectFacesWithWorker(processedImage, imageId);
                } else {
                    faces = await this.detectFacesWithQuality(processedImage);
                }

                // If we used reduced resolution, scale back the coordinates
                if (this.reducedResolution?.checked) {
                    const scale = image.width / processedImage.width;
                    faces = faces.map(face => ({
                        ...face,
                        x: (face.x || 0) * scale,
                        y: (face.y || 0) * scale,
                        width: (face.width || 0) * scale,
                        height: (face.height || 0) * scale
                    }));
                }

                return faces;

            } catch (error: unknown) {
                lastError = error;
                this.addToDetailedErrorLog(`Detection attempt ${attempt + 1} failed for ${imageId}`, (error as Error).message, 'error');

                if (attempt === maxRetries) {
                    throw new Error(`Face detection failed after ${maxRetries + 1} attempts: ${(error as Error).message}`);
                }
            }
        }

        // Fallback return (should never reach here due to throw above)
        return [];
    }

    async createReducedResolutionImage(image: HTMLImageElement): Promise<HTMLImageElement> {
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');

        canvas.width = Math.floor(image.width * 0.5);
        canvas.height = Math.floor(image.height * 0.5);

        ctx!.drawImage(image, 0, 0, canvas.width, canvas.height);

        return new Promise((resolve) => {
            const reducedImage = new Image();
            reducedImage.onload = () => resolve(reducedImage);
            reducedImage.src = canvas.toDataURL();
        });
    }

    delay(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    // Enhanced UI Helper Methods
    updateOffsetDisplays(): void {
        this.updateOffsetDisplay('vertical');
        this.updateOffsetDisplay('horizontal');
    }

    findMatchingPreset(width: number, height: number): string {
        const presets = {
            linkedin: { width: 400, height: 400 },
            passport: { width: 413, height: 531 },
            instagram: { width: 1080, height: 1080 },
            idcard: { width: 332, height: 498 },
            avatar: { width: 512, height: 512 },
            headshot: { width: 600, height: 800 }
        };

        for (const [name, dimensions] of Object.entries(presets)) {
            if (dimensions.width === width && dimensions.height === height) {
                return name;
            }
        }

        return 'custom';
    }

    updateAspectRatioDisplay(): void {
        const width = parseInt(this.outputWidth.value);
        const height = parseInt(this.outputHeight.value);
        const ratio = width / height;

        let ratioText = `${ratio.toFixed(2)}:1`;
        if (Math.abs(ratio - 1) < 0.01) ratioText = '1:1 (Square)';
        else if (Math.abs(ratio - 4/3) < 0.01) ratioText = '4:3 (Standard)';
        else if (Math.abs(ratio - 16/9) < 0.01) ratioText = '16:9 (Widescreen)';
        else if (Math.abs(ratio - 3/4) < 0.01) ratioText = '3:4 (Portrait)';

        const aspectRatioElement = document.getElementById('aspectRatioText');
        if (aspectRatioElement) {
            aspectRatioElement.textContent = `${ratioText} ratio`;
        }
        this.currentAspectRatio = ratio;
    }

    // ============================================================================
    // DOWNLOAD AND FILE MANAGEMENT METHODS
    // ============================================================================

    // Download all results as ZIP archive
    async downloadResultsAsZip(results: any[]): Promise<void> {
        if (results.length === 0) {
            this.updateStatus('No results to download');
            return;
        }

        try {
            this.updateStatus('Creating ZIP archive...');

            // @ts-ignore - JSZip is loaded via CDN
            const zip = new JSZip();

            results.forEach((result: any, index: number) => {
                // Convert data URL to binary data
                const base64Data = result.dataUrl.split(',')[1];
                zip.file(result.filename, base64Data, { base64: true });
            });

            const zipBlob = await zip.generateAsync({ type: 'blob' });
            const zipUrl = URL.createObjectURL(zipBlob);

            const link = document.createElement('a');
            link.download = `cropped_faces_${new Date().toISOString().slice(0, 10)}.zip`;
            link.href = zipUrl;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);

            URL.revokeObjectURL(zipUrl);

            this.updateStatus(`Downloaded ${results.length} cropped faces as ZIP archive!`);
        } catch (error: unknown) {
            console.error('Error creating ZIP:', (error as Error));
            this.updateStatus(`Error creating ZIP: ${(error as Error).message}`);
        }
    }

    // Download individual face from current image
    async downloadIndividualFace(faceId: string, imageData: any): Promise<void> {
        const face = imageData.faces?.find((f: any) => f.id === faceId);
        if (!face) return;

        try {
            this.updateStatus('Cropping individual face...');

            // Crop just this face
            const result = await this.cropSingleFace(imageData, face);

            // Download directly
            const link = document.createElement('a');
            link.download = result.filename;
            link.href = result.dataUrl;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);

            this.updateStatus('Individual face downloaded!');
        } catch (error: unknown) {
            console.error('Error downloading individual face:', (error as Error));
            this.updateStatus(`Error downloading face: ${(error as Error).message}`);
        }
    }

    // Crop a single face from image data
    async cropSingleFace(imageData: any, face: any): Promise<any> {
        const outputWidth = parseInt(this.outputWidth.value);
        const outputHeight = parseInt(this.outputHeight.value);
        const faceHeightPct = parseInt(this.faceHeightPct.value) / 100;

        // Create temporary canvas for processing
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        tempCanvas.width = imageData.image.width;
        tempCanvas.height = imageData.image.height;
        tempCtx!.drawImage(imageData.image, 0, 0);

        // Create crop canvas
        const cropCanvas = document.createElement('canvas');
        const cropCtx = cropCanvas.getContext('2d');
        cropCanvas.width = outputWidth;
        cropCanvas.height = outputHeight;

        // Calculate crop parameters
        const targetFaceHeight = outputHeight * faceHeightPct;
        const scale = targetFaceHeight / face.height;
        const cropWidthSrc = outputWidth / scale;
        const cropHeightSrc = outputHeight / scale;

        // Calculate face position based on positioning mode
        const { cropX, cropY } = this.calculateSmartCropPosition(
            face, cropWidthSrc, cropHeightSrc, imageData.image.width, imageData.image.height
        );

        const finalCropWidth = Math.min(cropWidthSrc, imageData.image.width - cropX);
        const finalCropHeight = Math.min(cropHeightSrc, imageData.image.height - cropY);

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
        const filename = this.generateBatchFilename(imageData.file.name, face.index, outputWidth, outputHeight);

        return {
            dataUrl: croppedDataUrl,
            filename: filename,
            faceId: face.id
        };
    }

    // ============================================================================
    // FACE SELECTION AND COUNTER METHODS
    // ============================================================================

    // Toggle selection of a specific face
    toggleFaceSelection(faceId: string, imageData?: any): void {
        if (imageData && imageData.faces) {
            const face = imageData.faces.find((f: any) => f.id === faceId);
            if (face) {
                face.selected = !face.selected;
                this.updateFaceCounter();
                this.refreshFaceDisplay(imageData);
            }
        }
    }

    // Update the face counter display
    updateFaceCounter(): void {
        // This method should be overridden by subclasses or can work with generic face arrays
        if (this.faceCount && this.selectedFaceCount) {
            // Subclasses should implement their specific logic
        }
    }

    // Select all faces in current image
    selectAllFaces(imageData?: any): void {
        if (imageData && imageData.faces) {
            imageData.faces.forEach((face: any) => {
                face.selected = true;
            });
            this.updateFaceCounter();
            this.refreshFaceDisplay(imageData);
        }
    }

    // Deselect all faces in current image
    selectNoneFaces(imageData?: any): void {
        if (imageData && imageData.faces) {
            imageData.faces.forEach((face: any) => {
                face.selected = false;
            });
            this.updateFaceCounter();
            this.refreshFaceDisplay(imageData);
        }
    }

    // Refresh face display (override in subclasses)
    protected refreshFaceDisplay(imageData?: any): void {
        // To be implemented by subclasses
    }

    // Refresh image display (override in subclasses)
    protected refreshImageDisplay(): void {
        // To be implemented by subclasses
    }

    // ============================================================================
    // UNDO/REDO FUNCTIONALITY
    // ============================================================================

    // Save current state for undo
    saveState(): void {
        if (!this.images) return;

        const state = {
            images: new Map(),
            currentImageIndex: this.currentImageIndex,
            currentFaceIndex: this.currentFaceIndex
        };

        // Deep copy the images state
        for (const [id, imageData] of this.images) {
            state.images.set(id, {
                ...imageData,
                faces: imageData.faces ? imageData.faces.map((face: any) => ({ ...face })) : [],
                results: [...imageData.results]
            });
        }

        this.undoStack.push(state);

        // Limit undo stack size
        if (this.undoStack.length > 50) {
            this.undoStack.shift();
        }

        // Clear redo stack when new action is performed
        this.redoStack = [];
    }

    // Undo last action
    undo(): void {
        if (this.undoStack.length === 0) {
            this.updateStatus('Nothing to undo');
            return;
        }

        // Save current state to redo stack
        this.saveCurrentStateToRedo();

        // Restore previous state
        const previousState = this.undoStack.pop();
        this.images = previousState.images;
        this.currentImageIndex = previousState.currentImageIndex;
        this.currentFaceIndex = previousState.currentFaceIndex;

        this.refreshImageDisplay();
        this.updateStatus('Undid last action');
    }

    // Redo last undone action
    redo(): void {
        if (this.redoStack.length === 0) {
            this.updateStatus('Nothing to redo');
            return;
        }

        // Save current state to undo stack
        this.saveState();

        // Restore next state
        const nextState = this.redoStack.pop();
        this.images = nextState.images;
        this.currentImageIndex = nextState.currentImageIndex;
        this.currentFaceIndex = nextState.currentFaceIndex;

        this.refreshImageDisplay();
        this.updateStatus('Redid last action');
    }

    // Save current state to redo stack
    protected saveCurrentStateToRedo(): void {
        if (!this.images) return;

        const state = {
            images: new Map(),
            currentImageIndex: this.currentImageIndex,
            currentFaceIndex: this.currentFaceIndex
        };

        // Deep copy the images state
        for (const [id, imageData] of this.images) {
            state.images.set(id, {
                ...imageData,
                faces: imageData.faces ? imageData.faces.map((face: any) => ({ ...face })) : [],
                results: [...imageData.results]
            });
        }

        this.redoStack.push(state);

        // Limit redo stack size
        if (this.redoStack.length > 50) {
            this.redoStack.shift();
        }
    }

    // ============================================================================
    // KEYBOARD SHORTCUTS
    // ============================================================================

    setupKeyboardShortcuts(): void {
        document.addEventListener('keydown', (e) => {
            // Don't handle shortcuts when typing in input fields
            const target = e.target as HTMLElement;
            if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')) {
                return;
            }

            this.handleKeyPress(e);
        });
    }

    // Handle key press - can be overridden by subclasses
    protected handleKeyPress(e: KeyboardEvent): void {
        switch (e.key) {
            case 'z': // Ctrl+Z - Undo
                if (e.ctrlKey && !e.shiftKey) {
                    e.preventDefault();
                    this.undo();
                }
                break;

            case 'y': // Ctrl+Y - Redo
                if (e.ctrlKey) {
                    e.preventDefault();
                    this.redo();
                }
                break;

            case 'Z': // Ctrl+Shift+Z - Redo (alternative)
                if (e.ctrlKey && e.shiftKey) {
                    e.preventDefault();
                    this.redo();
                }
                break;

            case 'Escape': // Clear selection/cancel
                e.preventDefault();
                this.handleEscapeKey();
                break;

            // Subclasses can add more shortcuts by overriding this method
        }
    }

    // Handle escape key - can be overridden
    protected handleEscapeKey(): void {
        // To be implemented by subclasses
    }

    // ============================================================================
    // NAVIGATION METHODS
    // ============================================================================

    // Navigate between faces
    navigateFace(direction: number): void {
        // To be implemented by subclasses with their specific image structure
    }

    // Navigate between images
    navigateImage(direction: number): void {
        // To be implemented by subclasses with their specific image structure
    }

    // Highlight current face (for keyboard navigation)
    protected highlightCurrentFace(): void {
        // To be implemented by subclasses
    }

    // ============================================================================
    // THEME/DARK MODE
    // ============================================================================

    loadThemePreference(): void {
        const savedTheme = localStorage.getItem('faceCropperTheme');
        if (savedTheme === 'dark') {
            this.isDarkMode = true;
            document.documentElement.setAttribute('data-theme', 'dark');
        }
    }

    toggleDarkMode(): void {
        this.isDarkMode = !this.isDarkMode;

        if (this.isDarkMode) {
            document.documentElement.setAttribute('data-theme', 'dark');
            localStorage.setItem('faceCropperTheme', 'dark');
        } else {
            document.documentElement.removeAttribute('data-theme');
            localStorage.removeItem('faceCropperTheme');
        }

        this.updateDarkModeButton();
    }

    protected updateDarkModeButton(): void {
        // To be implemented by subclasses if they have a dark mode button
    }

    // ============================================================================
    // DRAG AND DROP
    // ============================================================================

    setupDragAndDrop(dropZone: HTMLElement, fileInputCallback: (files: File[]) => void): void {
        ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
            dropZone.addEventListener(eventName, (e) => {
                e.preventDefault();
                e.stopPropagation();
            });
        });

        ['dragenter', 'dragover'].forEach(eventName => {
            dropZone.addEventListener(eventName, () => {
                dropZone.classList.add('drag-over');
            });
        });

        ['dragleave', 'drop'].forEach(eventName => {
            dropZone.addEventListener(eventName, () => {
                dropZone.classList.remove('drag-over');
            });
        });

        dropZone.addEventListener('drop', (e: DragEvent) => {
            const files = Array.from(e.dataTransfer?.files || []);
            const imageFiles = files.filter(file => file.type.startsWith('image/'));

            if (imageFiles.length > 0) {
                fileInputCallback(imageFiles);
            }
        });
    }

    // ============================================================================
    // IMAGE ENHANCEMENT METHODS
    // ============================================================================

    /**
     * Apply image enhancements to an entire image (for batch processing)
     */
    async applyImageEnhancements(image: HTMLImageElement): Promise<HTMLImageElement> {
        // Create canvas for processing
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        canvas.width = image.width;
        canvas.height = image.height;
        ctx!.drawImage(image, 0, 0);

        // Apply all enhancements
        await this.applyEnhancements(ctx!, canvas);

        // Return enhanced image
        return new Promise((resolve) => {
            const enhancedImg = new Image();
            enhancedImg.onload = () => resolve(enhancedImg);
            enhancedImg.src = canvas.toDataURL();
        });
    }

    /**
     * Apply auto color correction using histogram stretching
     */
    applyAutoColorCorrection(imageData: ImageData): ImageData {
        const data = imageData.data;
        const length = data.length;

        // Calculate histogram for each channel
        const rHist = new Array(256).fill(0);
        const gHist = new Array(256).fill(0);
        const bHist = new Array(256).fill(0);

        for (let i = 0; i < length; i += 4) {
            rHist[data[i]]++;
            gHist[data[i + 1]]++;
            bHist[data[i + 2]]++;
        }

        // Calculate cumulative distribution
        const totalPixels = length / 4;
        const getCumulativeValue = (hist: number[], targetPercentile: number) => {
            let cumulative = 0;
            for (let i = 0; i < 256; i++) {
                cumulative += hist[i];
                if (cumulative / totalPixels >= targetPercentile) {
                    return i;
                }
            }
            return 255;
        };

        // Get 1% and 99% percentiles for each channel
        const rMin = getCumulativeValue(rHist, 0.01);
        const rMax = getCumulativeValue(rHist, 0.99);
        const gMin = getCumulativeValue(gHist, 0.01);
        const gMax = getCumulativeValue(gHist, 0.99);
        const bMin = getCumulativeValue(bHist, 0.01);
        const bMax = getCumulativeValue(bHist, 0.99);

        // Apply histogram stretching
        for (let i = 0; i < length; i += 4) {
            data[i] = Math.min(255, Math.max(0, ((data[i] - rMin) / (rMax - rMin)) * 255));
            data[i + 1] = Math.min(255, Math.max(0, ((data[i + 1] - gMin) / (gMax - gMin)) * 255));
            data[i + 2] = Math.min(255, Math.max(0, ((data[i + 2] - bMin) / (bMax - bMin)) * 255));
        }

        return imageData;
    }

    /**
     * Apply exposure adjustment
     */
    applyExposureAdjustment(imageData: ImageData, exposure: number): ImageData {
        const data = imageData.data;
        const exposureFactor = Math.pow(2, exposure);

        for (let i = 0; i < data.length; i += 4) {
            data[i] = Math.min(255, data[i] * exposureFactor);
            data[i + 1] = Math.min(255, data[i + 1] * exposureFactor);
            data[i + 2] = Math.min(255, data[i + 2] * exposureFactor);
        }

        return imageData;
    }

    /**
     * Apply contrast adjustment
     */
    applyContrastAdjustment(imageData: ImageData, contrast: number): ImageData {
        const data = imageData.data;
        const contrastFactor = contrast;

        for (let i = 0; i < data.length; i += 4) {
            data[i] = Math.min(255, Math.max(0, ((data[i] - 128) * contrastFactor) + 128));
            data[i + 1] = Math.min(255, Math.max(0, ((data[i + 1] - 128) * contrastFactor) + 128));
            data[i + 2] = Math.min(255, Math.max(0, ((data[i + 2] - 128) * contrastFactor) + 128));
        }

        return imageData;
    }

    /**
     * Apply sharpness using unsharp mask
     */
    applySharpness(imageData: ImageData, amount: number): ImageData {
        const data = imageData.data;
        const width = imageData.width;
        const height = imageData.height;
        const outputData = new Uint8ClampedArray(data);

        // Unsharp mask kernel
        const kernel = [
            0, -amount, 0,
            -amount, 1 + 4 * amount, -amount,
            0, -amount, 0
        ];

        for (let y = 1; y < height - 1; y++) {
            for (let x = 1; x < width - 1; x++) {
                for (let c = 0; c < 3; c++) {
                    let sum = 0;
                    for (let ky = -1; ky <= 1; ky++) {
                        for (let kx = -1; kx <= 1; kx++) {
                            const idx = ((y + ky) * width + (x + kx)) * 4 + c;
                            sum += data[idx] * kernel[(ky + 1) * 3 + (kx + 1)];
                        }
                    }
                    const outputIdx = (y * width + x) * 4 + c;
                    outputData[outputIdx] = Math.min(255, Math.max(0, sum));
                }
            }
        }

        return new ImageData(outputData, width, height);
    }

    /**
     * Apply skin smoothing using selective blur on skin tone regions
     */
    async applySkinSmoothing(imageData: ImageData, amount: number): Promise<ImageData> {
        // Simple gaussian blur on skin tone regions
        const data = imageData.data;
        const width = imageData.width;
        const height = imageData.height;

        // Create skin mask based on color range
        const skinMask = new Uint8Array(width * height);
        for (let i = 0; i < data.length; i += 4) {
            const r = data[i];
            const g = data[i + 1];
            const b = data[i + 2];

            // Simple skin tone detection
            const isSkin = (r > 60 && g > 40 && b > 20 && r > b && r > g * 0.8 && (r - g) > 15);
            skinMask[Math.floor(i / 4)] = isSkin ? 1 : 0;
        }

        // Apply blur only to skin regions
        const blurRadius = Math.round(amount);
        if (blurRadius > 0) {
            return this.applySelectiveBlur(imageData, skinMask, blurRadius);
        }

        return imageData;
    }

    /**
     * Apply selective blur based on mask
     */
    applySelectiveBlur(imageData: ImageData, mask: Uint8Array, radius: number): ImageData {
        const data = imageData.data;
        const width = imageData.width;
        const height = imageData.height;
        const outputData = new Uint8ClampedArray(data);

        for (let y = 0; y < height; y++) {
            for (let x = 0; x < width; x++) {
                const idx = y * width + x;
                if (mask[idx]) {
                    let r = 0, g = 0, b = 0, count = 0;

                    for (let dy = -radius; dy <= radius; dy++) {
                        for (let dx = -radius; dx <= radius; dx++) {
                            const ny = y + dy;
                            const nx = x + dx;
                            if (ny >= 0 && ny < height && nx >= 0 && nx < width) {
                                const nidx = (ny * width + nx) * 4;
                                r += data[nidx];
                                g += data[nidx + 1];
                                b += data[nidx + 2];
                                count++;
                            }
                        }
                    }

                    const pixelIdx = idx * 4;
                    outputData[pixelIdx] = r / count;
                    outputData[pixelIdx + 1] = g / count;
                    outputData[pixelIdx + 2] = b / count;
                }
            }
        }

        return new ImageData(outputData, width, height);
    }

    /**
     * Apply red-eye removal
     */
    async applyRedEyeRemoval(imageData: ImageData): Promise<ImageData> {
        // Simple red-eye detection and correction
        const data = imageData.data;

        for (let i = 0; i < data.length; i += 4) {
            const r = data[i];
            const g = data[i + 1];
            const b = data[i + 2];

            // Detect red-eye: high red, low green/blue
            if (r > 150 && r > g * 2 && r > b * 2) {
                // Replace with more natural color
                data[i] = Math.min(r * 0.7, g * 1.2);
                data[i + 1] = g;
                data[i + 2] = Math.max(b, g * 0.8);
            }
        }

        return imageData;
    }

    /**
     * Apply background blur (simplified implementation)
     */
    async applyBackgroundBlur(imageData: ImageData, blurAmount: number): Promise<ImageData> {
        // This is a simplified background blur - in a real application,
        // you'd want to use proper segmentation
        const data = imageData.data;
        const width = imageData.width;
        const height = imageData.height;

        // Create a simple edge-based mask (assuming faces are in center)
        const centerX = width / 2;
        const centerY = height / 2;
        const maxDistance = Math.sqrt(centerX * centerX + centerY * centerY);

        const backgroundMask = new Uint8Array(width * height);
        for (let y = 0; y < height; y++) {
            for (let x = 0; x < width; x++) {
                const distance = Math.sqrt((x - centerX) ** 2 + (y - centerY) ** 2);
                const isBackground = distance > maxDistance * 0.4;
                backgroundMask[y * width + x] = isBackground ? 1 : 0;
            }
        }

        return this.applySelectiveBlur(imageData, backgroundMask, Math.round(blurAmount));
    }
}
