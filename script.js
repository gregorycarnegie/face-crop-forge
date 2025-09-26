class FaceCropper {
    constructor() {
        this.detector = null;
        this.images = new Map(); // imageId -> { file, image, faces, results, selected, processed }
        this.processingQueue = [];
        this.isProcessing = false;
        this.currentProcessingId = null;
        this.aspectRatioLocked = false;
        this.currentAspectRatio = 1;

        this.initializeElements();
        this.setupEventListeners();
        this.loadModel();
    }

    initializeElements() {
        this.imageInput = document.getElementById('imageInput');
        this.processAllBtn = document.getElementById('processAllBtn');
        this.processSelectedBtn = document.getElementById('processSelectedBtn');
        this.clearAllBtn = document.getElementById('clearAllBtn');
        this.downloadAllBtn = document.getElementById('downloadAllBtn');

        this.progressSection = document.getElementById('progressSection');
        this.progressFill = document.getElementById('progressFill');
        this.progressText = document.getElementById('progressText');

        this.imageGallery = document.getElementById('imageGallery');
        this.galleryGrid = document.getElementById('galleryGrid');
        this.selectAllBtn = document.getElementById('selectAllBtn');
        this.selectNoneBtn = document.getElementById('selectNoneBtn');
        this.selectedCount = document.getElementById('selectedCount');
        this.totalCount = document.getElementById('totalCount');

        this.canvasContainer = document.getElementById('canvasContainer');
        this.inputCanvas = document.getElementById('inputCanvas');
        this.outputCanvas = document.getElementById('outputCanvas');
        this.faceOverlays = document.getElementById('faceOverlays');
        this.faceCount = document.getElementById('faceCount');
        this.selectedFaceCount = document.getElementById('selectedFaceCount');
        this.croppedContainer = document.getElementById('croppedContainer');
        this.status = document.getElementById('status');

        // Face selection controls
        this.selectAllFacesBtn = document.getElementById('selectAllFacesBtn');
        this.selectNoneFacesBtn = document.getElementById('selectNoneFacesBtn');
        this.detectFacesBtn = document.getElementById('detectFacesBtn');

        // Crop settings elements
        this.outputWidth = document.getElementById('outputWidth');
        this.outputHeight = document.getElementById('outputHeight');
        this.faceHeightPct = document.getElementById('faceHeightPct');
        this.previewText = document.getElementById('previewText');

        // Smart cropping elements
        this.sizePreset = document.getElementById('sizePreset');
        this.aspectRatioLock = document.getElementById('aspectRatioLock');
        this.positioningMode = document.getElementById('positioningMode');
        this.verticalOffset = document.getElementById('verticalOffset');
        this.horizontalOffset = document.getElementById('horizontalOffset');
        this.verticalOffsetValue = document.getElementById('verticalOffsetValue');
        this.horizontalOffsetValue = document.getElementById('horizontalOffsetValue');
        this.advancedPositioning = document.getElementById('advancedPositioning');
        this.applyToAllBtn = document.getElementById('applyToAllBtn');
        this.resetSettingsBtn = document.getElementById('resetSettingsBtn');
        this.aspectRatioText = document.getElementById('aspectRatioText');

        // Output settings elements
        this.outputFormat = document.getElementById('outputFormat');
        this.jpegQuality = document.getElementById('jpegQuality');
        this.jpegQualityGroup = document.getElementById('jpegQualityGroup');
        this.qualityValue = document.getElementById('qualityValue');
        this.namingTemplate = document.getElementById('namingTemplate');
        this.zipDownload = document.getElementById('zipDownload');
        this.individualDownload = document.getElementById('individualDownload');

        this.ctx = this.inputCanvas.getContext('2d');
    }

    setupEventListeners() {
        this.imageInput.addEventListener('change', (e) => this.handleMultipleImageUpload(e));
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

        // Initialize controls
        this.updatePreview();
        this.updateFormatControls();
        this.updateQualityDisplay();
        this.updatePositioningControls();
        this.updateOffsetDisplays();
    }

    updateStatus(message, type = '') {
        this.status.textContent = message;
        this.status.className = `status ${type}`;
    }

    updatePreview() {
        const width = parseInt(this.outputWidth.value);
        const height = parseInt(this.outputHeight.value);
        const faceHeight = parseInt(this.faceHeightPct.value);
        const format = this.outputFormat.value.toUpperCase();
        const positionMode = this.positioningMode.value;

        this.previewText.textContent = `${width}×${height}px, face at ${faceHeight}% height, ${format} format, ${positionMode} positioning`;
        this.updateAspectRatioDisplay();
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
        this.jpegQualityGroup.style.display = format === 'jpeg' ? 'flex' : 'none';
        this.updatePreview();
    }

    updateQualityDisplay() {
        this.qualityValue.textContent = this.jpegQuality.value + '%';
    }

    toggleIndividualDownloadButtons() {
        // Refresh face overlays to show/hide individual download buttons
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
        if (selectedImages.length > 0 && selectedImages[0].faces) {
            this.displayImageWithFaceOverlays(selectedImages[0]);
        }
    }

    // Smart cropping methods
    applyPreset() {
        const preset = this.sizePreset.value;
        const presets = {
            custom: { width: 256, height: 256 },
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

    toggleAspectRatioLock() {
        this.aspectRatioLocked = !this.aspectRatioLocked;

        if (this.aspectRatioLocked) {
            this.aspectRatioLock.classList.add('locked');
            this.aspectRatioLock.textContent = '🔒';
            this.aspectRatioLock.title = 'Unlock aspect ratio';
            this.currentAspectRatio = parseInt(this.outputWidth.value) / parseInt(this.outputHeight.value);
        } else {
            this.aspectRatioLock.classList.remove('locked');
            this.aspectRatioLock.textContent = '🔓';
            this.aspectRatioLock.title = 'Lock aspect ratio';
        }
    }

    handleDimensionChange(changedDimension) {
        if (this.aspectRatioLocked) {
            if (changedDimension === 'width') {
                const newWidth = parseInt(this.outputWidth.value);
                const newHeight = Math.round(newWidth / this.currentAspectRatio);
                this.outputHeight.value = Math.max(64, Math.min(2048, newHeight));
            } else {
                const newHeight = parseInt(this.outputHeight.value);
                const newWidth = Math.round(newHeight * this.currentAspectRatio);
                this.outputWidth.value = Math.max(64, Math.min(2048, newWidth));
            }
        }

        // Update preset to custom if dimensions don't match any preset
        const width = parseInt(this.outputWidth.value);
        const height = parseInt(this.outputHeight.value);

        const matchingPreset = this.findMatchingPreset(width, height);
        this.sizePreset.value = matchingPreset;

        this.updatePreview();
    }

    findMatchingPreset(width, height) {
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
        this.advancedPositioning.style.display =
            (mode === 'custom' || mode === 'rule-of-thirds') ? 'block' : 'none';
        this.updatePreview();
    }

    updateOffsetDisplay(direction) {
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
        this.outputWidth.value = 256;
        this.outputHeight.value = 256;
        this.faceHeightPct.value = 70;
        this.sizePreset.value = 'custom';
        this.positioningMode.value = 'center';
        this.verticalOffset.value = 0;
        this.horizontalOffset.value = 0;
        this.aspectRatioLocked = false;
        this.aspectRatioLock.classList.remove('locked');
        this.aspectRatioLock.textContent = '🔓';

        this.updatePreview();
        this.updatePositioningControls();
        this.updateOffsetDisplays();

        this.updateStatus('Settings reset to defaults', 'success');
    }

    calculateSmartCropPosition(face, cropWidth, cropHeight, imageWidth, imageHeight) {
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

    async loadModel() {
        try {
            this.updateStatus('Loading MediaPipe face detection model...', 'loading');

            // Wait for TensorFlow to be ready
            await tf.ready();
            console.log('TensorFlow.js ready');

            // Check if faceLandmarksDetection is available
            if (typeof faceLandmarksDetection === 'undefined') {
                throw new Error('MediaPipe face landmarks detection library not loaded');
            }

            const model = faceLandmarksDetection.SupportedModels.MediaPipeFaceMesh;
            const detectorConfig = {
                runtime: 'tfjs',
                refineLandmarks: false,
                maxFaces: 20,
                solutionPath: 'https://cdn.jsdelivr.net/npm/@mediapipe/face_mesh'
            };

            console.log('Creating detector with config:', detectorConfig);
            this.detector = await faceLandmarksDetection.createDetector(model, detectorConfig);
            console.log('Detector created successfully');

            this.updateStatus('MediaPipe model loaded successfully. Ready to process images!', 'success');
        } catch (error) {
            console.error('Error loading MediaPipe model:', error);
            this.updateStatus(`Error loading model: ${error.message}. Please refresh the page.`, 'error');

            // Try to provide helpful error information
            if (typeof tf === 'undefined') {
                this.updateStatus('TensorFlow.js not loaded. Please refresh the page.', 'error');
            } else if (typeof faceLandmarksDetection === 'undefined') {
                this.updateStatus('MediaPipe face detection library not loaded. Please refresh the page.', 'error');
            }
        }
    }

    async handleMultipleImageUpload(event) {
        const files = Array.from(event.target.files);
        if (files.length === 0) return;

        this.updateStatus('Loading images...', 'loading');

        for (const file of files) {
            const imageId = this.generateImageId();

            try {
                const image = await this.loadImageFromFile(file);
                this.images.set(imageId, {
                    id: imageId,
                    file: file,
                    image: image,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    status: 'loaded'
                });
            } catch (error) {
                console.error('Error loading image:', file.name, error);
            }
        }

        this.updateGallery();
        this.updateControls();
        this.imageGallery.style.display = 'block';
        this.updateStatus(`Loaded ${this.images.size} images. Select images and click "Process Selected" or "Process All".`, 'success');
    }

    async loadImageFromFile(file) {
        return new Promise((resolve, reject) => {
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

        for (const [imageId, imageData] of this.images) {
            const galleryItem = this.createGalleryItem(imageData);
            this.galleryGrid.appendChild(galleryItem);
        }

        this.updateSelectionCounter();
    }

    createGalleryItem(imageData) {
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
        img.src = imageData.image.src;
        img.alt = imageData.file.name;

        const info = document.createElement('div');
        info.className = 'gallery-item-info';

        const name = document.createElement('div');
        name.className = 'gallery-item-name';
        name.textContent = imageData.file.name;

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

        item.addEventListener('click', () => this.toggleSelection(imageData.id));

        return item;
    }

    toggleSelection(imageId) {
        const imageData = this.images.get(imageId);
        if (imageData) {
            imageData.selected = !imageData.selected;
            this.updateGallery();
            this.updateControls();
        }
    }

    selectAll() {
        for (const imageData of this.images.values()) {
            imageData.selected = true;
        }
        this.updateGallery();
        this.updateControls();
    }

    selectNone() {
        for (const imageData of this.images.values()) {
            imageData.selected = false;
        }
        this.updateGallery();
        this.updateControls();
    }

    updateSelectionCounter() {
        const selected = Array.from(this.images.values()).filter(img => img.selected).length;
        const total = this.images.size;

        this.selectedCount.textContent = selected;
        this.totalCount.textContent = total;
    }

    updateControls() {
        const hasImages = this.images.size > 0;
        const hasSelected = Array.from(this.images.values()).some(img => img.selected);
        const hasResults = Array.from(this.images.values()).some(img => img.results.length > 0);

        this.processAllBtn.disabled = !hasImages || this.isProcessing;
        this.processSelectedBtn.disabled = !hasSelected || this.isProcessing;
        this.clearAllBtn.disabled = !hasImages;
        this.downloadAllBtn.disabled = !hasResults;
    }

    async processAll() {
        const allImages = Array.from(this.images.values());
        await this.processImages(allImages);
    }

    async processSelected() {
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
        await this.processImages(selectedImages);
    }

    async processImages(imagesToProcess) {
        if (this.isProcessing) return;

        if (!this.detector) {
            this.updateStatus('Face detection model not loaded. Please wait and try again.', 'error');
            return;
        }

        this.isProcessing = true;
        this.processingQueue = imagesToProcess;
        this.progressSection.style.display = 'block';

        this.updateControls();

        for (let i = 0; i < imagesToProcess.length; i++) {
            const imageData = imagesToProcess[i];
            const progress = ((i + 1) / imagesToProcess.length) * 100;

            this.currentProcessingId = imageData.id;
            imageData.status = 'processing';
            this.updateGallery();

            this.progressFill.style.width = `${progress}%`;
            this.progressText.textContent = `Processing image ${i + 1} of ${imagesToProcess.length}: ${imageData.file.name}`;

            try {
                await this.processImageData(imageData);
                imageData.processed = true;
                imageData.status = 'completed';
            } catch (error) {
                console.error('Error processing image:', imageData.file.name, error);
                imageData.status = 'error';
            }

            this.updateGallery();

            // Small delay to prevent UI blocking
            await new Promise(resolve => setTimeout(resolve, 100));
        }

        this.isProcessing = false;
        this.currentProcessingId = null;
        this.progressSection.style.display = 'none';

        this.updateControls();

        const successCount = imagesToProcess.filter(img => img.status === 'completed').length;
        const totalFaces = imagesToProcess.reduce((sum, img) => sum + img.results.length, 0);

        this.updateStatus(`Processed ${successCount} images and found ${totalFaces} faces total!`, 'success');
    }

    async processImageData(imageData) {
        // Detect faces with quality analysis
        const faces = await this.detectFacesWithQuality(imageData.image);
        imageData.faces = faces;

        // Crop faces (only selected ones)
        imageData.results = await this.cropFacesFromImageData(imageData);
    }

    async detectFacesWithQuality(image) {
        if (!this.detector) {
            throw new Error('Face detection model not loaded. Please wait for model to load.');
        }

        const faces = await this.detector.estimateFaces(image);
        const detectedFaces = [];

        if (faces.length > 0) {
            for (let i = 0; i < faces.length; i++) {
                const face = faces[i];
                const keypoints = face.keypoints;
                const xs = keypoints.map(point => point.x);
                const ys = keypoints.map(point => point.y);
                const minX = Math.min(...xs);
                const maxX = Math.max(...xs);
                const minY = Math.min(...ys);
                const maxY = Math.max(...ys);

                // Calculate confidence score from keypoint stability
                const confidence = this.calculateConfidenceScore(keypoints);

                // Calculate face quality using blur detection
                const quality = await this.calculateFaceQuality(image, minX, minY, maxX - minX, maxY - minY);

                detectedFaces.push({
                    id: `face_${i}`,
                    x: minX,
                    y: minY,
                    width: maxX - minX,
                    height: maxY - minY,
                    confidence: confidence,
                    quality: quality,
                    selected: true, // Default to selected
                    index: i + 1
                });
            }
        }

        return detectedFaces;
    }

    calculateConfidenceScore(keypoints) {
        // Calculate confidence based on keypoint distribution and stability
        if (keypoints.length === 0) return 0;

        // Simple confidence score based on keypoint spread
        const xs = keypoints.map(p => p.x);
        const ys = keypoints.map(p => p.y);
        const width = Math.max(...xs) - Math.min(...xs);
        const height = Math.max(...ys) - Math.min(...ys);

        // Higher confidence for larger, well-distributed faces
        const score = Math.min(0.95, (width * height) / 50000 + 0.3);
        return Math.max(0.1, score);
    }

    async calculateFaceQuality(image, x, y, width, height) {
        // Create canvas to extract face region
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        canvas.width = width;
        canvas.height = height;

        // Draw face region
        ctx.drawImage(image, x, y, width, height, 0, 0, width, height);

        // Get image data for blur analysis
        const imageData = ctx.getImageData(0, 0, width, height);
        const data = imageData.data;

        // Calculate Laplacian variance for blur detection
        const laplacianVariance = this.calculateLaplacianVariance(data, width, height);

        // Classify quality based on variance
        if (laplacianVariance > 1000) return { score: laplacianVariance, level: 'high' };
        if (laplacianVariance > 300) return { score: laplacianVariance, level: 'medium' };
        return { score: laplacianVariance, level: 'low' };
    }

    calculateLaplacianVariance(data, width, height) {
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
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
        if (selectedImages.length === 0) {
            this.updateStatus('Please select an image first', 'error');
            return;
        }

        const currentImage = selectedImages[0]; // Use first selected image
        this.updateStatus('Detecting faces...', 'loading');

        try {
            currentImage.faces = await this.detectFacesWithQuality(currentImage.image);
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
            this.canvasContainer.style.display = 'block';
            this.updateStatus(`Detected ${currentImage.faces.length} faces with quality analysis`, 'success');
        } catch (error) {
            console.error('Error detecting faces:', error);
            this.updateStatus(`Error detecting faces: ${error.message}`, 'error');
        }
    }

    displayImageWithFaceOverlays(imageData) {
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
        imageData.faces.forEach((face) => {
            this.createFaceOverlay(face, scale);
        });
    }

    createFaceOverlay(face, scale) {
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

    toggleFaceSelection(faceId) {
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
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
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            currentImage.faces.forEach(face => face.selected = true);
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
        }
    }

    selectNoneFaces() {
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
        if (selectedImages.length === 0) return;

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            currentImage.faces.forEach(face => face.selected = false);
            this.displayImageWithFaceOverlays(currentImage);
            this.updateFaceCounter();
        }
    }

    updateFaceCounter() {
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
        if (selectedImages.length === 0) {
            this.faceCount.textContent = '0';
            this.selectedFaceCount.textContent = '0';
            return;
        }

        const currentImage = selectedImages[0];
        if (currentImage.faces) {
            const total = currentImage.faces.length;
            const selected = currentImage.faces.filter(f => f.selected).length;
            this.faceCount.textContent = total;
            this.selectedFaceCount.textContent = selected;
        }
    }

    startResize(e, faceId) {
        e.preventDefault();
        e.stopPropagation();
        // Basic resize functionality - can be expanded
        console.log('Resize started for face:', faceId, 'handle:', e.target.dataset.position);
    }

    async cropFacesFromImageData(imageData) {
        if (!imageData.faces || imageData.faces.length === 0) return [];

        // Only crop selected faces
        const selectedFaces = imageData.faces.filter(face => face.selected);
        if (selectedFaces.length === 0) return [];

        const results = [];
        const outputWidth = parseInt(this.outputWidth.value);
        const outputHeight = parseInt(this.outputHeight.value);
        const faceHeightPct = parseInt(this.faceHeightPct.value) / 100;

        // Create temporary canvas for processing
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        tempCanvas.width = imageData.image.width;
        tempCanvas.height = imageData.image.height;
        tempCtx.drawImage(imageData.image, 0, 0);

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
                face, cropWidthSrc, cropHeightSrc, imageData.image.width, imageData.image.height
            );

            const finalCropWidth = Math.min(cropWidthSrc, imageData.image.width - cropX);
            const finalCropHeight = Math.min(cropHeightSrc, imageData.image.height - cropY);

            // Crop and resize
            cropCtx.drawImage(
                tempCanvas,
                cropX, cropY, finalCropWidth, finalCropHeight,
                0, 0, outputWidth, outputHeight
            );

            // Generate image with selected format and quality
            const format = this.outputFormat.value;
            const quality = format === 'jpeg' ? this.jpegQuality.value / 100 : 1.0;

            let mimeType = 'image/png';
            if (format === 'jpeg') mimeType = 'image/jpeg';
            if (format === 'webp') mimeType = 'image/webp';

            const croppedDataUrl = cropCanvas.toDataURL(mimeType, quality);

            // Generate filename using template
            const filename = this.generateFilename(imageData.file.name, face.index, outputWidth, outputHeight);

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

    generateFilename(originalName, faceIndex, width, height) {
        const template = this.namingTemplate.value;
        const format = this.outputFormat.value;
        const baseName = originalName.replace(/\.[^/.]+$/, ''); // Remove extension
        const timestamp = new Date().toISOString().slice(0, 19).replace(/[:.]/g, '-');

        return template
            .replace(/{original}/g, baseName)
            .replace(/{index}/g, faceIndex)
            .replace(/{timestamp}/g, timestamp)
            .replace(/{width}/g, width)
            .replace(/{height}/g, height) + '.' + format;
    }

    async downloadAllResults() {
        const allResults = [];

        // Collect all results
        for (const imageData of this.images.values()) {
            if (imageData.results.length > 0) {
                allResults.push(...imageData.results);
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

    async downloadAsZip(results) {
        try {
            this.updateStatus('Creating ZIP archive...', 'loading');

            const zip = new JSZip();

            results.forEach((result, index) => {
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
        } catch (error) {
            console.error('Error creating ZIP:', error);
            this.updateStatus(`Error creating ZIP: ${error.message}`, 'error');
        }
    }

    downloadIndividually(results) {
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

    async downloadIndividualFace(faceId) {
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);
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
        } catch (error) {
            console.error('Error downloading individual face:', error);
            this.updateStatus(`Error downloading face: ${error.message}`, 'error');
        }
    }

    async cropSingleFace(imageData, face) {
        const outputWidth = parseInt(this.outputWidth.value);
        const outputHeight = parseInt(this.outputHeight.value);
        const faceHeightPct = parseInt(this.faceHeightPct.value) / 100;

        // Create temporary canvas for processing
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        tempCanvas.width = imageData.image.width;
        tempCanvas.height = imageData.image.height;
        tempCtx.drawImage(imageData.image, 0, 0);

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
        cropCtx.drawImage(
            tempCanvas,
            cropX, cropY, finalCropWidth, finalCropHeight,
            0, 0, outputWidth, outputHeight
        );

        // Generate image with selected format and quality
        const format = this.outputFormat.value;
        const quality = format === 'jpeg' ? this.jpegQuality.value / 100 : 1.0;

        let mimeType = 'image/png';
        if (format === 'jpeg') mimeType = 'image/jpeg';
        if (format === 'webp') mimeType = 'image/webp';

        const croppedDataUrl = cropCanvas.toDataURL(mimeType, quality);
        const filename = this.generateFilename(imageData.file.name, face.index, outputWidth, outputHeight);

        return {
            dataUrl: croppedDataUrl,
            filename: filename,
            faceId: face.id
        };
    }

    clearAll() {
        // Clean up object URLs
        for (const imageData of this.images.values()) {
            if (imageData.image.src.startsWith('blob:')) {
                URL.revokeObjectURL(imageData.image.src);
            }
        }

        this.images.clear();
        this.processingQueue = [];
        this.isProcessing = false;
        this.currentProcessingId = null;

        this.imageGallery.style.display = 'none';
        this.canvasContainer.style.display = 'none';
        this.progressSection.style.display = 'none';
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