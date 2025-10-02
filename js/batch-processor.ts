import { BaseFaceCropper } from './base-face-cropper.js';
import type {
    ImageData as TypedImageData,
    FaceData,
    CropResult,
    Statistics as TypedStatistics,
    BoundingBox
} from './types.js';

interface Face {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  selected: boolean;
  score?: number;
  landmarks?: Array<{ x: number; y: number }>;
}

interface ProcessingResult {
  faceId: string;
  blobUrl: string;
  fileName: string;
  format: 'png' | 'jpeg' | 'webp';
  width: number;
  height: number;
}

interface ImageEntry {
  id: string;
  file: File;
  image: HTMLImageElement | null;
  faces: Face[];
  results: ProcessingResult[];
  selected: boolean;
  processed: boolean;
  status: 'loaded' | 'processing' | 'processed' | 'error';
  enhancedImage?: HTMLImageElement | null;
  processedAt?: number;
  memoryCleanedUp?: boolean;
  page?: number;
}

type LogLevel = 'info' | 'success' | 'warning' | 'error';
type ErrorSeverity = 'error' | 'critical' | 'warning' | 'info';

interface ProcessingLogEntry {
  timestamp: string;
  message: string;
  type: LogLevel;
}

interface ErrorLogEntry {
  timestamp: string;
  title: string;
  details: string;
  severity: ErrorSeverity;
}

interface Statistics {
  totalFacesDetected: number;
  imagesProcessed: number;
  successfulProcessing: number;
  processingTimes: number[];
  startTime: number | null;
}
//#endregion

class FaceCropper extends BaseFaceCropper {
    // State properties
    imageResults!: Map<string, any>;
    processingQueue!: string[];
    isProcessing!: boolean;
    currentProcessingId!: string | null;
    currentImageIndex!: number;
    currentFaceIndex!: number;
    undoStack!: Array<any>;
    redoStack!: Array<any>;
    savedSettings!: Map<string, any>;
    recentSettings!: string[];
    faceDetectionWorker!: Worker | null;
    workerInitialized!: boolean;
    memoryUsage!: { images: number; processed: number };
    galleryPage!: number;
    galleryPageSize!: number;
    imageLoadQueue!: Array<{ files: File[]; page: number }>;
    isLoadingImages!: boolean;
    globalSettings!: any;

    // DOM Elements
    imageInput!: HTMLInputElement;
    processAllBtn!: HTMLButtonElement;
    processSelectedBtn!: HTMLButtonElement;
    clearAllBtn!: HTMLButtonElement;
    downloadAllBtn!: HTMLButtonElement;
    progressSection!: HTMLElement;
    progressFill!: HTMLElement;
    progressText!: HTMLElement;
    imageGallery!: HTMLElement;
    galleryGrid!: HTMLElement;
    selectAllBtn!: HTMLButtonElement;
    selectNoneBtn!: HTMLButtonElement;
    selectedCount!: HTMLElement;
    totalCount!: HTMLElement;
    canvasContainer!: HTMLElement;
    inputCanvas!: HTMLCanvasElement;
    outputCanvas!: HTMLCanvasElement;
    faceOverlays!: HTMLElement;
    faceCount!: HTMLElement;
    selectedFaceCount!: HTMLElement;
    croppedContainer!: HTMLElement;
    status!: HTMLElement;
    selectAllFacesBtn!: HTMLButtonElement;
    selectNoneFacesBtn!: HTMLButtonElement;
    detectFacesBtn!: HTMLButtonElement;
    outputWidth!: HTMLInputElement;
    outputHeight!: HTMLInputElement;
    faceHeightPct!: HTMLInputElement;
    previewText!: HTMLElement;
    sizePreset!: HTMLSelectElement;
    aspectRatioLock!: HTMLButtonElement;
    positioningMode!: HTMLSelectElement;
    verticalOffset!: HTMLInputElement;
    horizontalOffset!: HTMLInputElement;
    verticalOffsetValue!: HTMLElement;
    horizontalOffsetValue!: HTMLElement;
    advancedPositioning!: HTMLElement;
    applyToAllBtn!: HTMLButtonElement;
    resetSettingsBtn!: HTMLButtonElement;
    aspectRatioText!: HTMLElement;
    aspectRatioLocked!: boolean;
    outputFormat!: HTMLSelectElement;
    jpegQuality!: HTMLInputElement;
    jpegQualityGroup!: HTMLElement;
    qualityValue!: HTMLElement;
    namingTemplate!: HTMLInputElement;
    zipDownload!: HTMLInputElement;
    individualDownload!: HTMLInputElement;
    darkModeBtn!: HTMLButtonElement;
    canvasWrapper!: HTMLElement;
    originalPanel!: HTMLElement;
    processedPanel!: HTMLElement;
    autoColorCorrection!: HTMLInputElement;
    exposureAdjustment!: HTMLInputElement;
    exposureValue!: HTMLElement;
    contrastAdjustment!: HTMLInputElement;
    contrastValue!: HTMLElement;
    sharpnessControl!: HTMLInputElement;
    sharpnessValue!: HTMLElement;
    skinSmoothing!: HTMLInputElement;
    skinSmoothingValue!: HTMLElement;
    redEyeRemoval!: HTMLInputElement;
    backgroundBlur!: HTMLInputElement;
    backgroundBlurValue!: HTMLElement;
    previewEnhancementsBtn!: HTMLButtonElement;
    resetEnhancementsBtn!: HTMLButtonElement;
    applyToAllImagesBtn!: HTMLButtonElement;
    enhancementSummary!: HTMLElement;
    totalFacesDetected!: HTMLElement;
    successRate!: HTMLElement;
    avgProcessingTime!: HTMLElement;
    imagesProcessed!: HTMLElement;
    recentSettingsDropdown!: HTMLSelectElement;
    loadRecentBtn!: HTMLButtonElement;
    settingsName!: HTMLInputElement;
    saveSettingsBtn!: HTMLButtonElement;
    exportSettingsBtn!: HTMLButtonElement;
    importSettingsBtn!: HTMLButtonElement;
    importSettingsFile!: HTMLInputElement;
    exportLogBtn!: HTMLButtonElement;
    exportCsvBtn!: HTMLButtonElement;
    clearStatsBtn!: HTMLButtonElement;
    continueOnError!: HTMLInputElement;
    reducedResolution!: HTMLInputElement;
    enableWebWorkers!: HTMLInputElement;
    memoryManagement!: HTMLSelectElement;
    retryAttempts!: HTMLInputElement;
    errorLogElement!: HTMLElement;
    clearErrorsBtn!: HTMLButtonElement;
    exportErrorsBtn!: HTMLButtonElement;
    ctx!: CanvasRenderingContext2D;
    workerCallbacks?: Map<string, { resolve: (faces: Face[]) => void; reject: (e: Error) => void }>;

    constructor() {
        super();
        this.images = new Map(); // imageId -> { file, image, faces, results, selected, processed }
        this.imageResults = new Map(); // Store results from streamed processing
        this.processingQueue = [];
        this.isProcessing = false;
        this.currentProcessingId = null;

        // UI state
        this.currentImageIndex = 0;
        this.currentFaceIndex = 0;
        this.undoStack = [];
        this.redoStack = [];

        this.processingLog = [];
        this.savedSettings = new Map();
        this.recentSettings = [];

        // Production optimization state
        this.errorLog = [];
        this.faceDetectionWorker = null;
        this.workerInitialized = false;
        this.memoryUsage = { images: 0, processed: 0 };
        this.galleryPage = 0;
        this.galleryPageSize = 20;
        this.imageLoadQueue = [];
        this.isLoadingImages = false;

        this.initializeElements();
        this.setupEventListeners();
        this.setupKeyboardShortcuts();
        this.loadModel();
    }

    initializeElements() {
        this.imageInput = document.getElementById('imageInput')! as HTMLInputElement;
        this.processAllBtn = document.getElementById('processAllBtn')! as HTMLButtonElement;
        this.processSelectedBtn = document.getElementById('processSelectedBtn')! as HTMLButtonElement;
        this.clearAllBtn = document.getElementById('clearAllBtn')! as HTMLButtonElement;
        this.downloadAllBtn = document.getElementById('downloadAllBtn')! as HTMLButtonElement;

        this.progressSection = document.getElementById('progressSection')!;
        this.progressFill = document.getElementById('progressFill')!;
        this.progressText = document.getElementById('progressText')!;

        this.imageGallery = document.getElementById('imageGallery')!;
        this.galleryGrid = document.getElementById('galleryGrid')!;
        this.selectAllBtn = document.getElementById('selectAllBtn')! as HTMLButtonElement;
        this.selectNoneBtn = document.getElementById('selectNoneBtn')! as HTMLButtonElement;
        this.selectedCount = document.getElementById('selectedCount')!;
        this.totalCount = document.getElementById('totalCount')!;

        this.canvasContainer = document.getElementById('canvasContainer')!;
        this.inputCanvas = document.getElementById('inputCanvas')! as HTMLCanvasElement;
        this.outputCanvas = document.getElementById('outputCanvas')! as HTMLCanvasElement;
        this.faceOverlays = document.getElementById('faceOverlays')!;
        this.faceCount = document.getElementById('faceCount')!;
        this.selectedFaceCount = document.getElementById('selectedFaceCount')!;
        this.croppedContainer = document.getElementById('croppedContainer')!;
        this.status = document.getElementById('status')!;

        // Face selection controls
        this.selectAllFacesBtn = document.getElementById('selectAllFacesBtn')! as HTMLButtonElement;
        this.selectNoneFacesBtn = document.getElementById('selectNoneFacesBtn')! as HTMLButtonElement;
        this.detectFacesBtn = document.getElementById('detectFacesBtn')! as HTMLButtonElement;

        // Crop settings elements
        this.outputWidth = document.getElementById('outputWidth')! as HTMLInputElement;
        this.outputHeight = document.getElementById('outputHeight')! as HTMLInputElement;
        this.faceHeightPct = document.getElementById('faceHeightPct')! as HTMLInputElement;
        this.previewText = document.getElementById('previewText')!;

        // Smart cropping elements
        this.sizePreset = document.getElementById('sizePreset')! as HTMLSelectElement;
        this.aspectRatioLock = document.getElementById('aspectRatioLock')! as HTMLButtonElement;
        this.positioningMode = document.getElementById('positioningMode')! as HTMLSelectElement;
        this.verticalOffset = document.getElementById('verticalOffset')! as HTMLInputElement;
        this.horizontalOffset = document.getElementById('horizontalOffset')! as HTMLInputElement;
        this.verticalOffsetValue = document.getElementById('verticalOffsetValue')!;
        this.horizontalOffsetValue = document.getElementById('horizontalOffsetValue')!;
        this.advancedPositioning = document.getElementById('advancedPositioning')!;
        this.applyToAllBtn = document.getElementById('applyToAllBtn')! as HTMLButtonElement;
        this.resetSettingsBtn = document.getElementById('resetSettingsBtn')! as HTMLButtonElement;
        this.aspectRatioText = document.getElementById('aspectRatioText')!;

        // Output settings elements
        this.outputFormat = document.getElementById('outputFormat')! as HTMLSelectElement;
        this.jpegQuality = document.getElementById('jpegQuality')! as HTMLInputElement;
        this.jpegQualityGroup = document.getElementById('jpegQualityGroup')!;
        this.qualityValue = document.getElementById('qualityValue')!;
        this.namingTemplate = document.getElementById('namingTemplate')! as HTMLInputElement;
        this.zipDownload = document.getElementById('zipDownload')! as HTMLInputElement;
        this.individualDownload = document.getElementById('individualDownload')! as HTMLInputElement;

        // UI enhancement elements
        this.darkModeBtn = document.getElementById('darkModeBtn')! as HTMLButtonElement;
        this.canvasWrapper = document.getElementById('canvasWrapper')!;
        this.originalPanel = document.getElementById('originalPanel')!;
        this.processedPanel = document.getElementById('processedPanel')!;

        // Preprocessing elements
        this.autoColorCorrection = document.getElementById('autoColorCorrection')! as HTMLInputElement;
        this.exposureAdjustment = document.getElementById('exposureAdjustment')! as HTMLInputElement;
        this.exposureValue = document.getElementById('exposureValue')!;
        this.contrastAdjustment = document.getElementById('contrastAdjustment')! as HTMLInputElement;
        this.contrastValue = document.getElementById('contrastValue')!;
        this.sharpnessControl = document.getElementById('sharpnessControl')! as HTMLInputElement;
        this.sharpnessValue = document.getElementById('sharpnessValue')!;
        this.skinSmoothing = document.getElementById('skinSmoothing')! as HTMLInputElement;
        this.skinSmoothingValue = document.getElementById('skinSmoothingValue')!;
        this.redEyeRemoval = document.getElementById('redEyeRemoval')! as HTMLInputElement;
        this.backgroundBlur = document.getElementById('backgroundBlur')! as HTMLInputElement;
        this.backgroundBlurValue = document.getElementById('backgroundBlurValue')!;
        this.previewEnhancementsBtn = document.getElementById('previewEnhancementsBtn')! as HTMLButtonElement;
        this.resetEnhancementsBtn = document.getElementById('resetEnhancementsBtn')! as HTMLButtonElement;
        this.applyToAllImagesBtn = document.getElementById('applyToAllImagesBtn')! as HTMLButtonElement;
        this.enhancementSummary = document.getElementById('enhancementSummary')!;

        // Workflow tools elements
        this.totalFacesDetected = document.getElementById('totalFacesDetected')!;
        this.successRate = document.getElementById('successRate')!;
        this.avgProcessingTime = document.getElementById('avgProcessingTime')!;
        this.imagesProcessed = document.getElementById('imagesProcessed')!;
        this.recentSettingsDropdown = document.getElementById('recentSettingsDropdown')! as HTMLSelectElement;
        this.loadRecentBtn = document.getElementById('loadRecentBtn')! as HTMLButtonElement;
        this.settingsName = document.getElementById('settingsName')! as HTMLInputElement;
        this.saveSettingsBtn = document.getElementById('saveSettingsBtn')! as HTMLButtonElement;
        this.exportSettingsBtn = document.getElementById('exportSettingsBtn')! as HTMLButtonElement;
        this.importSettingsBtn = document.getElementById('importSettingsBtn')! as HTMLButtonElement;
        this.importSettingsFile = document.getElementById('importSettingsFile')! as HTMLInputElement;
        this.exportLogBtn = document.getElementById('exportLogBtn')! as HTMLButtonElement;
        this.exportCsvBtn = document.getElementById('exportCsvBtn')! as HTMLButtonElement;
        this.clearStatsBtn = document.getElementById('clearStatsBtn')! as HTMLButtonElement;
        this.processingLogElement = document.getElementById('processingLog')!;

        // Production optimization elements
        this.continueOnError = document.getElementById('continueOnError')! as HTMLInputElement;
        this.reducedResolution = document.getElementById('reducedResolution')! as HTMLInputElement;
        this.enableWebWorkers = document.getElementById('enableWebWorkers')! as HTMLInputElement;
        this.memoryManagement = document.getElementById('memoryManagement')! as HTMLSelectElement;
        this.retryAttempts = document.getElementById('retryAttempts')! as HTMLInputElement;
        this.errorLogElement = document.getElementById('errorLog')!;
        this.clearErrorsBtn = document.getElementById('clearErrorsBtn')! as HTMLButtonElement;
        this.exportErrorsBtn = document.getElementById('exportErrorsBtn')! as HTMLButtonElement;

        this.ctx = this.inputCanvas.getContext('2d')!;
    }

    setupEventListeners() {
        this.imageInput.addEventListener('change', (e) => this.handleMultipleImageUploadProduction(e));
        this.processAllBtn.addEventListener('click', () => this.processAll());
        this.processSelectedBtn.addEventListener('click', () => this.processSelected());
        this.clearAllBtn.addEventListener('click', () => this.clearAll());
        this.downloadAllBtn.addEventListener('click', () => this.downloadAllResults());

        this.selectAllBtn.addEventListener('click', () => this.selectAll());
        this.selectNoneBtn.addEventListener('click', () => this.selectNone());

        // Face selection listeners
        this.selectAllFacesBtn.addEventListener('click', () => this.selectAllFaces());
        this.selectNoneFacesBtn.addEventListener('click', () => this.selectNoneFaces());
        this.detectFacesBtn.addEventListener('click', () => this.detectCurrentImageFaces());

        // Crop settings listeners
        this.outputWidth.addEventListener('input', () => this.updatePreview());
        this.outputHeight.addEventListener('input', () => this.updatePreview());
        this.faceHeightPct.addEventListener('input', () => this.updatePreview());

        // Output settings listeners
        this.outputFormat.addEventListener('change', () => this.updateFormatControls());
        this.jpegQuality.addEventListener('input', () => this.updateQualityDisplay());
        this.individualDownload.addEventListener('change', () => this.toggleIndividualDownloadButtons());

        // Smart cropping listeners
        this.sizePreset.addEventListener('change', () => this.applyPreset());
        this.aspectRatioLock.addEventListener('click', () => this.toggleAspectRatioLock());
        this.outputWidth.addEventListener('input', () => this.handleDimensionChange('width'));
        this.outputHeight.addEventListener('input', () => this.handleDimensionChange('height'));
        this.positioningMode.addEventListener('change', () => this.updatePositioningControls());
        this.verticalOffset.addEventListener('input', () => this.updateOffsetDisplay('vertical'));
        this.horizontalOffset.addEventListener('input', () => this.updateOffsetDisplay('horizontal'));
        this.applyToAllBtn.addEventListener('click', () => this.applySettingsToAll());
        this.resetSettingsBtn.addEventListener('click', () => this.resetToDefaults());

        // UI enhancement listeners
        this.darkModeBtn.addEventListener('click', () => this.toggleDarkMode());

        // Navigation listeners
        const singleImageModeBtn = document.getElementById('singleImageModeBtn');
        if (singleImageModeBtn) {
            singleImageModeBtn.addEventListener('click', () => {
                window.location.href = 'single-processing.html';
            });
        }

        const csvBatchModeBtn = document.getElementById('csvBatchModeBtn');
        if (csvBatchModeBtn) {
            csvBatchModeBtn.addEventListener('click', () => {
                window.location.href = 'csv-processing.html';
            });
        }

        // Preprocessing listeners
        this.autoColorCorrection.addEventListener('change', () => this.updateEnhancementSummary());
        this.exposureAdjustment.addEventListener('input', () => this.updateSliderValue('exposure'));
        this.contrastAdjustment.addEventListener('input', () => this.updateSliderValue('contrast'));
        this.sharpnessControl.addEventListener('input', () => this.updateSliderValue('sharpness'));
        this.skinSmoothing.addEventListener('input', () => this.updateSliderValue('skinSmoothing'));
        this.redEyeRemoval.addEventListener('change', () => this.updateEnhancementSummary());
        this.backgroundBlur.addEventListener('input', () => this.updateSliderValue('backgroundBlur'));
        this.previewEnhancementsBtn.addEventListener('click', () => this.previewEnhancements());
        this.resetEnhancementsBtn.addEventListener('click', () => this.resetEnhancements());
        this.applyToAllImagesBtn.addEventListener('click', () => this.applyEnhancementsToAll());

        // Workflow tools listeners
        this.loadRecentBtn.addEventListener('click', () => this.loadRecentSettings());
        this.saveSettingsBtn.addEventListener('click', () => this.saveCurrentSettings());
        this.exportSettingsBtn.addEventListener('click', () => this.exportSettingsToJSON());
        this.importSettingsBtn.addEventListener('click', () => this.importSettingsFile.click());
        this.importSettingsFile.addEventListener('change', (e) => this.importSettingsFromJSON(e));
        this.exportLogBtn.addEventListener('click', () => this.exportProcessingLog());
        this.exportCsvBtn.addEventListener('click', () => this.exportCSVReport());
        this.clearStatsBtn.addEventListener('click', () => this.clearStatistics());

        // Production optimization listeners
        this.enableWebWorkers.addEventListener('change', () => this.toggleWebWorkers());
        this.memoryManagement.addEventListener('change', () => this.updateMemorySettings());
        this.clearErrorsBtn.addEventListener('click', () => this.clearErrorLog());
        this.exportErrorsBtn.addEventListener('click', () => this.exportErrorLog());

        // Initialize controls
        this.updatePreview();
        this.updateFormatControls();
        this.updateQualityDisplay();
        this.updatePositioningControls();
        this.updateOffsetDisplays();
        this.setupDragAndDrop();
        this.setupTooltips();
        this.setupCollapsiblePanels();
        this.loadThemePreference();
        this.updateAllSliderValues();
        this.updateEnhancementSummary();
        this.loadSavedSettings();
        this.updateStatisticsDisplay();
        this.initializeWebWorker();
        this.createMemoryIndicator();
        this.setupErrorLogCollapsible();
    }

    setupKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Don't handle shortcuts when typing in input fields
            const target = e.target as HTMLElement;
            if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')) {
                return;
            }

            const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
            const currentImage = selectedImages.length > 0 ? selectedImages[0] : null;

            switch (e.key) {
                case ' ': // Space - Toggle current face selection
                    e.preventDefault();
                    if (currentImage && currentImage!.faces && currentImage!.faces.length > 0) {
                        this.toggleCurrentFaceSelection();
                    }
                    break;

                case 'ArrowLeft': // Navigate to previous face
                    e.preventDefault();
                    this.navigateFace(-1);
                    break;

                case 'ArrowRight': // Navigate to next face
                    e.preventDefault();
                    this.navigateFace(1);
                    break;

                case 'ArrowUp': // Navigate to previous image
                    e.preventDefault();
                    this.navigateImage(-1);
                    break;

                case 'ArrowDown': // Navigate to next image
                    e.preventDefault();
                    this.navigateImage(1);
                    break;

                case 'Enter': // Process selected
                    e.preventDefault();
                    if (!this.isProcessing) {
                        this.processSelected();
                    }
                    break;

                case 'Delete': // Remove current image
                    e.preventDefault();
                    if (currentImage) {
                        this.removeCurrentImage();
                    }
                    break;

                case 'a': // Ctrl+A - Select all faces
                    if (e.ctrlKey) {
                        e.preventDefault();
                        this.selectAllFaces();
                    }
                    break;

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
                    this.clearFaceSelection();
                    break;
            }
        });
    }

    toggleCurrentFaceSelection() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (!currentImage.faces || currentImage.faces.length === 0) return;

        const face = currentImage.faces[this.currentFaceIndex];
        if (face) {
            this.saveState(); // Save state for undo
            face.selected = !face.selected;
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
            this.highlightCurrentFace();
        }
    }

    navigateFace(direction: number) {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (!currentImage.faces || currentImage.faces.length === 0) return;

        this.currentFaceIndex += direction;
        if (this.currentFaceIndex < 0) {
            this.currentFaceIndex = currentImage.faces.length - 1;
        } else if (this.currentFaceIndex >= currentImage.faces.length) {
            this.currentFaceIndex = 0;
        }

        this.highlightCurrentFace();
    }

    navigateImage(direction: number) {
        const imageArray = Array.from(this.images!.values());
        if (imageArray.length === 0) return;

        this.currentImageIndex += direction;
        if (this.currentImageIndex < 0) {
            this.currentImageIndex = imageArray.length - 1;
        } else if (this.currentImageIndex >= imageArray.length) {
            this.currentImageIndex = 0;
        }

        // Select the new current image
        imageArray.forEach((img, index) => {
            img.selected = index === this.currentImageIndex;
        });

        this.currentFaceIndex = 0; // Reset face index
        this.updateGallery();
        this.updateControls();

        // Display the new current image if it has faces
        const currentImage = imageArray[this.currentImageIndex];
        if (currentImage.faces && currentImage.faces.length > 0) {
            this.displayImageWithFaceOverlays(currentImage);
            this.highlightCurrentFace();
        }
    }

    removeCurrentImage() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        this.saveState(); // Save state for undo

        // Clean up object URL
        if (currentImage.image.src.startsWith('blob:')) {
            URL.revokeObjectURL(currentImage.image.src);
        }

        this.images!.delete(currentImage.id);
        this.updateGallery();
        this.updateControls();

        // Navigate to next image if available
        const remainingImages = Array.from(this.images!.values());
        if (remainingImages.length > 0) {
            this.currentImageIndex = Math.min(this.currentImageIndex, remainingImages.length - 1);
            remainingImages[this.currentImageIndex].selected = true;
            this.updateGallery();
        } else {
            this.canvasContainer.classList.add('hidden');
            this.imageGallery.classList.add('hidden');
        }

        this.updateStatus(`Removed image. ${this.images!.size} images remaining.`, 'success');
    }

    highlightCurrentFace() {
        // Remove existing highlights
        document.querySelectorAll('.face-box').forEach(box => {
            box.classList.remove('keyboard-selected');
        });

        // Highlight current face
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (!currentImage.faces || currentImage.faces.length === 0) return;

        const currentFace = currentImage.faces[this.currentFaceIndex];
        if (currentFace) {
            const faceBox = document.querySelector(`[data-face-id="${currentFace.id}"]`);
            if (faceBox) {
                faceBox.classList.add('keyboard-selected');
                faceBox.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            }
        }
    }

    clearFaceSelection() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            this.saveState(); // Save state for undo
            currentImage.faces.forEach(face => face.selected = false);
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
        }
    }

    saveState() {
        // Save current state for undo functionality
        const state = {
            images: new Map(),
            currentImageIndex: this.currentImageIndex,
            currentFaceIndex: this.currentFaceIndex
        };

        // Deep copy the images state
        for (const [id, imageData] of this.images!) {
            state.images.set(id, {
                ...imageData,
                faces: imageData.faces ? imageData.faces.map(face => ({ ...face })) : [],
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

    undo() {
        if (this.undoStack.length === 0) {
            this.updateStatus('Nothing to undo', 'info');
            return;
        }

        // Save current state to redo stack
        this.saveCurrentStateToRedo();

        // Restore previous state
        const previousState = this.undoStack.pop();
        this.images = previousState.images;
        this.currentImageIndex = previousState.currentImageIndex;
        this.currentFaceIndex = previousState.currentFaceIndex;

        this.updateGallery();
        this.updateControls();

        // Refresh display if there's a selected image
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length > 0 && selectedImages[0].faces) {
            this.displayImageWithFaceOverlays(selectedImages[0]);
            this.highlightCurrentFace();
        }

        this.updateStatus('Undid last action', 'success');
    }

    redo() {
        if (this.redoStack.length === 0) {
            this.updateStatus('Nothing to redo', 'info');
            return;
        }

        // Save current state to undo stack
        this.saveState();

        // Restore next state
        const nextState = this.redoStack.pop();
        this.images = nextState.images;
        this.currentImageIndex = nextState.currentImageIndex;
        this.currentFaceIndex = nextState.currentFaceIndex;

        this.updateGallery();
        this.updateControls();

        // Refresh display if there's a selected image
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length > 0 && selectedImages[0].faces) {
            this.displayImageWithFaceOverlays(selectedImages[0]);
            this.highlightCurrentFace();
        }

        this.updateStatus('Redid last action', 'success');
    }

    saveCurrentStateToRedo() {
        const state = {
            images: new Map(),
            currentImageIndex: this.currentImageIndex,
            currentFaceIndex: this.currentFaceIndex
        };

        // Deep copy the images state
        for (const [id, imageData] of this.images!) {
            state.images.set(id, {
                ...imageData,
                faces: imageData.faces ? imageData.faces.map(face => ({ ...face })) : [],
                results: [...imageData.results]
            });
        }

        this.redoStack.push(state);

        // Limit redo stack size
        if (this.redoStack.length > 50) {
            this.redoStack.shift();
        }
    }



    loadThemePreference() {
        const savedTheme = localStorage.getItem('faceCropperTheme');
        if (savedTheme === 'dark') {
            this.isDarkMode = true;
            document.documentElement.setAttribute('data-theme', 'dark');
        }

        const icon = this.isDarkMode ? '☀️' : '🌙';
        const tooltip = this.isDarkMode ? 'Switch to light mode' : 'Switch to dark mode';
        this.darkModeBtn.textContent = icon;
        this.darkModeBtn.setAttribute('aria-label', tooltip);
        this.darkModeBtn.setAttribute('title', tooltip);
        this.darkModeBtn.setAttribute('aria-pressed', this.isDarkMode.toString());
    }


    setupDragAndDrop() {
        const uploadCard = document.querySelector('.upload-card');
        if (!uploadCard) {
            return;
        }

        const dragMessage = document.createElement('div');
        dragMessage.className = 'drag-message';
        dragMessage.textContent = 'Drop your images here!';
        uploadCard.appendChild(dragMessage);

        ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
            uploadCard.addEventListener(eventName, this.preventDefaults, false);
            document.body.addEventListener(eventName, this.preventDefaults, false);
        });

        ['dragenter', 'dragover'].forEach(eventName => {
            uploadCard.addEventListener(eventName, () => {
                uploadCard.classList.add('drag-over');
            }, false);
        });

        ['dragleave', 'drop'].forEach(eventName => {
            uploadCard.addEventListener(eventName, () => {
                uploadCard.classList.remove('drag-over');
            }, false);
        });

        uploadCard.addEventListener('drop', (e: Event) => {
            const de = e as DragEvent;
            const dt = de.dataTransfer;
            const files = dt?.files;
            if (files) {
                this.handleDroppedFiles(files);
            }
        }, false);
    }

    preventDefaults(e: Event) {
        e.preventDefault();
        e.stopPropagation();
    }

    async handleDroppedFiles(files: FileList) {
        const fileArray = Array.from(files).filter((file: File) => file.type.startsWith('image/'));

        if (fileArray.length === 0) {
            this.updateStatus('Please drop image files only', 'error');
            return;
        }

        this.updateStatus(`Loading ${fileArray.length} dropped images...`, 'loading');

        for (const file of fileArray) {
            const imageId = this.generateImageId();

            try {
                const image = await this.loadImageFromFile(file);
                this.images!.set(imageId, {
                    id: imageId,
                    file: file as File,
                    image: image as HTMLImageElement,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    status: 'loaded'
                });
            } catch (error: unknown) {
                console.error('Error loading dropped image:', (file as File).name, (error as Error));
            }
        }

        this.updateGallery();
        this.updateControls();
        this.imageGallery.classList.remove('hidden');
        this.updateStatus(`Loaded ${fileArray.length} images via drag & drop!`, 'success');
    }

    setupTooltips() {
        const tooltipData = {
            'processAllBtn': 'Process all images in the gallery (Enter)',
            'processSelectedBtn': 'Process only selected images',
            'clearAllBtn': 'Clear all images from the workspace',
            'downloadAllBtn': 'Download all processed faces',
            'darkModeBtn': 'Switch between light and dark themes',
            'outputWidth': 'Set the width of cropped face images',
            'outputHeight': 'Set the height of cropped face images',
            'faceHeightPct': 'Percentage of output height the face should occupy',
            'sizePreset': 'Quick size presets for common use cases',
            'aspectRatioLock': 'Lock aspect ratio when changing dimensions',
            'positioningMode': 'How to position the face in the crop area',
            'outputFormat': 'Image format for saved faces',
            'jpegQuality': 'Quality setting for JPEG images (1-100)',
            'namingTemplate': 'Template for naming output files',
            'zipDownload': 'Package all results in a ZIP file',
            'individualDownload': 'Show download buttons for each face',
            'selectAllFacesBtn': 'Select all detected faces (Ctrl+A)',
            'selectNoneFacesBtn': 'Deselect all faces (Escape)',
            'detectFacesBtn': 'Detect faces in the current image',
            'autoColorCorrection': 'Automatically adjust brightness and contrast',
            'exposureAdjustment': 'Adjust image brightness (-2 to +2 stops)',
            'contrastAdjustment': 'Adjust image contrast (0.5 to 2.0)',
            'sharpnessControl': 'Apply unsharp mask filter (0 to 2)',
            'skinSmoothing': 'Smooth skin tones with selective blur (0 to 5)',
            'redEyeRemoval': 'Detect and correct red-eye effect',
            'backgroundBlur': 'Blur background around faces (0-10px)',
            'previewEnhancementsBtn': 'Preview enhancements on selected image',
            'resetEnhancementsBtn': 'Reset all enhancements to defaults',
            'applyToAllImagesBtn': 'Apply current enhancement settings to all images',
            'recentSettingsDropdown': 'Load previously saved configurations',
            'loadRecentBtn': 'Load the selected configuration',
            'settingsName': 'Enter a name for this configuration',
            'saveSettingsBtn': 'Save current settings for later use',
            'exportSettingsBtn': 'Export settings as JSON file',
            'importSettingsBtn': 'Import settings from JSON file',
            'exportLogBtn': 'Export detailed processing log',
            'exportCsvBtn': 'Export processing statistics as CSV',
            'clearStatsBtn': 'Clear all statistics and logs'
        };

        Object.entries(tooltipData).forEach(([id, text]) => {
            const element = document.getElementById(id);
            if (element) {
                this.addTooltip(element, text);
            }
        });

        // Add keyboard shortcuts help
        this.createKeyboardHelp();
    }

    addTooltip(element: HTMLElement, text: string) {
        element.classList.add('tooltip');

        const tooltipText = document.createElement('span');
        tooltipText.className = 'tooltiptext';
        tooltipText.textContent = text;

        element.appendChild(tooltipText);
    }

    createKeyboardHelp() {
        const helpDiv = document.createElement('div');
        helpDiv.className = 'keyboard-help';
        helpDiv.innerHTML = `
            <h4>Keyboard Shortcuts</h4>
            <div class="shortcut">
                <span>Toggle face selection</span>
                <span class="key">Space</span>
            </div>
            <div class="shortcut">
                <span>Navigate faces</span>
                <span class="key">← →</span>
            </div>
            <div class="shortcut">
                <span>Navigate images</span>
                <span class="key">↑ ↓</span>
            </div>
            <div class="shortcut">
                <span>Process selected</span>
                <span class="key">Enter</span>
            </div>
            <div class="shortcut">
                <span>Remove image</span>
                <span class="key">Delete</span>
            </div>
            <div class="shortcut">
                <span>Select all faces</span>
                <span class="key">Ctrl+A</span>
            </div>
            <div class="shortcut">
                <span>Undo / Redo</span>
                <span class="key">Ctrl+Z/Y</span>
            </div>
            <div class="shortcut">
                <span>Clear selection</span>
                <span class="key">Esc</span>
            </div>
        `;

        document.body.appendChild(helpDiv);

        // Show/hide on ? key
        document.addEventListener('keydown', (e) => {
            const target = e.target as HTMLElement;
            if (e.key === '?' && target && target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
                e.preventDefault();
                helpDiv.classList.toggle('show');
            }
        });
    }

    setupCollapsiblePanels() {
        const headers = document.querySelectorAll('.collapsible-header');

        headers.forEach(header => {
            header.addEventListener('click', () => {
                const content = header.nextElementSibling;
                const isCollapsed = header.classList.contains('collapsed');

                if (isCollapsed) {
                    // Expand
                    header.classList.remove('collapsed');
                    content!.classList.remove('collapsed');
                } else {
                    // Collapse
                    header.classList.add('collapsed');
                    content!.classList.add('collapsed');
                }
            });
        });
    }

    // Preprocessing Methods
    updateSliderValue(type: string) {
        switch (type) {
            case 'exposure':
                this.exposureValue.textContent = parseFloat(this.exposureAdjustment.value).toFixed(1);
                break;
            case 'contrast':
                this.contrastValue.textContent = parseFloat(this.contrastAdjustment.value).toFixed(1);
                break;
            case 'sharpness':
                this.sharpnessValue.textContent = parseFloat(this.sharpnessControl.value).toFixed(1);
                break;
            case 'skinSmoothing':
                this.skinSmoothingValue.textContent = parseFloat(this.skinSmoothing.value).toFixed(1);
                break;
            case 'backgroundBlur':
                this.backgroundBlurValue.textContent = parseFloat(this.backgroundBlur.value).toFixed(1) + 'px';
                break;
        }
        this.updateEnhancementSummary();
    }

    updateAllSliderValues() {
        this.updateSliderValue('exposure');
        this.updateSliderValue('contrast');
        this.updateSliderValue('sharpness');
        this.updateSliderValue('skinSmoothing');
        this.updateSliderValue('backgroundBlur');
    }

    updateEnhancementSummary() {
        const enhancements = [];

        if (this.autoColorCorrection.checked) {
            enhancements.push('Auto Color');
        }
        if (parseFloat(this.exposureAdjustment.value) !== 0) {
            enhancements.push(`Exposure ${this.exposureAdjustment.value}`);
        }
        if (parseFloat(this.contrastAdjustment.value) !== 1) {
            enhancements.push(`Contrast ${this.contrastAdjustment.value}`);
        }
        if (parseFloat(this.sharpnessControl.value) !== 0) {
            enhancements.push(`Sharpness ${this.sharpnessControl.value}`);
        }
        if (parseFloat(this.skinSmoothing.value) !== 0) {
            enhancements.push(`Skin Smoothing ${this.skinSmoothing.value}`);
        }
        if (this.redEyeRemoval.checked) {
            enhancements.push('Red-eye Removal');
        }
        if (parseFloat(this.backgroundBlur.value) !== 0) {
            enhancements.push(`Background Blur ${this.backgroundBlur.value}px`);
        }

        this.enhancementSummary.textContent = enhancements.length > 0
            ? enhancements.join(', ')
            : 'No enhancements applied';
    }

    resetEnhancements() {
        this.autoColorCorrection.checked = true;
        this.exposureAdjustment.value = String(0);
        this.contrastAdjustment.value = String(1);
        this.sharpnessControl.value = String(0);
        this.skinSmoothing.value = String(0);
        this.redEyeRemoval.checked = false;
        this.backgroundBlur.value = String(0);

        this.updateAllSliderValues();
        this.updateEnhancementSummary();
        this.updateStatus('Enhancements reset to defaults', 'success');
    }

    async previewEnhancements() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) {
            this.updateStatus('Please select an image to preview enhancements', 'error');
            return;
        }

        const currentImage = selectedImages[0];
        this.updateStatus('Applying enhancements for preview...', 'loading');

        try {
            const enhancedImage = await this.applyImageEnhancements(currentImage.image);
            this.displayEnhancedPreview(enhancedImage, currentImage);
            this.updateStatus('Enhancement preview applied!', 'success');
        } catch (error: unknown) {
            console.error('Error applying enhancements:', (error as Error));
            this.updateStatus(`Error applying enhancements: ${(error as Error).message}`, 'error');
        }
    }

    async applyEnhancementsToAll() {
        const allImages = Array.from(this.images!.values());
        if (allImages.length === 0) {
            this.updateStatus('No images to enhance', 'error');
            return;
        }

        this.updateStatus('Applying enhancements to all images...', 'loading');

        try {
            for (const imageData of allImages) {
                const imgEntry = imageData as unknown as ImageEntry;
                imgEntry.enhancedImage = await this.applyImageEnhancements(imgEntry.image!);
            }
            this.updateStatus(`Applied enhancements to ${allImages.length} images!`, 'success');
        } catch (error: unknown) {
            console.error('Error applying enhancements to all images:', (error as Error));
            this.updateStatus(`Error applying enhancements: ${(error as Error).message}`, 'error');
        }
    }

    async applyImageEnhancements(image: HTMLImageElement): Promise<HTMLImageElement> {
        // Create canvas for processing
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        canvas.width = image.width;
        canvas.height = image.height;
        ctx!.drawImage(image, 0, 0);

        // Get image data
        let imageData = ctx!.getImageData(0, 0, canvas.width, canvas.height);
        const data = imageData.data;

        // Apply auto color correction
        if (this.autoColorCorrection.checked) {
            imageData = this.applyAutoColorCorrection(imageData);
        }

        // Apply exposure adjustment
        const exposure = parseFloat(this.exposureAdjustment.value);
        if (exposure !== 0) {
            imageData = this.applyExposureAdjustment(imageData, exposure);
        }

        // Apply contrast adjustment
        const contrast = parseFloat(this.contrastAdjustment.value);
        if (contrast !== 1) {
            imageData = this.applyContrastAdjustment(imageData, contrast);
        }

        // Apply sharpness
        const sharpness = parseFloat(this.sharpnessControl.value);
        if (sharpness > 0) {
            imageData = this.applySharpness(imageData, sharpness);
        }

        // Apply skin smoothing
        const skinSmoothingAmount = parseFloat(this.skinSmoothing.value);
        if (skinSmoothingAmount > 0) {
            imageData = await this.applySkinSmoothing(imageData, skinSmoothingAmount);
        }

        // Apply red-eye removal
        if (this.redEyeRemoval.checked) {
            imageData = await this.applyRedEyeRemoval(imageData);
        }

        // Apply background blur
        const blurAmount = parseFloat(this.backgroundBlur.value);
        if (blurAmount > 0) {
            imageData = await this.applyBackgroundBlur(imageData, blurAmount);
        }

        // Put enhanced data back to canvas
        ctx!.putImageData(imageData, 0, 0);

        // Return enhanced image
        return new Promise((resolve) => {
            const enhancedImg = new Image();
            enhancedImg.onload = () => resolve(enhancedImg);
            enhancedImg.src = canvas.toDataURL();
        });
    }

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

    displayEnhancedPreview(enhancedImage: HTMLCanvasElement | HTMLImageElement, originalImageData: any) {
        // Store the enhanced image in the image data
        originalImageData.enhancedImage = enhancedImage;
        // Display with face overlays
        this.displayImageWithFaceOverlays(originalImageData);
    }

    // Workflow Tools Methods
    addToProcessingLog(message: string, type: string = 'info') {
        const timestamp = new Date().toLocaleTimeString();
        const entry = {
            timestamp,
            message,
            type
        };

        this.processingLog!.push(entry);

        // Keep only last 100 entries
        if (this.processingLog!.length > 100) {
            this.processingLog!.shift();
        }

        // Update display
        this.updateProcessingLogDisplay();
    }

    updateProcessingLogDisplay() {
        const logHtml = this.processingLog!
            .slice(-10) // Show last 10 entries
            .map(entry => `
                <div class="log-entry ${entry.type}">
                    <span class="log-timestamp">${entry.timestamp}</span>
                    ${entry.message}
                </div>
            `)
            .join('');

        this.processingLogElement!.innerHTML = logHtml || '<div class="log-entry">No processing activity yet...</div>';

        // Auto-scroll to bottom
        this.processingLogElement!.scrollTop = this.processingLogElement!.scrollHeight;
    }

    updateStatisticsDisplay() {
        this.totalFacesDetected.textContent = String(this.statistics.totalFacesDetected);
        this.imagesProcessed.textContent = String(this.statistics.imagesProcessed);

        // Calculate success rate
        const successRate = this.statistics.imagesProcessed > 0
            ? (this.statistics.successfulProcessing / this.statistics.imagesProcessed * 100).toFixed(1)
            : 0;
        this.successRate.textContent = `${successRate}%`;

        // Calculate average processing time
        const avgTime = this.statistics.processingTimes.length > 0
            ? (this.statistics.processingTimes.reduce((a, b) => a + b, 0) / this.statistics.processingTimes.length).toFixed(0)
            : 0;
        this.avgProcessingTime.textContent = `${avgTime}ms`;
    }

    recordProcessingStart() {
        this.statistics.startTime = Date.now();
    }

    recordProcessingEnd(success: boolean, facesDetected: number = 0) {
        if (this.statistics.startTime) {
            const processingTime = Date.now() - this.statistics.startTime;
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

        this.updateStatisticsDisplay();
    }

    clearStatistics() {
        this.statistics = {
            totalFacesDetected: 0,
            imagesProcessed: 0,
            successfulProcessing: 0,
            processingTimes: [],
            startTime: null
        };

        this.processingLog = [];
        this.updateStatisticsDisplay();
        this.updateProcessingLogDisplay();
        this.addToProcessingLog('Statistics cleared', 'info');
        this.updateStatus('Statistics cleared successfully', 'success');
    }

    getCurrentSettings() {
        return {
            // Smart cropping settings
            outputWidth: parseInt(this.outputWidth.value),
            outputHeight: parseInt(this.outputHeight.value),
            faceHeightPct: parseInt(this.faceHeightPct.value),
            sizePreset: this.sizePreset.value,
            positioningMode: this.positioningMode.value,
            verticalOffset: parseInt(this.verticalOffset.value),
            horizontalOffset: parseInt(this.horizontalOffset.value),
            aspectRatioLocked: this.aspectRatioLocked,

            // Output settings
            outputFormat: this.outputFormat.value,
            jpegQuality: parseInt(this.jpegQuality.value),
            namingTemplate: this.namingTemplate.value,
            zipDownload: this.zipDownload.checked,
            individualDownload: this.individualDownload.checked,

            // Preprocessing settings
            autoColorCorrection: this.autoColorCorrection.checked,
            exposureAdjustment: parseFloat(this.exposureAdjustment.value),
            contrastAdjustment: parseFloat(this.contrastAdjustment.value),
            sharpnessControl: parseFloat(this.sharpnessControl.value),
            skinSmoothing: parseFloat(this.skinSmoothing.value),
            redEyeRemoval: this.redEyeRemoval.checked,
            backgroundBlur: parseFloat(this.backgroundBlur.value)
        };
    }

    applySettings(settings: any) {
        // Smart cropping settings
        this.outputWidth.value = settings.outputWidth || 256;
        this.outputHeight.value = settings.outputHeight || 256;
        this.faceHeightPct.value = settings.faceHeightPct || 70;
        this.sizePreset.value = settings.sizePreset || 'custom';
        this.positioningMode.value = settings.positioningMode || 'center';
        this.verticalOffset.value = settings.verticalOffset || 0;
        this.horizontalOffset.value = settings.horizontalOffset || 0;
        this.aspectRatioLocked = settings.aspectRatioLocked || false;

        // Output settings
        this.outputFormat.value = settings.outputFormat || 'png';
        this.jpegQuality.value = settings.jpegQuality || 85;
        this.namingTemplate.value = settings.namingTemplate || 'face_{original}_{index}';
        this.zipDownload.checked = settings.zipDownload !== undefined ? settings.zipDownload : true;
        this.individualDownload.checked = settings.individualDownload || false;

        // Preprocessing settings
        this.autoColorCorrection.checked = settings.autoColorCorrection !== undefined ? settings.autoColorCorrection : true;
        this.exposureAdjustment.value = settings.exposureAdjustment || 0;
        this.contrastAdjustment.value = settings.contrastAdjustment || 1;
        this.sharpnessControl.value = settings.sharpnessControl || 0;
        this.skinSmoothing.value = settings.skinSmoothing || 0;
        this.redEyeRemoval.checked = settings.redEyeRemoval || false;
        this.backgroundBlur.value = settings.backgroundBlur || 0;

        // Update UI
        this.updatePreview();
        this.updateFormatControls();
        this.updateQualityDisplay();
        this.updatePositioningControls();
        this.updateOffsetDisplays();
        this.updateAllSliderValues();
        this.updateEnhancementSummary();

        // Update aspect ratio lock UI
        if (this.aspectRatioLocked) {
            this.aspectRatioLock.classList.add('locked');
            this.aspectRatioLock.textContent = '🔒';
            this.aspectRatioLock.title = 'Unlock aspect ratio';
        } else {
            this.aspectRatioLock.classList.remove('locked');
            this.aspectRatioLock.textContent = '🔓';
            this.aspectRatioLock.title = 'Lock aspect ratio';
        }
    }

    saveCurrentSettings() {
        const settingsName = this.settingsName.value.trim();
        if (!settingsName) {
            this.updateStatus('Please enter a configuration name', 'error');
            return;
        }

        const settings: any = this.getCurrentSettings();
        settings.name = settingsName;
        settings.timestamp = new Date().toISOString();

        // Save to local storage
        this.savedSettings.set(settingsName, settings);
        this.addToRecentSettings(settingsName);

        // Update localStorage
        localStorage.setItem('faceCropperSettings', JSON.stringify([...this.savedSettings]));

        this.updateRecentSettingsDropdown();
        this.settingsName.value = '';
        this.addToProcessingLog(`Saved settings: ${settingsName}`, 'success');
        this.updateStatus(`Settings saved as "${settingsName}"`, 'success');
    }

    loadRecentSettings() {
        const selectedName = this.recentSettingsDropdown.value;
        if (!selectedName) {
            this.updateStatus('Please select a configuration to load', 'error');
            return;
        }

        const settings = this.savedSettings.get(selectedName);
        if (settings) {
            this.applySettings(settings);
            this.addToRecentSettings(selectedName);
            this.addToProcessingLog(`Loaded settings: ${selectedName}`, 'success');
            this.updateStatus(`Loaded settings: "${selectedName}"`, 'success');
        } else {
            this.updateStatus('Configuration not found', 'error');
        }
    }

    addToRecentSettings(settingsName: string) {
        // Remove if already exists
        this.recentSettings = this.recentSettings.filter(name => name !== settingsName);
        // Add to beginning
        this.recentSettings.unshift(settingsName);
        // Keep only last 10
        this.recentSettings = this.recentSettings.slice(0, 10);

        localStorage.setItem('faceCropperRecentSettings', JSON.stringify(this.recentSettings));
        this.updateRecentSettingsDropdown();
    }

    updateRecentSettingsDropdown() {
        const dropdown = this.recentSettingsDropdown;
        dropdown.innerHTML = '<option value="">Select a saved configuration...</option>';

        this.recentSettings.forEach(settingsName => {
            if (this.savedSettings.has(settingsName)) {
                const option = document.createElement('option');
                option.value = settingsName;
                option.textContent = settingsName;
                dropdown.appendChild(option);
            }
        });
    }

    loadSavedSettings() {
        try {
            // Load saved settings
            const saved = localStorage.getItem('faceCropperSettings');
            if (saved) {
                const settingsArray = JSON.parse(saved);
                this.savedSettings = new Map(settingsArray);
            }

            // Load recent settings
            const recent = localStorage.getItem('faceCropperRecentSettings');
            if (recent) {
                this.recentSettings = JSON.parse(recent);
            }

            this.updateRecentSettingsDropdown();
        } catch (error: unknown) {
            console.error('Error loading saved settings:', (error as Error));
        }
    }

    exportSettingsToJSON() {
        const settings: any = this.getCurrentSettings();
        settings.exportedAt = new Date().toISOString();
        settings.version = '1.0';

        const blob = new Blob([JSON.stringify(settings, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const link = document.createElement('a');
        link.download = `face-cropper-settings-${new Date().toISOString().slice(0, 10)}.json`;
        link.href = url;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);

        URL.revokeObjectURL(url);
        this.addToProcessingLog('Settings exported to JSON', 'success');
        this.updateStatus('Settings exported successfully', 'success');
    }

    async importSettingsFromJSON(event: Event) {
        const target = event.target as HTMLInputElement;
        const file = target?.files?.[0];
        if (!file) return;

        try {
            const text = await file.text();
            const settings = JSON.parse(text);

            this.applySettings(settings);
            this.addToProcessingLog(`Settings imported from ${file.name}`, 'success');
            this.updateStatus('Settings imported successfully', 'success');
        } catch (error: unknown) {
            console.error('Error importing settings:', (error as Error));
            this.addToProcessingLog(`Failed to import settings: ${(error as Error).message}`, 'error');
            this.updateStatus('Failed to import settings. Please check the file format.', 'error');
        }

        // Reset file input
        target.value = '';
    }

    exportProcessingLog() {
        const logData = {
            exportedAt: new Date().toISOString(),
            statistics: this.statistics,
            settings: this.getCurrentSettings(),
            log: this.processingLog
        };

        const blob = new Blob([JSON.stringify(logData, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const link = document.createElement('a');
        link.download = `face-cropper-log-${new Date().toISOString().slice(0, 10)}.json`;
        link.href = url;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);

        URL.revokeObjectURL(url);
        this.addToProcessingLog('Processing log exported', 'success');
        this.updateStatus('Processing log exported successfully', 'success');
    }

    exportCSVReport() {
        const csvData = [];
        const headers = [
            'Timestamp',
            'Images Processed',
            'Total Faces Detected',
            'Success Rate (%)',
            'Avg Processing Time (ms)',
            'Output Format',
            'Output Dimensions',
            'Face Height %',
            'Positioning Mode',
            'Auto Color Correction',
            'Exposure Adjustment',
            'Contrast Adjustment',
            'Sharpness',
            'Skin Smoothing',
            'Red-eye Removal',
            'Background Blur'
        ];

        csvData.push(headers);

        const settings = this.getCurrentSettings();
        const successRate = this.statistics.imagesProcessed > 0
            ? (this.statistics.successfulProcessing / this.statistics.imagesProcessed * 100).toFixed(1)
            : 0;
        const avgTime = this.statistics.processingTimes.length > 0
            ? (this.statistics.processingTimes.reduce((a, b) => a + b, 0) / this.statistics.processingTimes.length).toFixed(0)
            : 0;

        const row = [
            new Date().toISOString(),
            this.statistics.imagesProcessed,
            this.statistics.totalFacesDetected,
            successRate,
            avgTime,
            settings.outputFormat.toUpperCase(),
            `${settings.outputWidth}×${settings.outputHeight}`,
            settings.faceHeightPct,
            settings.positioningMode,
            settings.autoColorCorrection ? 'Yes' : 'No',
            settings.exposureAdjustment,
            settings.contrastAdjustment,
            settings.sharpnessControl,
            settings.skinSmoothing,
            settings.redEyeRemoval ? 'Yes' : 'No',
            settings.backgroundBlur
        ];

        csvData.push(row);

        // Convert to CSV string
        const csvString = csvData.map(row =>
            row.map(cell => `"${cell}"`).join(',')
        ).join('\n');

        const blob = new Blob([csvString], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);

        const link = document.createElement('a');
        link.download = `face-cropper-report-${new Date().toISOString().slice(0, 10)}.csv`;
        link.href = url;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);

        URL.revokeObjectURL(url);
        this.addToProcessingLog('CSV report exported', 'success');
        this.updateStatus('CSV report exported successfully', 'success');
    }

    // Production Optimization Methods
    async initializeWebWorker() {
        if (!this.enableWebWorkers.checked) return;

        try {
            this.faceDetectionWorker = new Worker('js/face-detection-worker.js');

            this.faceDetectionWorker.onmessage = (e) => {
                const { type, data, id, success, faces, error } = e.data;

                switch (type) {
                    case 'initialized':
                        this.workerInitialized = success;
                        if (success) {
                            this.addToProcessingLog('Web Worker initialized successfully', 'success');
                        } else {
                            this.addToDetailedErrorLog('Web Worker initialization failed', (error as Error).message, 'critical');
                            this.enableWebWorkers.checked = false;
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
                this.enableWebWorkers.checked = false;
            };

            // Initialize the worker
            this.faceDetectionWorker!.postMessage({ type: 'initialize' });

        } catch (error: unknown) {
            this.addToDetailedErrorLog('Failed to create Web Worker', (error as Error).message, 'critical');
            this.enableWebWorkers.checked = false;
        }
    }

    toggleWebWorkers() {
        if (this.enableWebWorkers.checked) {
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


    async detectFacesWithQualityProduction(image: HTMLImageElement, imageId: string) {
        // Enhanced version with error handling and retry logic
        const maxRetries = parseInt(this.retryAttempts.value) || 0;
        let lastError = null;

        for (let attempt = 0; attempt <= maxRetries; attempt++) {
            try {
                if (attempt > 0) {
                    this.addToProcessingLog(`Retry attempt ${attempt} for ${imageId}`, 'warning');
                    await this.delay(1000 * attempt); // Progressive delay
                }

                let processedImage = image;

                // Apply reduced resolution if enabled
                if (this.reducedResolution.checked) {
                    processedImage = await this.createReducedResolutionImage(image);
                }

                let faces;

                if (this.enableWebWorkers.checked && this.workerInitialized) {
                    faces = await this.detectFacesWithWorker(processedImage, imageId);
                } else {
                    faces = await this.detectFacesWithQuality(processedImage);
                }

                // If we used reduced resolution, scale back the coordinates
                if (this.reducedResolution.checked) {
                    const scale = image.width / processedImage.width;
                    faces = faces.map(face => ({
                        ...face,
                        x: face.x * scale,
                        y: face.y * scale,
                        width: face.width * scale,
                        height: face.height * scale
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

    async detectFacesWithWorker(image: HTMLImageElement, imageId: string): Promise<Face[]> {
        return new Promise((resolve, reject) => {
            if (!this.workerInitialized) {
                reject(new Error('Worker not initialized'));
                return;
            }

            // Create canvas to get image data
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d', { willReadFrequently: true });
            canvas.width = image.width;
            canvas.height = image.height;
            ctx!.drawImage(image, 0, 0);

            const imageData = ctx!.getImageData(0, 0, canvas.width, canvas.height);

            // Store callback for this request
            const requestId = `${imageId}_${Date.now()}`;
            this.workerCallbacks = this.workerCallbacks || new Map();
            this.workerCallbacks.set(requestId, { resolve, reject });

            // Send to worker
            this.faceDetectionWorker!.postMessage({
                type: 'detectFaces',
                id: requestId,
                data: {
                    imageData: {
                        data: imageData.data,
                        width: imageData.width,
                        height: imageData.height
                    },
                    options: { includeQuality: true }
                }
            });

            // Set timeout
            setTimeout(() => {
                if (this.workerCallbacks!.has(requestId)) {
                    this.workerCallbacks!.delete(requestId);
                    reject(new Error('Worker detection timeout'));
                }
            }, 30000); // 30 second timeout
        });
    }

    handleWorkerDetectionResult(requestId: string, faces: any[]) {
        if (this.workerCallbacks && this.workerCallbacks.has(requestId)) {
            const callback = this.workerCallbacks.get(requestId);
            if (callback) {
                const { resolve } = callback;
                this.workerCallbacks.delete(requestId);
                resolve(faces);
            }
        }
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

    addToDetailedErrorLog(title: string, details: string, severity: ErrorSeverity = 'error'): void {
        const timestamp = new Date().toLocaleTimeString();
        const entry = {
            timestamp,
            title,
            details,
            severity
        };

        this.errorLog!.push(entry);

        // Keep only last 50 error entries
        if (this.errorLog!.length > 50) {
            this.errorLog!.shift();
        }

        this.updateErrorLogDisplay();
    }

    updateErrorLogDisplay() {
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

    clearErrorLog() {
        this.errorLog = [];
        this.updateErrorLogDisplay();
        this.addToProcessingLog('Error log cleared', 'info');
        this.updateStatus('Error log cleared successfully', 'success');
    }

    exportErrorLog() {
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
        this.addToProcessingLog('Error log exported', 'success');
        this.updateStatus('Error log exported successfully', 'success');
    }

    setupErrorLogCollapsible() {
        const errorHeader = document.querySelector('.error-log-header');
        if (errorHeader) {
            errorHeader.addEventListener('click', () => {
                const content = errorHeader.nextElementSibling;
                const isCollapsed = errorHeader.classList.contains('collapsed');

                if (isCollapsed) {
                    errorHeader.classList.remove('collapsed');
                    content!.classList.remove('collapsed');
                } else {
                    errorHeader.classList.add('collapsed');
                    content!.classList.add('collapsed');
                }
            });
        }
    }

    createMemoryIndicator() {
        const indicator = document.createElement('div');
        indicator.className = 'memory-indicator';
        indicator.id = 'memoryIndicator';
        document.body.appendChild(indicator);

        // Update memory indicator periodically
        setInterval(() => this.updateMemoryIndicator(), 5000);
    }

    updateMemoryIndicator() {
        const indicator = document.getElementById('memoryIndicator');
        if (!indicator) return;

        const memoryInfo = {
            images: this.images!.size,
            processed: Array.from(this.images!.values()).filter((img: any) => img.processed).length,
            errors: this.errorLog!.length
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

    updateMemorySettings() {
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

    cleanupMemoryAuto() {
        // Clean up processed images older than 5 minutes
        const fiveMinutesAgo = Date.now() - (5 * 60 * 1000);

        for (const [id, imageData] of this.images!) {
            const imgEntry = imageData as unknown as ImageEntry;
            if (imgEntry.processed && imgEntry.processedAt && imgEntry.processedAt < fiveMinutesAgo) {
                this.cleanupImageData(imgEntry);
            }
        }
    }

    cleanupMemoryAggressive() {
        // Clean up all processed images immediately
        for (const [id, imageData] of this.images!) {
            if (imageData.processed) {
                this.cleanupImageData(imageData);
            }
        }
    }

    cleanupImageData(imageData: any) {
        // Clean up blob URLs
        if (imageData.image && imageData.image.src.startsWith('blob:')) {
            URL.revokeObjectURL(imageData.image.src);
        }

        if (imageData.enhancedImage && imageData.enhancedImage.src.startsWith('blob:')) {
            URL.revokeObjectURL(imageData.enhancedImage.src);
        }

        // Clear large data
        imageData.image = null;
        imageData.enhancedImage = null;

        // Mark as cleaned
        imageData.memoryCleanedUp = true;
    }

    async handleMultipleImageUploadProduction(event: Event) {
        const target = event.target as HTMLInputElement;
        const files = Array.from(target?.files || []) as File[];
        if (files.length === 0) return;

        this.updateStatus('Loading images...', 'loading');

        // Use lazy loading for large batches
        if (files.length > this.galleryPageSize) {
            await this.handleLargeImageBatch(files);
        } else {
            await this.handleStandardImageBatch(files);
        }
    }

    async handleLargeImageBatch(files: File[]) {
        this.addToProcessingLog(`Loading large batch: ${files.length} images`, 'info');

        // Load first page immediately
        const firstPage = files.slice(0, this.galleryPageSize);
        await this.loadImagePage(firstPage, 0);

        // Queue remaining files
        for (let i = this.galleryPageSize; i < files.length; i += this.galleryPageSize) {
            const page = files.slice(i, i + this.galleryPageSize);
            this.imageLoadQueue.push({ files: page, page: Math.floor(i / this.galleryPageSize) });
        }

        this.setupLazyLoading();
        this.updateStatus(`Loaded first ${firstPage.length} images. ${files.length - firstPage.length} queued for lazy loading.`, 'success');
    }

    async handleStandardImageBatch(files: File[]) {
        await this.loadImagePage(files, 0);
        this.updateStatus(`Loaded ${files.length} images.`, 'success');
    }

    async loadImagePage(files: File[], pageIndex: number) {
        for (const file of files) {
            const imageId = this.generateImageId();

            try {
                const image = await this.loadImageFromFileWithErrorHandling(file);
                this.images!.set(imageId, {
                    id: imageId,
                    file: file,
                    image: image,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    status: 'loaded',
                    page: pageIndex
                });
            } catch (error: unknown) {
                this.addToDetailedErrorLog(`Failed to load image: ${file.name}`, (error as Error).message, 'error');
            }
        }

        this.updateGallery();
        this.updateControls();
        this.imageGallery.classList.remove('hidden');
    }

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

    setupLazyLoading() {
        // Add lazy loading indicator to gallery
        const indicator = document.createElement('div');
        indicator.className = 'gallery-lazy-loading';
        indicator.innerHTML = 'Scroll to load more images...';
        this.galleryGrid.appendChild(indicator);

        // Setup intersection observer for lazy loading
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting && !this.isLoadingImages && this.imageLoadQueue.length > 0) {
                    this.loadNextImagePage();
                }
            });
        });

        observer.observe(indicator);
    }

    async loadNextImagePage() {
        if (this.imageLoadQueue.length === 0) return;

        this.isLoadingImages = true;
        const indicator = document.querySelector('.gallery-lazy-loading');

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

    async loadAllQueuedImages() {
        if (this.imageLoadQueue.length === 0) return;

        this.isLoadingImages = true;
        const totalQueued = this.imageLoadQueue.length;

        while (this.imageLoadQueue.length > 0) {
            const batch = this.imageLoadQueue.shift();
            if (!batch) break;
            const { files, page } = batch;
            const remainingPages = this.imageLoadQueue.length;

            this.updateStatus(`Loading batch ${totalQueued - remainingPages}/${totalQueued} (${files.length} images)...`, 'loading');

            await this.loadImagePage(files, page);

            // Small delay to prevent UI blocking
            await new Promise(resolve => setTimeout(resolve, 10));
        }

        this.isLoadingImages = false;

        // Remove lazy loading indicator
        const indicator = this.galleryGrid.querySelector('.gallery-lazy-loading');
        if (indicator) {
            indicator.remove();
        }

        this.updateStatus(`All images loaded. Total: ${this.images!.size} images.`, 'success');
    }

    // Stream processing: handles both loaded images and file references efficiently
    async processImagesAndFilesProduction(loadedImages: any[], queuedFiles: File[]) {
        if (this.isProcessing) return;

        if (!this.detector && (!this.enableWebWorkers.checked || !this.workerInitialized)) {
            this.updateStatus('Face detection not available. Please wait or check settings.', 'error');
            return;
        }

        this.isProcessing = true;
        const totalCount = loadedImages.length + queuedFiles.length;
        this.progressSection.classList.remove('hidden');
        this.updateControls();

        this.addToProcessingLog(`Starting stream processing of ${totalCount} images (${loadedImages.length} loaded, ${queuedFiles.length} from files)`, 'info');

        let successCount = 0;
        let errorCount = 0;

        // Process loaded images first
        for (let i = 0; i < loadedImages.length; i++) {
            const imageData = loadedImages[i];
            const progress = ((i + 1) / totalCount) * 100;

            this.progressFill.style.width = `${progress}%`;
            this.progressText.textContent = `Processing loaded image ${i + 1}/${totalCount}...`;

            try {
                await this.processImageDataProduction(imageData);
                successCount++;
            } catch (error: unknown) {
                errorCount++;
                this.addToProcessingLog(`Error processing ${imageData.file.name}: ${(error as Error).message}`, 'error');
            }
        }

        // Stream process queued files (load -> process -> discard)
        for (let i = 0; i < queuedFiles.length; i++) {
            const file = queuedFiles[i];
            const overallIndex = loadedImages.length + i + 1;
            const progress = (overallIndex / totalCount) * 100;

            this.progressFill.style.width = `${progress}%`;
            this.progressText.textContent = `Streaming file ${overallIndex}/${totalCount}: ${file.name}`;

            try {
                // Create temporary image data for processing
                const image = await this.loadImageFromFileWithErrorHandling(file);
                const tempImageData: any = {
                    id: `temp_${Date.now()}_${i}`,
                    file: file,
                    image: image,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    status: 'loaded' as const
                };

                // Process without storing in main images collection
                await this.processImageDataProduction(tempImageData);

                // Add results to permanent storage if any faces found
                if (tempImageData.results.length > 0) {
                    // Store only results, not the image data
                    this.imageResults.set(tempImageData.id, {
                        filename: file.name,
                        results: tempImageData.results,
                        processedAt: new Date().toISOString()
                    });
                }

                // Clean up temporary image from memory
                if (image.src && image.src.startsWith('blob:')) {
                    URL.revokeObjectURL(image.src);
                }

                successCount++;
            } catch (error: unknown) {
                errorCount++;
                this.addToProcessingLog(`Error processing ${file.name}: ${(error as Error).message}`, 'error');
            }
        }

        this.isProcessing = false;
        this.updateControls();
        this.progressFill.style.width = '100%';
        this.progressText.textContent = 'Processing complete!';

        const message = `Stream processing complete! Processed: ${successCount}, Errors: ${errorCount}`;
        this.updateStatus(message, errorCount > 0 ? 'warning' : 'success');
        this.addToProcessingLog(message, 'info');

        setTimeout(() => {
            this.progressSection.classList.add('hidden');
        }, 3000);
    }

    // Enhanced processImages with production optimizations
    async processImagesProduction(imagesToProcess: any[]) {
        if (this.isProcessing) return;

        if (!this.detector && (!this.enableWebWorkers.checked || !this.workerInitialized)) {
            this.updateStatus('Face detection not available. Please wait or check settings.', 'error');
            return;
        }

        this.isProcessing = true;
        this.processingQueue = imagesToProcess;
        this.progressSection.classList.remove('hidden');

        this.updateControls();
        this.addToProcessingLog(`Starting batch processing of ${imagesToProcess.length} images`, 'info');

        let successCount = 0;
        let errorCount = 0;

        for (let i = 0; i < imagesToProcess.length; i++) {
            const imageData = imagesToProcess[i];
            const progress = ((i + 1) / imagesToProcess.length) * 100;

            this.currentProcessingId = imageData.id;
            imageData.status = 'processing';
            this.updateGallery();

            this.progressFill.style.width = `${progress}%`;
            this.progressText.textContent = `Processing image ${i + 1} of ${imagesToProcess.length}: ${imageData.file.name}`;

            try {
                this.recordProcessingStart();
                await this.processImageDataProduction(imageData);
                imageData.processed = true;
                imageData.status = 'completed';
                imageData.processedAt = Date.now();
                this.recordProcessingEnd(true, imageData.faces.length);
                this.addToProcessingLog(`✓ Processed ${imageData.file.name}: ${imageData.faces.length} faces found`, 'success');
                successCount++;

                // Auto cleanup if enabled
                if (this.memoryManagement.value === 'auto') {
                    this.cleanupImageData(imageData);
                }

            } catch (error: unknown) {
                console.error('Error processing image:', imageData.file.name, (error as Error));
                imageData.status = 'error';
                this.recordProcessingEnd(false, 0);
                this.addToProcessingLog(`✗ Failed to process ${imageData.file.name}: ${(error as Error).message}`, 'error');
                this.addToDetailedErrorLog(`Processing failed: ${imageData.file.name}`, (error as Error).message, 'error');
                errorCount++;

                // Continue on error if enabled
                if (!this.continueOnError.checked) {
                    this.addToProcessingLog('Processing stopped due to error (Continue on Error disabled)', 'warning');
                    break;
                }
            }

            this.updateGallery();

            // Small delay to prevent UI blocking
            await new Promise(resolve => setTimeout(resolve, 50));
        }

        this.isProcessing = false;
        this.currentProcessingId = null;
        this.progressSection.classList.add('hidden');

        this.updateControls();

        const totalFaces = imagesToProcess.reduce((sum: number, img: any) => sum + (img.results?.length || 0), 0);

        this.addToProcessingLog(`Batch processing completed: ${successCount} successful, ${errorCount} failed, ${totalFaces} total faces cropped`, 'info');
        this.updateStatus(`Processed ${successCount} images (${errorCount} failed) and found ${totalFaces} faces total!`, successCount > 0 ? 'success' : 'error');
    }

    async processImageDataProduction(imageData: any) {
        // Detect faces with enhanced error handling
        const faces = await this.detectFacesWithQualityProduction(imageData.image, imageData.id);
        imageData.faces = faces;

        // Crop faces (only selected ones)
        imageData.results = await this.cropFacesFromImageData(imageData);

    }

    updateStatus(message: string, type: string = '') {
        this.status!.textContent = message;
        this.status!.className = `status ${type}`;
    }


    updateAspectRatioDisplay() {
        const width = parseInt(this.outputWidth.value);
        const height = parseInt(this.outputHeight.value);
        const ratio = width / height;

        let ratioText = `${ratio.toFixed(2)}:1`;
        if (Math.abs(ratio - 1) < 0.01) ratioText = '1:1 (Square)';
        else if (Math.abs(ratio - 4/3) < 0.01) ratioText = '4:3 (Standard)';
        else if (Math.abs(ratio - 16/9) < 0.01) ratioText = '16:9 (Widescreen)';
        else if (Math.abs(ratio - 3/4) < 0.01) ratioText = '3:4 (Portrait)';

        this.aspectRatioText.textContent = `${ratioText} ratio`;
        this.currentAspectRatio = ratio;
    }

    updateFormatControls() {
        const format = this.outputFormat.value;
        if (format === 'jpeg') {
            this.jpegQualityGroup.classList.remove('hidden');
        } else {
            this.jpegQualityGroup.classList.add('hidden');
        }
        this.updatePreview();
    }

    updateQualityDisplay() {
        this.qualityValue.textContent = this.jpegQuality.value + '%';
    }

    toggleIndividualDownloadButtons() {
        // Refresh face overlays to show/hide individual download buttons
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length > 0 && selectedImages[0].faces) {
            this.displayImageWithFaceOverlays(selectedImages[0]);
        }
    }

    // Smart cropping methods
    applyPreset() {
        const preset = this.sizePreset.value;
        const presets: Record<string, { width: number; height: number }> = {
            custom: { width: 256, height: 256 },
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


    handleDimensionChange(changedDimension: string) {
        if (this.aspectRatioLocked) {
            if (changedDimension === 'width') {
                const newWidth = parseInt(this.outputWidth.value);
                const newHeight = Math.round(newWidth / this.currentAspectRatio);
                this.outputHeight.value = String(Math.max(64, Math.min(2048, newHeight)));
            } else {
                const newHeight = parseInt(this.outputHeight.value);
                const newWidth = Math.round(newHeight * this.currentAspectRatio);
                this.outputWidth.value = String(Math.max(64, Math.min(2048, newWidth)));
            }
        }

        // Update preset to custom if dimensions don't match any preset
        const width = parseInt(this.outputWidth.value);
        const height = parseInt(this.outputHeight.value);

        const matchingPreset = this.findMatchingPreset(width, height);
        this.sizePreset.value = matchingPreset;

        this.updatePreview();
    }

    findMatchingPreset(width: number, height: number) {
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

    updatePositioningControls() {
        const mode = this.positioningMode.value;
        if (mode === 'custom' || mode === 'rule-of-thirds') {
            this.advancedPositioning.classList.remove('hidden');
        } else {
            this.advancedPositioning.classList.add('hidden');
        }
        this.updatePreview();
    }

    updateOffsetDisplay(direction: string) {
        if (direction === 'vertical') {
            this.verticalOffsetValue.textContent = this.verticalOffset.value + '%';
        } else {
            this.horizontalOffsetValue.textContent = this.horizontalOffset.value + '%';
        }
    }

    updateOffsetDisplays() {
        this.updateOffsetDisplay('vertical');
        this.updateOffsetDisplay('horizontal');
    }

    applySettingsToAll() {
        // Get current settings
        const settings = {
            width: parseInt(this.outputWidth.value),
            height: parseInt(this.outputHeight.value),
            faceHeightPct: parseInt(this.faceHeightPct.value),
            positioningMode: this.positioningMode.value,
            verticalOffset: parseInt(this.verticalOffset.value),
            horizontalOffset: parseInt(this.horizontalOffset.value),
            format: this.outputFormat.value,
            jpegQuality: parseInt(this.jpegQuality.value),
            preset: this.sizePreset.value
        };

        // Apply to all image processing settings
        // This will be used during batch processing
        this.globalSettings = settings;

        this.updateStatus(`Applied settings to all images: ${settings.width}×${settings.height}px, ${settings.positioningMode} positioning`, 'success');
    }

    resetToDefaults() {
        this.outputWidth.value = String(256);
        this.outputHeight.value = String(256);
        this.faceHeightPct.value = String(70);
        this.sizePreset.value = 'custom';
        this.positioningMode.value = 'center';
        this.verticalOffset.value = String(0);
        this.horizontalOffset.value = String(0);
        this.aspectRatioLocked = false;
        this.aspectRatioLock.classList.remove('locked');
        this.aspectRatioLock.textContent = '🔓';

        this.updatePreview();
        this.updatePositioningControls();
        this.updateOffsetDisplays();

        this.updateStatus('Settings reset to defaults', 'success');
    }

    calculateSmartCropPosition(face: any, cropWidth: number, cropHeight: number, imageWidth: number, imageHeight: number) {
        const mode = this.positioningMode.value;
        const vOffset = parseInt(this.verticalOffset.value) / 100;
        const hOffset = parseInt(this.horizontalOffset.value) / 100;

        const faceCenterX = face.x + face.width / 2;
        const faceCenterY = face.y + face.height / 2;

        let targetX, targetY;

        switch (mode) {
            case 'rule-of-thirds':
                // Position eyes at rule of thirds points
                // Eyes are typically at about 65% from top of face bounding box
                const eyesY = face.y + face.height * 0.35;

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

    async waitForMediaPipe() {
        // Wait for MediaPipe Tasks Vision library to be available
        const maxWaitTime = 10000; // 10 seconds
        const checkInterval = 100; // 100ms
        let elapsed = 0;

        while (elapsed < maxWaitTime) {
            if (typeof window.vision !== 'undefined' &&
                window.vision.FilesetResolver &&
                window.vision.FaceDetector) {
                return; // Library is loaded
            }
            await new Promise(resolve => setTimeout(resolve, checkInterval));
            elapsed += checkInterval;
        }

        throw new Error('MediaPipe Tasks Vision library failed to load within timeout');
    }

    async loadModel() {
        try {
            this.updateStatus('Loading MediaPipe face detection model...', 'loading');

            // Wait for MediaPipe Tasks Vision library to load
            await this.waitForMediaPipe();

            // Check if MediaPipe Tasks Vision is available
            if (!window.vision || !window.vision.FilesetResolver || !window.vision.FaceDetector) {
                throw new Error('MediaPipe Tasks Vision library not loaded');
            }

            console.log('Initializing MediaPipe Tasks Vision...');

            // Initialize the MediaPipe Vision tasks
            const visionFileset = await window.vision.FilesetResolver.forVisionTasks(
                "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm"
            );

            // Create face detector with WebAssembly runtime
            this.detector = await window.vision.FaceDetector.createFromOptions(visionFileset, {
                baseOptions: {
                    modelAssetPath: `https://storage.googleapis.com/mediapipe-models/face_detector/blaze_face_short_range/float16/1/blaze_face_short_range.tflite`,
                    delegate: "GPU"
                },
                runningMode: "IMAGE"
            });

            console.log('MediaPipe face detector created successfully');
            this.updateStatus('MediaPipe model loaded successfully. Ready to process images!', 'success');
        } catch (error: unknown) {
            console.error('Error loading MediaPipe model:', (error as Error));
            this.updateStatus(`Error loading model: ${(error as Error).message}. Please refresh the page.`, 'error');

            // Try to provide helpful error information
            if (!window.vision || !window.vision.FilesetResolver || !window.vision.FaceDetector) {
                this.updateStatus('MediaPipe Tasks Vision library not loaded. Please refresh the page.', 'error');
            }
        }
    }

    async handleMultipleImageUpload(event: Event) {
        const target = event.target as HTMLInputElement;
        const files = Array.from(target?.files || []) as File[];
        if (files.length === 0) return;

        this.updateStatus('Loading images...', 'loading');

        for (const file of files) {
            const imageId = this.generateImageId();

            try {
                const image = await this.loadImageFromFile(file);
                this.images!.set(imageId, {
                    id: imageId,
                    file: file,
                    image: image,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    status: 'loaded'
                });
            } catch (error: unknown) {
                console.error('Error loading image:', file.name, (error as Error));
            }
        }

        this.updateGallery();
        this.updateControls();
        this.imageGallery.classList.remove('hidden');
        this.updateStatus(`Loaded ${this.images!.size} images. Select images and click "Process Selected" or "Process All".`, 'success');
    }

    async loadImageFromFile(file: File): Promise<HTMLImageElement> {
        return new Promise<HTMLImageElement>((resolve, reject) => {
            const img = new Image();
            img.onload = () => resolve(img);
            img.onerror = reject;
            img.src = URL.createObjectURL(file);
        });
    }

    generateImageId() {
        return 'img_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
    }

    updateGallery() {
        this.galleryGrid.innerHTML = '';

        for (const [imageId, imageData] of this.images!) {
            const galleryItem = this.createGalleryItem(imageData);
            this.galleryGrid.appendChild(galleryItem);
        }

        this.updateSelectionCounter();
    }

    createGalleryItem(imageData: any) {
        const item = super.createGalleryItem(imageData);
        item.addEventListener('click', () => this.toggleSelection(imageData.id));
        return item;
    }

    toggleSelection(imageId: string) {
        const imageData = this.images!.get(imageId);
        if (imageData) {
            imageData.selected = !imageData.selected;
            this.updateGallery();
            this.updateControls();
        }
    }

    selectAll() {
        for (const imageData of this.images!.values()) {
            imageData.selected = true;
        }
        this.updateGallery();
        this.updateControls();
    }

    selectNone() {
        for (const imageData of this.images!.values()) {
            imageData.selected = false;
        }
        this.updateGallery();
        this.updateControls();
    }

    updateSelectionCounter() {
        const selected = Array.from(this.images!.values()).filter((img: any) => img.selected).length;
        const loaded = this.images!.size;
        const queued = this.imageLoadQueue.length * this.galleryPageSize;
        const total = loaded + queued;

        this.selectedCount.textContent = String(selected);

        if (queued > 0) {
            this.totalCount.textContent = `${loaded} (+${queued} queued)`;
        } else {
            this.totalCount.textContent = String(total);
        }
    }

    updateControls() {
        const hasImages = this.images!.size > 0;
        const hasQueuedFiles = this.imageLoadQueue.length > 0;
        const hasSelected = Array.from(this.images!.values()).some(img => img.selected);
        const hasLoadedResults = Array.from(this.images!.values()).some(img => img.results.length > 0);
        const hasStreamedResults = this.imageResults.size > 0;
        const hasResults = hasLoadedResults || hasStreamedResults;

        this.processAllBtn!.disabled = (!hasImages && !hasQueuedFiles) || this.isProcessing;
        this.processSelectedBtn.disabled = !hasSelected || this.isProcessing;
        this.clearAllBtn.disabled = !hasImages && !hasQueuedFiles;
        this.downloadAllBtn.disabled = !hasResults;
    }

    async processAll() {
        // Process both loaded images and queued file references
        const loadedImages = Array.from(this.images!.values());
        const queuedFiles = this.imageLoadQueue.flatMap(batch => batch.files);

        if (queuedFiles.length > 0) {
            this.updateStatus(`Processing ${loadedImages.length} loaded images + ${queuedFiles.length} queued files...`, 'loading');
            await this.processImagesAndFilesProduction(loadedImages, queuedFiles);
        } else {
            await this.processImagesProduction(loadedImages);
        }
    }

    async processSelected() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);

        // If no images are selected but there are queued images, inform the user
        if (selectedImages.length === 0 && this.imageLoadQueue.length > 0) {
            this.updateStatus('No images selected. Use "Process All" to process all images including queued ones.', 'warning');
            return;
        }

        await this.processImagesProduction(selectedImages);
    }

    async processImages(imagesToProcess: any[]) {
        if (this.isProcessing) return;

        if (!this.detector) {
            this.updateStatus('Face detection model not loaded. Please wait and try again.', 'error');
            return;
        }

        this.isProcessing = true;
        this.processingQueue = imagesToProcess;
        this.progressSection.classList.remove('hidden');

        this.updateControls();
        this.addToProcessingLog(`Starting batch processing of ${imagesToProcess.length} images`, 'info');

        for (let i = 0; i < imagesToProcess.length; i++) {
            const imageData = imagesToProcess[i];
            const progress = ((i + 1) / imagesToProcess.length) * 100;

            this.currentProcessingId = imageData.id;
            imageData.status = 'processing';
            this.updateGallery();

            this.progressFill.style.width = `${progress}%`;
            this.progressText.textContent = `Processing image ${i + 1} of ${imagesToProcess.length}: ${imageData.file.name}`;

            try {
                this.recordProcessingStart();
                await this.processImageData(imageData);
                imageData.processed = true;
                imageData.status = 'completed';
                this.recordProcessingEnd(true, imageData.faces.length);
                this.addToProcessingLog(`✓ Processed ${imageData.file.name}: ${imageData.faces.length} faces found`, 'success');
            } catch (error: unknown) {
                console.error('Error processing image:', imageData.file.name, (error as Error));
                imageData.status = 'error';
                this.recordProcessingEnd(false, 0);
                this.addToProcessingLog(`✗ Failed to process ${imageData.file.name}: ${(error as Error).message}`, 'error');
            }

            this.updateGallery();

            // Small delay to prevent UI blocking
            await new Promise(resolve => setTimeout(resolve, 100));
        }

        this.isProcessing = false;
        this.currentProcessingId = null;
        this.progressSection.classList.add('hidden');

        this.updateControls();

        const successCount = imagesToProcess.filter((img: any) => img.status === 'completed').length;
        const totalFaces = imagesToProcess.reduce((sum: number, img: any) => sum + img.results.length, 0);

        this.addToProcessingLog(`Batch processing completed: ${successCount}/${imagesToProcess.length} images successful, ${totalFaces} total faces cropped`, 'info');
        this.updateStatus(`Processed ${successCount} images and found ${totalFaces} faces total!`, 'success');
    }

    async processImageData(imageData: any) {
        // Detect faces with quality analysis
        const faces = await this.detectFacesWithQuality(imageData.image);
        imageData.faces = faces;

        // Crop faces (only selected ones)
        imageData.results = await this.cropFacesFromImageData(imageData);

    }

    async detectFacesWithQuality(image: HTMLImageElement) {
        if (!this.detector) {
            throw new Error('Face detection model not loaded. Please wait for model to load.');
        }

        const detectionResult = await this.detector.detect(image);
        const detectedFaces = [];

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


    async calculateFaceQuality(image: HTMLImageElement, x: number, y: number, width: number, height: number) {
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
            console.warn('Face quality analysis skipped due to canvas limits', (error as Error));
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

    calculateLaplacianVariance(data: Uint8ClampedArray, width: number, height: number) {
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

    // Face selection and overlay methods
    async detectCurrentImageFaces() {
        // For now, we'll work with a selected image from gallery
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) {
            this.updateStatus('Please select an image first', 'error');
            return;
        }

        const currentImage = selectedImages[0]; // Use first selected image
        this.updateStatus('Detecting faces...', 'loading');

        try {
            currentImage.faces = await this.detectFacesWithQuality(currentImage.image) as any;
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
            this.canvasContainer.classList.remove('hidden');
            this.updateStatus(`Detected ${currentImage.faces.length} faces with quality analysis`, 'success');
        } catch (error: unknown) {
            console.error('Error detecting faces:', (error as Error));
            this.updateStatus(`Error detecting faces: ${(error as Error).message}`, 'error');
        }
    }

    displayImageWithFaceOverlays(imageData: any) {
        const maxWidth = 800;
        const maxHeight = 600;

        let { width, height } = imageData.image;
        const scale = Math.min(maxWidth / width, maxHeight / height, 1);

        const displayWidth = width * scale;
        const displayHeight = height * scale;

        this.inputCanvas.width = displayWidth;
        this.inputCanvas.height = displayHeight;
        this.ctx.clearRect(0, 0, displayWidth, displayHeight);
        this.ctx.drawImage(imageData.image, 0, 0, displayWidth, displayHeight);

        // Clear and recreate overlays
        this.faceOverlays.innerHTML = '';
        this.faceOverlays.style.width = displayWidth + 'px';
        this.faceOverlays.style.height = displayHeight + 'px';

        // Create face overlays
        imageData.faces.forEach((face: any) => {
            this.createFaceOverlay(face, scale);
        });
    }

    createFaceOverlay(face: any, scale: number) {
        const faceBox = document.createElement('div');
        faceBox.className = 'face-box';
        faceBox.dataset.faceId = face.id;
        faceBox.style.left = (face.x * scale) + 'px';
        faceBox.style.top = (face.y * scale) + 'px';
        faceBox.style.width = (face.width * scale) + 'px';
        faceBox.style.height = (face.height * scale) + 'px';

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
            this.toggleFaceSelection(face.id);
        });

        // Create index number
        const index = document.createElement('div');
        index.className = 'face-index';
        index.textContent = face.index;

        // Create confidence score
        const confidence = document.createElement('div');
        confidence.className = 'face-confidence';
        confidence.textContent = `${(face.confidence * 100).toFixed(0)}%`;

        // Create quality indicator
        const quality = document.createElement('div');
        quality.className = `face-quality ${face.quality.level}`;
        quality.textContent = face.quality.level.toUpperCase();

        // Create resize handles
        const handles = ['nw', 'ne', 'sw', 'se'].map(pos => {
            const handle = document.createElement('div');
            handle.className = `resize-handle ${pos}`;
            handle.dataset.position = pos;
            handle.addEventListener('mousedown', (e) => this.startResize(e, face.id));
            return handle;
        });

        // Add click to select
        faceBox.addEventListener('click', () => this.toggleFaceSelection(face.id));

        // Create individual download button if enabled
        if (this.individualDownload.checked) {
            const downloadBtn = document.createElement('button');
            downloadBtn.className = 'individual-download-btn';
            downloadBtn.innerHTML = '↓';
            downloadBtn.title = 'Download this face';
            downloadBtn.addEventListener('click', (e) => {
                e.stopPropagation();
                this.downloadIndividualFace(face.id);
            });
            faceBox.appendChild(downloadBtn);
        }

        // Append all elements
        faceBox.appendChild(checkbox);
        faceBox.appendChild(index);
        faceBox.appendChild(confidence);
        faceBox.appendChild(quality);
        handles.forEach(handle => faceBox.appendChild(handle));

        this.faceOverlays.appendChild(faceBox);
    }

    toggleFaceSelection(faceId: string) {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        const face = currentImage.faces.find(f => f.id === faceId);
        if (face) {
            face.selected = !face.selected;
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
        }
    }

    selectAllFaces() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            currentImage.faces.forEach(face => face.selected = true);
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
        }
    }

    selectNoneFaces() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            currentImage.faces.forEach(face => face.selected = false);
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
        }
    }

    updateFaceCounter() {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) {
            this.faceCount.textContent = '0';
            this.selectedFaceCount.textContent = '0';
            return;
        }

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            const total = currentImage.faces.length;
            const selected = currentImage.faces.filter(f => f.selected).length;
            this.faceCount.textContent = String(total);
            this.selectedFaceCount.textContent = String(selected);
        }
    }

    startResize(e: MouseEvent, faceId: string) {
        e.preventDefault();
        e.stopPropagation();
        // Basic resize functionality - can be expanded
        const target = e.target as HTMLElement;
        console.log('Resize started for face:', faceId, 'handle:', target?.dataset?.position);
    }

    async cropFacesFromImageData(imageData: any) {
        if (!imageData.faces || imageData.faces.length === 0) return [];

        // Only crop selected faces
        const selectedFaces = imageData.faces.filter((face: any) => face.selected);
        if (selectedFaces.length === 0) return [];

        const results = [];
        const outputWidth = parseInt(this.outputWidth.value);
        const outputHeight = parseInt(this.outputHeight.value);
        const faceHeightPct = parseInt(this.faceHeightPct.value) / 100;

        // Use enhanced image if available, otherwise use original
        let sourceImage = imageData.image;
        if (imageData.enhancedImage) {
            sourceImage = imageData.enhancedImage;
        }

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
            const scale = targetFaceHeight / face.height;

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
            const filename = this.generateBatchFilename(imageData.file.name, face.index, outputWidth, outputHeight);

            results.push({
                dataUrl: croppedDataUrl,
                faceIndex: face.index,
                faceId: face.id,
                sourceImage: imageData.file.name,
                filename: filename,
                format: format,
                quality: format === 'jpeg' ? this.jpegQuality.value : 100
            });
        }

        return results;
    }

    generateBatchFilename(originalName: string, faceIndex: number, width: number, height: number) {
        const template = this.namingTemplate.value;
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

    async downloadAllResults() {
        const allResults = [];

        // Collect results from loaded images
        for (const imageData of this.images!.values()) {
            if (imageData.results.length > 0) {
                allResults.push(...imageData.results);
            }
        }

        // Collect results from streamed processing
        for (const streamedResult of this.imageResults.values()) {
            if (streamedResult.results.length > 0) {
                allResults.push(...streamedResult.results);
            }
        }

        if (allResults.length === 0) {
            this.updateStatus('No results to download', 'error');
            return;
        }

        if (this.zipDownload.checked) {
            await this.downloadAsZip(allResults);
        } else {
            this.downloadIndividually(allResults);
        }
    }

    async downloadAsZip(results: any[]) {
        try {
            this.updateStatus('Creating ZIP archive...', 'loading');

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

            this.updateStatus(`Downloaded ${results.length} cropped faces as ZIP archive!`, 'success');
        } catch (error: unknown) {
            console.error('Error creating ZIP:', (error as Error));
            this.updateStatus(`Error creating ZIP: ${(error as Error).message}`, 'error');
        }
    }

    downloadIndividually(results: any[]) {
        let totalDownloads = 0;

        results.forEach((result) => {
            const link = document.createElement('a');
            link.download = result.filename;
            link.href = result.dataUrl;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            totalDownloads++;
        });

        this.updateStatus(`Downloaded ${totalDownloads} cropped faces!`, 'success');
    }

    async downloadIndividualFace(faceId: string) {
        const selectedImages = Array.from(this.images!.values()).filter((img: any) => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        const face = currentImage.faces.find(f => f.id === faceId);
        if (!face) return;

        try {
            this.updateStatus('Cropping individual face...', 'loading');

            // Crop just this face
            const result = await this.cropSingleFace(currentImage, face);

            // Download directly
            const link = document.createElement('a');
            link.download = result.filename;
            link.href = result.dataUrl;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);

            this.updateStatus('Individual face downloaded!', 'success');
        } catch (error: unknown) {
            console.error('Error downloading individual face:', (error as Error));
            this.updateStatus(`Error downloading face: ${(error as Error).message}`, 'error');
        }
    }

    async cropSingleFace(imageData: any, face: any) {
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

    clearAll() {
        // Clean up object URLs
        for (const imageData of this.images!.values()) {
            if (imageData.image && imageData.image.src && imageData.image.src.startsWith('blob:')) {
                URL.revokeObjectURL(imageData.image.src);
            }
        }

        this.images!.clear();
        this.imageResults.clear();
        this.imageLoadQueue = [];
        this.processingQueue = [];
        this.isProcessing = false;
        this.currentProcessingId = null;

        this.imageGallery.classList.add('hidden');
        this.canvasContainer.classList.add('hidden');
        this.progressSection.classList.add('hidden');
        this.croppedContainer.innerHTML = '';

        this.updateControls();
        this.updateStatus('Workspace cleared. Ready to load new images.', 'success');

        // Reset file input
        this.imageInput.value = '';
    }
}

document.addEventListener('DOMContentLoaded', () => {
    new FaceCropper();
});
