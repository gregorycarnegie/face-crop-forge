class SingleImageFaceCropper extends BaseFaceCropper {
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

    initializeElements() {
        this.imageInput = document.getElementById('imageInput');
        this.processImageBtn = document.getElementById('processImageBtn');
        this.clearImageBtn = document.getElementById('clearImageBtn');
        this.downloadResultsBtn = document.getElementById('downloadResultsBtn');
        this.progressSection = document.getElementById('progressSection');
        this.progressFill = document.getElementById('progressFill');
        this.progressText = document.getElementById('progressText');
        this.canvasContainer = document.getElementById('canvasContainer');
        this.inputCanvas = document.getElementById('inputCanvas');
        this.outputCanvas = document.getElementById('outputCanvas');
        this.croppedFaces = document.getElementById('croppedFaces');
        this.croppedContainer = document.getElementById('croppedContainer');
        this.status = document.getElementById('status');
        this.faceOverlays = document.getElementById('faceOverlays');
        this.faceCount = document.getElementById('faceCount');
        this.selectedFaceCount = document.getElementById('selectedFaceCount');
        this.selectAllFacesBtn = document.getElementById('selectAllFacesBtn');
        this.selectNoneFacesBtn = document.getElementById('selectNoneFacesBtn');
        this.detectFacesBtn = document.getElementById('detectFacesBtn');

        // Settings elements
        this.outputWidth = document.getElementById('outputWidth');
        this.outputHeight = document.getElementById('outputHeight');
        this.faceHeightPct = document.getElementById('faceHeightPct');
        this.positioningMode = document.getElementById('positioningMode');
        this.verticalOffset = document.getElementById('verticalOffset');
        this.horizontalOffset = document.getElementById('horizontalOffset');
        this.aspectRatioLock = document.getElementById('aspectRatioLock');
        this.sizePreset = document.getElementById('sizePreset');
        this.outputFormat = document.getElementById('outputFormat');
        this.jpegQuality = document.getElementById('jpegQuality');
        this.namingTemplate = document.getElementById('namingTemplate');

        // Stats elements
        this.facesDetected = document.getElementById('facesDetected');
        this.processingTime = document.getElementById('processingTime');
        this.imageSize = document.getElementById('imageSize');
        this.processingStatus = document.getElementById('processingStatus');
        this.processingLog = document.getElementById('processingLog');

        // Enhancement elements
        this.autoColorCorrection = document.getElementById('autoColorCorrection');
        this.exposureAdjustment = document.getElementById('exposureAdjustment');
        this.contrastAdjustment = document.getElementById('contrastAdjustment');
        this.sharpnessControl = document.getElementById('sharpnessControl');
        this.skinSmoothing = document.getElementById('skinSmoothing');
        this.redEyeRemoval = document.getElementById('redEyeRemoval');
        this.backgroundBlur = document.getElementById('backgroundBlur');

        this.updateUI();
    }

    setupEventListeners() {
        this.imageInput.addEventListener('change', (e) => this.handleImageUpload(e));
        this.processImageBtn.addEventListener('click', () => this.processImage());
        this.clearImageBtn.addEventListener('click', () => this.clearImage());
        this.downloadResultsBtn.addEventListener('click', () => this.downloadResults());

        // Face selection listeners
        this.selectAllFacesBtn.addEventListener('click', () => this.selectAllFaces());
        this.selectNoneFacesBtn.addEventListener('click', () => this.selectNoneFaces());
        this.detectFacesBtn.addEventListener('click', () => this.detectFaces());

        // Settings listeners
        this.outputWidth.addEventListener('input', () => this.updatePreview());
        this.outputHeight.addEventListener('input', () => this.updatePreview());
        this.faceHeightPct.addEventListener('input', () => this.updatePreview());
        this.positioningMode.addEventListener('change', () => this.updateAdvancedPositioning());
        this.verticalOffset.addEventListener('input', () => this.updateOffsetDisplay('vertical'));
        this.horizontalOffset.addEventListener('input', () => this.updateOffsetDisplay('horizontal'));
        this.aspectRatioLock.addEventListener('click', () => this.toggleAspectRatioLock());
        this.sizePreset.addEventListener('change', () => this.applySizePreset());
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

        // Enhancement listeners
        if (this.exposureAdjustment) {
            this.exposureAdjustment.addEventListener('input', () => this.updateSliderValue('exposure'));
        }
        if (this.contrastAdjustment) {
            this.contrastAdjustment.addEventListener('input', () => this.updateSliderValue('contrast'));
        }
        if (this.sharpnessControl) {
            this.sharpnessControl.addEventListener('input', () => this.updateSliderValue('sharpness'));
        }
        if (this.skinSmoothing) {
            this.skinSmoothing.addEventListener('input', () => this.updateSliderValue('skinSmoothing'));
        }
        if (this.backgroundBlur) {
            this.backgroundBlur.addEventListener('input', () => this.updateSliderValue('backgroundBlur'));
        }

        // Drag and drop
        document.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.stopPropagation();
        });

        document.addEventListener('drop', (e) => {
            e.preventDefault();
            e.stopPropagation();

            const files = Array.from(e.dataTransfer.files);
            const imageFile = files.find(file => file.type.startsWith('image/'));

            if (imageFile) {
                this.handleFile(imageFile);
            }
        });
    }

    handleImageUpload(event) {
        const file = event.target.files[0];
        if (file) {
            this.handleFile(file);
        }
    }

    async handleFile(file) {
        if (!file.type.startsWith('image/')) {
            this.updateStatus('Please select a valid image file.');
            return;
        }

        this.currentFile = file;

        try {
            const image = new Image();
            image.onload = () => {
                this.currentImage = image;
                this.displayImage();
                this.updateStats();
                this.enableControls();
                this.updateStatus(`Image loaded: ${file.name}`);
                this.addToLog(`Image loaded: ${file.name} (${image.width}×${image.height})`);
            };
            image.src = URL.createObjectURL(file);
        } catch (error) {
            console.error('Error loading image:', error);
            this.updateStatus('Error loading image.');
            this.addToLog('Error loading image: ' + error.message, 'error');
        }
    }

    displayImage() {
        if (!this.currentImage) return;

        const canvas = this.inputCanvas;
        const ctx = canvas.getContext('2d');

        // Set canvas size
        const maxWidth = 600;
        const maxHeight = 400;
        const scale = Math.min(maxWidth / this.currentImage.width, maxHeight / this.currentImage.height);

        canvas.width = this.currentImage.width * scale;
        canvas.height = this.currentImage.height * scale;

        // Draw image
        ctx.drawImage(this.currentImage, 0, 0, canvas.width, canvas.height);

        this.canvasContainer.classList.remove('hidden');
        this.clearFaceOverlays();
    }

    async detectFaces() {
        if (!this.detector || !this.currentImage) {
            this.updateStatus('Model not loaded or no image selected.');
            return;
        }

        this.updateStatus('Detecting faces...');
        this.addToLog('Starting face detection...');
        this.processingStartTime = Date.now();

        try {
            const detectionResult = await this.detector.detect(this.currentImage);

            this.faces = [];
            if (detectionResult.detections && detectionResult.detections.length > 0) {
                this.faces = detectionResult.detections
                    .map((detection, index) => {
                        const bbox = detection.boundingBox;
                        const box = this.convertBoundingBoxToPixels(bbox, this.currentImage.width, this.currentImage.height);
                        if (!box) {
                            return null;
                        }

                        const { x, y, width, height } = box;

                        return {
                            id: index,
                            box: {
                                xMin: x,
                                yMin: y,
                                width: width,
                                height: height
                            },
                            confidence: detection.categories && detection.categories.length > 0
                                ? detection.categories[0].score
                                : 0.8,
                            selected: true
                        };
                    })
                    .filter(Boolean);
            }

            this.selectedFaces = new Set(this.faces.map(f => f.id));
            this.updateFaceOverlays();
            this.updateFaceCount();

            const processingTime = Date.now() - this.processingStartTime;
            this.updateStatus(`Detected ${this.faces.length} face(s) in ${processingTime}ms`);
            this.addToLog(`Face detection completed: ${this.faces.length} faces found in ${processingTime}ms`);

            this.updateStats();

            if (this.faces.length > 0) {
                this.processImageBtn.disabled = false;
            }
        } catch (error) {
            console.error('Error detecting faces:', error);
            this.updateStatus('Error detecting faces.');
            this.addToLog('Error during face detection: ' + error.message, 'error');
        }
    }

    updateFaceOverlays() {
        this.clearFaceOverlays();

        if (!this.faces.length) return;

        const canvas = this.inputCanvas;
        const canvasRect = canvas.getBoundingClientRect();
        const scaleX = canvas.width / this.currentImage.width;
        const scaleY = canvas.height / this.currentImage.height;

        this.faces.forEach((face, index) => {
            const box = face.box;
            const overlay = document.createElement('div');
            overlay.className = `face-overlay ${this.selectedFaces.has(face.id) ? 'selected' : ''}`;
            overlay.style.left = (box.xMin * scaleX) + 'px';
            overlay.style.top = (box.yMin * scaleY) + 'px';
            overlay.style.width = (box.width * scaleX) + 'px';
            overlay.style.height = (box.height * scaleY) + 'px';

            overlay.addEventListener('click', () => this.toggleFaceSelection(face.id));

            this.faceOverlays.appendChild(overlay);
        });
    }

    clearFaceOverlays() {
        this.faceOverlays.innerHTML = '';
    }

    toggleFaceSelection(faceId) {
        if (this.selectedFaces.has(faceId)) {
            this.selectedFaces.delete(faceId);
        } else {
            this.selectedFaces.add(faceId);
        }
        this.updateFaceOverlays();
        this.updateFaceCount();
    }

    selectAllFaces() {
        this.selectedFaces = new Set(this.faces.map(f => f.id));
        this.updateFaceOverlays();
        this.updateFaceCount();
    }

    selectNoneFaces() {
        this.selectedFaces.clear();
        this.updateFaceOverlays();
        this.updateFaceCount();
    }

    updateFaceCount() {
        this.faceCount.textContent = this.faces.length;
        this.selectedFaceCount.textContent = this.selectedFaces.size;
    }

    async processImage() {
        if (!this.detector || !this.currentImage || !this.selectedFaces.size) {
            this.updateStatus('No faces selected for processing.');
            return;
        }

        this.updateStatus('Processing selected faces...');
        this.addToLog(`Processing ${this.selectedFaces.size} selected faces...`);
        this.showProgress();

        const selectedFaceData = this.faces.filter(face => this.selectedFaces.has(face.id));
        const croppedResults = [];

        try {
            for (let i = 0; i < selectedFaceData.length; i++) {
                const face = selectedFaceData[i];
                this.updateProgress((i / selectedFaceData.length) * 100, `Processing face ${i + 1}/${selectedFaceData.length}`);

                const croppedImage = await this.cropFace(face);
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
            this.addToLog('Error during face processing: ' + error.message, 'error');
            this.hideProgress();
        }
    }

    async cropFace(face) {
        return super.cropFace(this.currentImage, face);
    }

    displayResults(results) {
        this.croppedContainer.innerHTML = '';

        results.forEach((result, index) => {
            const container = document.createElement('div');
            container.className = 'cropped-face-item';

            const canvas = result.image;
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

    downloadSingle(canvas, index) {
        const settings = this.getSettings();
        const filename = this.generateFilename(index);

        canvas.toBlob((blob) => {
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = filename;
            a.click();
            URL.revokeObjectURL(url);
        }, `image/${settings.format}`, settings.quality);
    }

    downloadResults() {
        const canvases = this.croppedContainer.querySelectorAll('canvas');
        if (canvases.length === 0) return;

        if (canvases.length === 1) {
            this.downloadSingle(canvases[0], 0);
        } else {
            this.downloadAsZip(Array.from(canvases));
        }
    }

    generateFilename(index) {
        const settings = this.getSettings();
        const template = this.namingTemplate.value || 'face_{original}_{index}';
        const originalName = this.currentFile ? this.currentFile.name.replace(/\.[^/.]+$/, '') : 'image';
        const extension = settings.format === 'jpeg' ? 'jpg' : settings.format;

        return template
            .replace('{original}', originalName)
            .replace('{index}', index + 1)
            .replace('{timestamp}', Date.now())
            .replace('{width}', settings.outputWidth)
            .replace('{height}', settings.outputHeight) + '.' + extension;
    }

    clearImage() {
        this.currentImage = null;
        this.currentFile = null;
        this.faces = [];
        this.selectedFaces.clear();
        this.imageInput.value = '';

        this.canvasContainer.classList.add('hidden');
        this.croppedFaces.classList.add('hidden');
        this.clearFaceOverlays();
        this.croppedContainer.innerHTML = '';

        this.updateUI();
        this.updateStatus('Ready to load image');
        this.addToLog('Image cleared');
    }

    updateUI() {
        const hasImage = !!this.currentImage;
        const hasFaces = this.faces.length > 0;
        const hasSelectedFaces = this.selectedFaces.size > 0;

        this.processImageBtn.disabled = !hasFaces || !hasSelectedFaces;
        this.clearImageBtn.disabled = !hasImage;
        this.detectFacesBtn.disabled = !hasImage;
        this.downloadResultsBtn.disabled = true;
    }

    enableControls() {
        this.updateUI();
    }

    updateStats() {
        if (this.currentImage) {
            this.imageSize.textContent = `${this.currentImage.width}×${this.currentImage.height}`;
        }
        this.facesDetected.textContent = this.faces.length;

        if (this.processingStartTime) {
            const time = Date.now() - this.processingStartTime;
            this.processingTime.textContent = time + 'ms';
        }
    }
}

// Initialize the application when the page loads
document.addEventListener('DOMContentLoaded', () => {
    new SingleImageFaceCropper();
});