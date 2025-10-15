import { BaseFaceCropper } from './base-face-cropper.js';
import type { FaceData } from './types.js';

// Single processor specific result interface
interface CroppedResult {
    face: FaceData;
    image: HTMLCanvasElement;
    index: number;
}

class SingleImageFaceCropper extends BaseFaceCropper {
    private currentImage: HTMLImageElement | null;
    private currentFile: File | null;
    private faces: FaceData[];
    private selectedFaces: Set<number | string>;
    private rotationAngle: number = 0; // Track rotation in degrees (0, 90, 180, 270)
    private autoReprocessTimer: number | null = null;
    private readonly AUTO_REPROCESS_DELAY = 500; // ms delay for debouncing

    // UI Elements
    private imageInput!: HTMLInputElement;
    private processImageBtn?: HTMLButtonElement; // Optional - may not exist in realtime mode
    private clearImageBtn!: HTMLButtonElement;
    private downloadResultsBtn!: HTMLButtonElement;
    protected progressSection!: HTMLElement;
    protected progressFill!: HTMLElement;
    protected progressText!: HTMLElement;
    private canvasContainer!: HTMLElement;
    protected inputCanvas!: HTMLCanvasElement;
    private outputCanvas!: HTMLCanvasElement;
    private croppedFaces!: HTMLElement;
    private croppedContainer!: HTMLElement;
    protected status!: HTMLElement;
    protected faceOverlays!: HTMLElement;
    protected faceCount!: HTMLElement;
    protected selectedFaceCount!: HTMLElement;
    private selectAllFacesBtn!: HTMLButtonElement;
    private selectNoneFacesBtn!: HTMLButtonElement;
    private detectFacesBtn?: HTMLButtonElement; // Optional - may not exist in realtime mode
    private rotateClockwiseBtn!: HTMLButtonElement;
    private rotateCounterClockwiseBtn!: HTMLButtonElement;

    // Settings elements
    protected outputWidth!: HTMLInputElement;
    protected outputHeight!: HTMLInputElement;
    protected faceHeightPct!: HTMLInputElement;
    protected positioningMode!: HTMLSelectElement;
    protected verticalOffset!: HTMLInputElement;
    protected horizontalOffset!: HTMLInputElement;
    protected aspectRatioLock!: HTMLButtonElement;
    protected sizePreset!: HTMLSelectElement;
    protected outputFormat!: HTMLSelectElement;
    protected jpegQuality!: HTMLInputElement;
    protected namingTemplate!: HTMLInputElement;

    // Stats elements
    private facesDetected!: HTMLElement;
    private processingTime!: HTMLElement;
    private imageSize!: HTMLElement;
    protected processingStatus!: HTMLElement;
    protected processingLogElement!: HTMLElement;

    // Enhancement elements
    protected autoColorCorrection!: HTMLInputElement;
    protected exposureAdjustment!: HTMLInputElement;
    protected contrastAdjustment!: HTMLInputElement;
    protected sharpnessControl!: HTMLInputElement;
    protected skinSmoothing!: HTMLInputElement;
    protected redEyeRemoval!: HTMLInputElement;
    protected backgroundBlur!: HTMLInputElement;

    constructor() {
        super();
        this.currentImage = null;
        this.currentFile = null;
        this.faces = [];
        this.selectedFaces = new Set();

        this.initializeElements();
        this.setupEventListeners();
        this.loadModel();
    }

    private initializeElements(): void {
        this.imageInput = document.getElementById('imageInput') as HTMLInputElement;
        this.processImageBtn = document.getElementById('processImageBtn') as HTMLButtonElement | undefined;
        this.clearImageBtn = document.getElementById('clearImageBtn') as HTMLButtonElement;
        this.downloadResultsBtn = document.getElementById('downloadResultsBtn') as HTMLButtonElement;
        this.progressSection = document.getElementById('progressSection') as HTMLElement;
        this.progressFill = document.getElementById('progressFill') as HTMLElement;
        this.progressText = document.getElementById('progressText') as HTMLElement;
        this.canvasContainer = document.getElementById('canvasContainer') as HTMLElement;
        this.inputCanvas = document.getElementById('inputCanvas') as HTMLCanvasElement;
        this.outputCanvas = document.getElementById('outputCanvas') as HTMLCanvasElement;
        this.croppedFaces = document.getElementById('croppedFaces') as HTMLElement;
        this.croppedContainer = document.getElementById('croppedContainer') as HTMLElement;
        this.status = document.getElementById('status') as HTMLElement;
        this.faceOverlays = document.getElementById('faceOverlays') as HTMLElement;
        this.faceCount = document.getElementById('faceCount') as HTMLElement;
        this.selectedFaceCount = document.getElementById('selectedFaceCount') as HTMLElement;
        this.selectAllFacesBtn = document.getElementById('selectAllFacesBtn') as HTMLButtonElement;
        this.selectNoneFacesBtn = document.getElementById('selectNoneFacesBtn') as HTMLButtonElement;
        this.detectFacesBtn = document.getElementById('detectFacesBtn') as HTMLButtonElement | undefined;
        this.rotateClockwiseBtn = document.getElementById('rotateClockwiseBtn') as HTMLButtonElement;
        this.rotateCounterClockwiseBtn = document.getElementById('rotateCounterClockwiseBtn') as HTMLButtonElement;

        // Settings elements
        this.outputWidth = document.getElementById('outputWidth') as HTMLInputElement;
        this.outputHeight = document.getElementById('outputHeight') as HTMLInputElement;
        this.faceHeightPct = document.getElementById('faceHeightPct') as HTMLInputElement;
        this.positioningMode = document.getElementById('positioningMode') as HTMLSelectElement;
        this.verticalOffset = document.getElementById('verticalOffset') as HTMLInputElement;
        this.horizontalOffset = document.getElementById('horizontalOffset') as HTMLInputElement;
        this.aspectRatioLock = document.getElementById('aspectRatioLock') as HTMLButtonElement;
        this.sizePreset = document.getElementById('sizePreset') as HTMLSelectElement;
        this.outputFormat = document.getElementById('outputFormat') as HTMLSelectElement;
        this.jpegQuality = document.getElementById('jpegQuality') as HTMLInputElement;
        this.namingTemplate = document.getElementById('namingTemplate') as HTMLInputElement;

        // Stats elements
        this.facesDetected = document.getElementById('facesDetected') as HTMLElement;
        this.processingTime = document.getElementById('processingTime') as HTMLElement;
        this.imageSize = document.getElementById('imageSize') as HTMLElement;
        this.processingStatus = document.getElementById('processingStatus') as HTMLElement;
        this.processingLogElement = document.getElementById('processingLog') as HTMLElement;

        // Enhancement elements
        this.autoColorCorrection = document.getElementById('autoColorCorrection') as HTMLInputElement;
        this.exposureAdjustment = document.getElementById('exposureAdjustment') as HTMLInputElement;
        this.contrastAdjustment = document.getElementById('contrastAdjustment') as HTMLInputElement;
        this.sharpnessControl = document.getElementById('sharpnessControl') as HTMLInputElement;
        this.skinSmoothing = document.getElementById('skinSmoothing') as HTMLInputElement;
        this.redEyeRemoval = document.getElementById('redEyeRemoval') as HTMLInputElement;
        this.backgroundBlur = document.getElementById('backgroundBlur') as HTMLInputElement;

        this.updateUI();
    }

    private setupEventListeners(): void {
        this.imageInput.addEventListener('change', (e: Event) => this.handleImageUpload(e));
        if (this.processImageBtn) {
            this.processImageBtn.addEventListener('click', () => this.processImage());
        }
        this.clearImageBtn.addEventListener('click', () => this.clearImage());
        this.downloadResultsBtn.addEventListener('click', () => this.downloadResults());

        // Face selection listeners
        this.selectAllFacesBtn.addEventListener('click', () => this.selectAllFaces());
        this.selectNoneFacesBtn.addEventListener('click', () => this.selectNoneFaces());
        if (this.detectFacesBtn) {
            this.detectFacesBtn.addEventListener('click', () => this.detectFaces());
        }

        // Rotation listeners
        this.rotateClockwiseBtn.addEventListener('click', () => this.rotateImage(90));
        this.rotateCounterClockwiseBtn.addEventListener('click', () => this.rotateImage(-90));

        // Settings listeners - trigger auto-reprocessing
        this.outputWidth.addEventListener('input', () => {
            this.updatePreview();
            this.autoReprocess();
        });
        this.outputHeight.addEventListener('input', () => {
            this.updatePreview();
            this.autoReprocess();
        });
        this.faceHeightPct.addEventListener('input', () => {
            this.updatePreview();
            this.autoReprocess();
        });
        this.positioningMode.addEventListener('change', () => {
            this.updateAdvancedPositioning();
            this.autoReprocess();
        });
        this.verticalOffset.addEventListener('input', () => {
            this.updateOffsetDisplay('vertical');
            this.autoReprocess();
        });
        this.horizontalOffset.addEventListener('input', () => {
            this.updateOffsetDisplay('horizontal');
            this.autoReprocess();
        });
        this.aspectRatioLock.addEventListener('click', () => {
            this.toggleAspectRatioLock();
            this.autoReprocess();
        });
        this.sizePreset.addEventListener('change', () => {
            this.applySizePreset();
            this.autoReprocess();
        });
        this.outputFormat.addEventListener('change', () => this.updateFormatSettings());

        // Navigation listeners
        const backToMultipleBtn = document.getElementById('backToMultipleBtn');
        if (backToMultipleBtn) {
            backToMultipleBtn.addEventListener('click', () => {
                window.location.href = 'batch-processing.html';
            });
        }

        const csvBatchModeBtn = document.getElementById('csvBatchModeBtn');
        if (csvBatchModeBtn) {
            csvBatchModeBtn.addEventListener('click', () => {
                window.location.href = 'csv-processing.html';
            });
        }

        // Dark mode listener
        const darkModeBtn = document.getElementById('darkModeBtn');
        if (darkModeBtn) {
            darkModeBtn.addEventListener('click', () => this.toggleDarkMode());
        }

        // Enhancement listeners - trigger auto-reprocessing
        if (this.exposureAdjustment) {
            this.exposureAdjustment.addEventListener('input', () => {
                this.updateSliderValue('exposure');
                this.autoReprocess();
            });
        }
        if (this.contrastAdjustment) {
            this.contrastAdjustment.addEventListener('input', () => {
                this.updateSliderValue('contrast');
                this.autoReprocess();
            });
        }
        if (this.sharpnessControl) {
            this.sharpnessControl.addEventListener('input', () => {
                this.updateSliderValue('sharpness');
                this.autoReprocess();
            });
        }
        if (this.skinSmoothing) {
            this.skinSmoothing.addEventListener('input', () => {
                this.updateSliderValue('skinSmoothing');
                this.autoReprocess();
            });
        }
        if (this.backgroundBlur) {
            this.backgroundBlur.addEventListener('input', () => {
                this.updateSliderValue('backgroundBlur');
                this.autoReprocess();
            });
        }
        if (this.autoColorCorrection) {
            this.autoColorCorrection.addEventListener('change', () => this.autoReprocess());
        }
        if (this.redEyeRemoval) {
            this.redEyeRemoval.addEventListener('change', () => this.autoReprocess());
        }

        // Drag and drop
        document.addEventListener('dragover', (e: DragEvent) => {
            e.preventDefault();
            e.stopPropagation();
        });

        document.addEventListener('drop', (e: DragEvent) => {
            e.preventDefault();
            e.stopPropagation();

            if (!e.dataTransfer) return;

            const files = Array.from(e.dataTransfer.files);
            const imageFile = files.find(file => file.type.startsWith('image/'));

            if (imageFile) {
                this.handleFile(imageFile);
            }
        });
    }

    private handleImageUpload(event: Event): void {
        const target = event.target as HTMLInputElement;
        const file = target.files?.[0];
        if (file) {
            this.handleFile(file);
        }
    }

    private async handleFile(file: File): Promise<void> {
        if (!file.type.startsWith('image/')) {
            this.updateStatus('Please select a valid image file.');
            return;
        }

        this.currentFile = file;

        try {
            const image = new Image();
            image.onload = async () => {
                this.currentImage = image;
                this.displayImage();
                this.updateStats();
                this.enableControls();
                this.updateStatus(`Image loaded: ${file.name}`);
                this.addToLog(`Image loaded: ${file.name} (${image.width}×${image.height})`);

                // Automatically detect faces
                await this.detectFaces();
            };
            image.src = URL.createObjectURL(file);
        } catch (error) {
            console.error('Error loading image:', error);
            this.updateStatus('Error loading image.');
            this.addToLog('Error loading image: ' + (error as Error).message, 'error');
        }
    }

    private displayImage(): void {
        if (!this.currentImage) return;

        const canvas = this.inputCanvas;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Set canvas size based on rotation
        const maxWidth = 600;
        const maxHeight = 400;

        let displayWidth = this.currentImage.width;
        let displayHeight = this.currentImage.height;

        // Swap dimensions for 90 or 270 degree rotations
        if (this.rotationAngle === 90 || this.rotationAngle === 270) {
            displayWidth = this.currentImage.height;
            displayHeight = this.currentImage.width;
        }

        const scale = Math.min(maxWidth / displayWidth, maxHeight / displayHeight);
        canvas.width = displayWidth * scale;
        canvas.height = displayHeight * scale;

        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        // Save context and apply rotation
        ctx.save();

        // Move to center of canvas
        ctx.translate(canvas.width / 2, canvas.height / 2);

        // Rotate
        ctx.rotate((this.rotationAngle * Math.PI) / 180);

        // Draw image centered at origin
        ctx.drawImage(
            this.currentImage,
            -this.currentImage.width * scale / 2,
            -this.currentImage.height * scale / 2,
            this.currentImage.width * scale,
            this.currentImage.height * scale
        );

        ctx.restore();

        this.canvasContainer.classList.remove('hidden');
        this.clearFaceOverlays();
    }

    private rotateImage(degrees: number): void {
        if (!this.currentImage) return;

        // Update rotation angle (keep it in range 0-359)
        this.rotationAngle = (this.rotationAngle + degrees + 360) % 360;

        // Clear any detected faces as they're no longer valid
        this.faces = [];
        this.selectedFaces.clear();
        this.updateFaceCount();

        // Create a new rotated image
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        if (!tempCtx) return;

        // Set temp canvas size based on rotation
        if (this.rotationAngle === 90 || this.rotationAngle === 270) {
            tempCanvas.width = this.currentImage.height;
            tempCanvas.height = this.currentImage.width;
        } else {
            tempCanvas.width = this.currentImage.width;
            tempCanvas.height = this.currentImage.height;
        }

        // Apply rotation
        tempCtx.save();
        tempCtx.translate(tempCanvas.width / 2, tempCanvas.height / 2);
        tempCtx.rotate((this.rotationAngle * Math.PI) / 180);
        tempCtx.drawImage(
            this.currentImage,
            -this.currentImage.width / 2,
            -this.currentImage.height / 2
        );
        tempCtx.restore();

        // Create new image from rotated canvas
        const newImage = new Image();
        newImage.onload = async () => {
            this.currentImage = newImage;
            this.rotationAngle = 0; // Reset rotation angle since we've created a new rotated image
            this.displayImage();
            this.updateStats();
            this.updateStatus(`Image rotated ${degrees > 0 ? 'clockwise' : 'counter-clockwise'} by 90°`);
            this.addToLog(`Image rotated ${degrees > 0 ? 'clockwise' : 'counter-clockwise'} by 90°`);

            // Automatically re-detect faces after rotation
            await this.detectFaces();
        };
        newImage.src = tempCanvas.toDataURL();
    }

    private async detectFaces(): Promise<void> {
        if ((!this.detector && !this.faceLandmarker) || !this.currentImage) {
            this.updateStatus('Model not loaded or no image selected.');
            return;
        }

        this.updateStatus('Detecting faces...');
        this.addToLog('Starting face detection...');
        this.processingStartTime = Date.now();

        try {
            // Use the base class method that handles both Face Landmarker and Face Detector
            this.faces = await this.detectFacesWithLandmarks(this.currentImage);

            this.selectedFaces = new Set(this.faces.map(f => f.id));
            this.updateFaceOverlays();
            this.updateFaceCount();

            const processingTime = Date.now() - this.processingStartTime;
            const detectionMethod = this.useSegmentation && this.faceLandmarker ? 'with segmentation' : 'with bounding boxes';
            this.updateStatus(`Detected ${this.faces.length} face(s) ${detectionMethod} in ${processingTime}ms`);
            this.addToLog(`Face detection completed: ${this.faces.length} faces found in ${processingTime}ms`);

            this.updateStats();

            if (this.faces.length > 0) {
                if (this.processImageBtn) {
                    this.processImageBtn.disabled = false;
                }
                // Automatically process the detected faces
                await this.processImage();
            }
        } catch (error) {
            console.error('Error detecting faces:', error);
            this.updateStatus('Error detecting faces.');
            this.addToLog('Error during face detection: ' + (error as Error).message, 'error');
        }
    }

    private updateFaceOverlays(): void {
        this.clearFaceOverlays();

        if (!this.faces.length || !this.currentImage) return;

        const canvas = this.inputCanvas;
        // Use uniform scale like the display image does
        const scale = canvas.width / this.currentImage.width;

        const panelContent = this.inputCanvas.parentElement!;
        const style = getComputedStyle(panelContent);
        const paddingLeft = parseFloat(style.paddingLeft);
        const paddingTop = parseFloat(style.paddingTop);

        this.faces.forEach((face) => {
            const box = face.box!;
            const overlay = document.createElement('div');
            overlay.className = 'face-box';
            overlay.style.left = ((box.xMin * scale) + paddingLeft) + 'px';
            overlay.style.top = ((box.yMin * scale) + paddingTop) + 'px';
            overlay.style.width = (box.width * scale) + 'px';
            overlay.style.height = (box.height * scale) + 'px';

            if (this.selectedFaces.has(face.id)) {
                overlay.classList.add('selected');
            } else {
                overlay.classList.add('unselected');
            }

            overlay.addEventListener('click', () => this.toggleFaceSelection(face.id));

            // Add mousedown for dragging the face box
            overlay.addEventListener('mousedown', (e) => {
                // Only start drag if clicking on the box itself, not on resize handles
                if ((e.target as HTMLElement).classList.contains('face-box')) {
                    this.startDrag(e, face.id);
                }
            });

            // Add resize handles for interactive manipulation
            const handles = ['nw', 'ne', 'sw', 'se'].map(pos => {
                const handle = document.createElement('div');
                handle.className = `resize-handle ${pos}`;
                handle.dataset.position = pos;
                handle.addEventListener('mousedown', (e) => this.startResize(e, face.id));
                return handle;
            });

            handles.forEach(handle => overlay.appendChild(handle));

            this.faceOverlays.appendChild(overlay);
        });
    }

    private startDrag(e: MouseEvent, faceId: string | number): void {
        e.preventDefault();
        e.stopPropagation();

        const overlay = e.currentTarget as HTMLElement;
        const face = this.faces.find(f => f.id === faceId);
        if (!face || !face.box) return;

        const startX = e.clientX;
        const startY = e.clientY;
        const startLeft = overlay.offsetLeft;
        const startTop = overlay.offsetTop;

        const canvas = this.inputCanvas;
        const scale = canvas.width / this.currentImage!.width;

        const panelContent = this.inputCanvas.parentElement!;
        const style = getComputedStyle(panelContent);
        const paddingLeft = parseFloat(style.paddingLeft);
        const paddingTop = parseFloat(style.paddingTop);

        // Change cursor to grabbing
        overlay.style.cursor = 'grabbing';

        const handleDrag = (moveEvent: MouseEvent) => {
            const dx = moveEvent.clientX - startX;
            const dy = moveEvent.clientY - startY;

            const newLeft = startLeft + dx;
            const newTop = startTop + dy;

            // Update overlay position
            overlay.style.left = newLeft + 'px';
            overlay.style.top = newTop + 'px';

            // Update face data
            if (face.box) {
                face.box.xMin = (newLeft - paddingLeft) / scale;
                face.box.yMin = (newTop - paddingTop) / scale;
            }
        };

        const stopDrag = () => {
            document.removeEventListener('mousemove', handleDrag);
            document.removeEventListener('mouseup', stopDrag);
            // Restore cursor
            overlay.style.cursor = 'move';
            // Trigger auto-reprocess after drag ends
            this.autoReprocess();
        };

        document.addEventListener('mousemove', handleDrag);
        document.addEventListener('mouseup', stopDrag);
    }

    private startResize(e: MouseEvent, faceId: string | number): void {
        e.preventDefault();
        e.stopPropagation();

        const target = e.target as HTMLElement;
        const overlay = target.parentElement;
        if (!overlay) return;

        const face = this.faces.find(f => f.id === faceId);
        if (!face || !face.box) return;

        const startX = e.clientX;
        const startY = e.clientY;
        const startLeft = overlay.offsetLeft;
        const startTop = overlay.offsetTop;
        const startWidth = overlay.offsetWidth;
        const startHeight = overlay.offsetHeight;
        const {position} = target.dataset;

        const canvas = this.inputCanvas;
        // Use uniform scale like the display image does
        const scale = canvas.width / this.currentImage!.width;

        const panelContent = this.inputCanvas.parentElement!;
        const style = getComputedStyle(panelContent);
        const paddingLeft = parseFloat(style.paddingLeft);
        const paddingTop = parseFloat(style.paddingTop);

        const handleResize = (moveEvent: MouseEvent) => {
            const dx = moveEvent.clientX - startX;
            const dy = moveEvent.clientY - startY;

            let newLeft = startLeft;
            let newTop = startTop;
            let newWidth = startWidth;
            let newHeight = startHeight;

            switch (position) {
                case 'nw':
                    newLeft = startLeft + dx;
                    newTop = startTop + dy;
                    newWidth = startWidth - dx;
                    newHeight = startHeight - dy;
                    break;
                case 'ne':
                    newTop = startTop + dy;
                    newWidth = startWidth + dx;
                    newHeight = startHeight - dy;
                    break;
                case 'sw':
                    newLeft = startLeft + dx;
                    newWidth = startWidth - dx;
                    newHeight = startHeight + dy;
                    break;
                case 'se':
                    newWidth = startWidth + dx;
                    newHeight = startHeight + dy;
                    break;
            }

            // Enforce minimum size
            const minSize = 20;
            if (newWidth >= minSize && newHeight >= minSize) {
                overlay.style.left = newLeft + 'px';
                overlay.style.top = newTop + 'px';
                overlay.style.width = newWidth + 'px';
                overlay.style.height = newHeight + 'px';

                // Update the face box coordinates using uniform scale
                face.box!.xMin = (newLeft - paddingLeft) / scale;
                face.box!.yMin = (newTop - paddingTop) / scale;
                face.box!.width = newWidth / scale;
                face.box!.height = newHeight / scale;
                face.x = face.box!.xMin;
                face.y = face.box!.yMin;
                face.width = face.box!.width;
                face.height = face.box!.height;
            }
        };

        const stopResize = () => {
            document.removeEventListener('mousemove', handleResize);
            document.removeEventListener('mouseup', stopResize);
            // Trigger auto-reprocess after resize ends
            this.autoReprocess();
        };

        document.addEventListener('mousemove', handleResize);
        document.addEventListener('mouseup', stopResize);
    }

    private clearFaceOverlays(): void {
        this.faceOverlays.innerHTML = '';
    }

    toggleFaceSelection(faceId: string | number): void {
        if (this.selectedFaces.has(faceId)) {
            this.selectedFaces.delete(faceId);
        } else {
            this.selectedFaces.add(faceId);
        }
        this.updateFaceOverlays();
        this.updateFaceCount();
        this.autoReprocess();
    }

    selectAllFaces(): void {
        this.selectedFaces = new Set(this.faces.map(f => f.id));
        this.updateFaceOverlays();
        this.updateFaceCount();
        this.autoReprocess();
    }

    selectNoneFaces(): void {
        this.selectedFaces.clear();
        this.updateFaceOverlays();
        this.updateFaceCount();
        this.autoReprocess();
    }

    private updateFaceCount(): void {
        this.faceCount.textContent = this.faces.length.toString();
        this.selectedFaceCount.textContent = this.selectedFaces.size.toString();
    }

    /**
     * Automatically reprocess the image with a debounce delay
     * This is called when settings change or faces are selected/deselected
     */
    private autoReprocess(): void {
        // Only auto-reprocess if we have faces and at least one is selected
        if (!this.faces.length || !this.selectedFaces.size) {
            return;
        }

        // Clear any pending auto-reprocess timer
        if (this.autoReprocessTimer !== null) {
            clearTimeout(this.autoReprocessTimer);
        }

        // Schedule a new auto-reprocess
        this.autoReprocessTimer = window.setTimeout(() => {
            this.processImage();
            this.autoReprocessTimer = null;
        }, this.AUTO_REPROCESS_DELAY);
    }

    private async processImage(): Promise<void> {
        if (!this.detector || !this.currentImage || !this.selectedFaces.size) {
            this.updateStatus('No faces selected for processing.');
            return;
        }

        this.updateStatus('Processing selected faces...');
        this.addToLog(`Processing ${this.selectedFaces.size} selected faces...`);
        this.showProgress();

        const selectedFaceData = this.faces.filter(face => this.selectedFaces.has(face.id));
        const croppedResults: CroppedResult[] = [];

        try {
            for (let i = 0; i < selectedFaceData.length; i++) {
                const face = selectedFaceData[i];
                this.updateProgress((i / selectedFaceData.length) * 100, `Processing face ${i + 1}/${selectedFaceData.length}`);

                const croppedImage = await this.cropCurrentFace(face);
                croppedResults.push({
                    face,
                    image: croppedImage,
                    index: i
                });
            }

            this.displayResults(croppedResults);
            this.updateProgress(100, 'Processing complete!');
            this.hideProgress();
            this.updateStatus(`Successfully processed ${croppedResults.length} face(s)`);
            this.addToLog(`Processing completed: ${croppedResults.length} faces cropped successfully`);

            this.downloadResultsBtn.disabled = false;
        } catch (error) {
            console.error('Error processing faces:', error);
            this.updateStatus('Error processing faces.');
            this.addToLog('Error during face processing: ' + (error as Error).message, 'error');
            this.hideProgress();
        }
    }

    private async cropCurrentFace(face: FaceData): Promise<HTMLCanvasElement> {
        const settings = this.getSettings();
        const box = face.box!;

        // Calculate how much to scale based on face height percentage
        const faceHeight = settings.outputHeight * (settings.faceHeightPct / 100);
        const scale = faceHeight / box.height;

        // Calculate crop dimensions maintaining aspect ratio
        let cropWidth = settings.outputWidth / scale;
        let cropHeight = settings.outputHeight / scale;

        // Center the crop on the face box (no extra positioning logic for manual adjustments)
        let centerX = box.xMin + (box.width / 2);
        let centerY = box.yMin + (box.height / 2);

        let cropX = centerX - (cropWidth / 2);
        let cropY = centerY - (cropHeight / 2);

        // Clamp to image boundaries
        cropX = Math.max(0, Math.min(cropX, this.currentImage!.width - cropWidth));
        cropY = Math.max(0, Math.min(cropY, this.currentImage!.height - cropHeight));

        const canvas = document.createElement('canvas');
        canvas.width = settings.outputWidth;
        canvas.height = settings.outputHeight;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });

        if (!ctx) {
            throw new Error('Failed to get canvas 2d context');
        }

        ctx.drawImage(
            this.currentImage!,
            cropX, cropY, cropWidth, cropHeight,
            0, 0, settings.outputWidth, settings.outputHeight
        );

        await this.applyEnhancements(ctx, canvas);

        return canvas;
    }

    private displayResults(results: CroppedResult[]): void {
        this.croppedContainer.innerHTML = '';

        results.forEach((result, index) => {
            const container = document.createElement('div');
            container.className = 'cropped-face-item';

            const canvas = result.image;

            // Scale the canvas to fit the UI while maintaining aspect ratio
            const maxDisplayWidth = 300;
            const maxDisplayHeight = 300;
            const scale = Math.min(
                maxDisplayWidth / canvas.width,
                maxDisplayHeight / canvas.height,
                1 // Don't scale up, only scale down
            );

            canvas.style.width = (canvas.width * scale) + 'px';
            canvas.style.height = (canvas.height * scale) + 'px';
            canvas.style.display = 'block';
            canvas.style.margin = '0 auto';
            canvas.className = 'cropped-face';

            const controls = document.createElement('div');
            controls.className = 'cropped-face-controls';

            const downloadBtn = document.createElement('button');
            downloadBtn.textContent = 'Download';
            downloadBtn.className = 'small-btn';
            downloadBtn.onclick = () => this.downloadSingle(canvas, index);

            controls.appendChild(downloadBtn);
            container.appendChild(canvas);
            container.appendChild(controls);
            this.croppedContainer.appendChild(container);
        });

        this.croppedFaces.classList.remove('hidden');
    }

    private downloadSingle(canvas: HTMLCanvasElement, index: number): void {
        const settings = this.getSettings();
        const filename = this.generateFaceFilename(index);

        canvas.toBlob((blob) => {
            if (!blob) return;
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = filename;
            a.click();
            URL.revokeObjectURL(url);
        }, `image/${settings.outputFormat}`, settings.quality);
    }

    private downloadResults(): void {
        const canvases = this.croppedContainer.querySelectorAll('canvas');
        if (canvases.length === 0) return;

        if (canvases.length === 1) {
            this.downloadSingle(canvases[0] as HTMLCanvasElement, 0);
        } else {
            this.downloadAsZip(Array.from(canvases) as HTMLCanvasElement[]);
        }
    }

    private generateFaceFilename(index: number): string {
        const settings = this.getSettings();
        const template = this.namingTemplate.value || 'face_{original}_{index}';
        const originalName = this.currentFile ? this.currentFile.name.replace(/\.[^/.]+$/, '') : 'image';
        const extension = settings.outputFormat === 'jpeg' ? 'jpg' : settings.outputFormat;

        return template
            .replace('{original}', originalName)
            .replace('{index}', (index + 1).toString())
            .replace('{timestamp}', Date.now().toString())
            .replace('{width}', settings.outputWidth.toString())
            .replace('{height}', settings.outputHeight.toString()) + '.' + extension;
    }

    private clearImage(): void {
        this.currentImage = null;
        this.currentFile = null;
        this.faces = [];
        this.selectedFaces.clear();
        this.rotationAngle = 0; // Reset rotation
        this.imageInput.value = '';

        this.canvasContainer.classList.add('hidden');
        this.croppedFaces.classList.add('hidden');
        this.clearFaceOverlays();
        this.croppedContainer.innerHTML = '';

        this.updateUI();
        this.updateStatus('Ready to load image');
        this.addToLog('Image cleared');
    }

    updateUI(): void {
        const hasImage = !!this.currentImage;
        const hasFaces = this.faces.length > 0;
        const hasSelectedFaces = this.selectedFaces.size > 0;

        if (this.processImageBtn) {
            this.processImageBtn.disabled = !hasFaces || !hasSelectedFaces;
        }
        this.clearImageBtn.disabled = !hasImage;
        if (this.detectFacesBtn) {
            this.detectFacesBtn.disabled = !hasImage;
        }
        this.rotateClockwiseBtn.disabled = !hasImage;
        this.rotateCounterClockwiseBtn.disabled = !hasImage;
        this.downloadResultsBtn.disabled = true;
    }

    private enableControls(): void {
        this.updateUI();
    }

    private updateStats(): void {
        if (this.currentImage) {
            this.imageSize.textContent = `${this.currentImage.width}×${this.currentImage.height}`;
        }
        this.facesDetected.textContent = this.faces.length.toString();

        if (this.processingStartTime) {
            const time = Date.now() - this.processingStartTime;
            this.processingTime.textContent = time + 'ms';
        }
    }
}

// Instantiate immediately - module is loaded dynamically after DOMContentLoaded
new SingleImageFaceCropper();

export default SingleImageFaceCropper;