class FaceCropper {
    constructor() {
        this.detector = null;
        this.images = new Map(); // imageId -> { file, image, faces, results, selected, processed }
        this.processingQueue = [];
        this.isProcessing = false;
        this.currentProcessingId = null;

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
        this.fitMode = document.getElementById('fitMode');
        this.previewText = document.getElementById('previewText');

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
        this.fitMode.addEventListener('change', () => this.updatePreview());

        // Initialize preview
        this.updatePreview();
    }

    updateStatus(message, type = '') {
        this.status.textContent = message;
        this.status.className = `status ${type}`;
    }

    updatePreview() {
        const width = parseInt(this.outputWidth.value);
        const height = parseInt(this.outputHeight.value);
        const faceHeight = parseInt(this.faceHeightPct.value);

        this.previewText.textContent = `${width}×${height}px, face at ${faceHeight}% height`;
    }

    async loadModel() {
        try {
            this.updateStatus('Loading MediaPipe face detection model...', 'loading');

            await tf.ready();

            const model = faceLandmarksDetection.SupportedModels.MediaPipeFaceMesh;
            const detectorConfig = {
                runtime: 'tfjs',
                refineLandmarks: false,
                maxFaces: 20
            };

            this.detector = await faceLandmarksDetection.createDetector(model, detectorConfig);
            this.updateStatus('MediaPipe model loaded successfully. Ready to process images!', 'success');
        } catch (error) {
            console.error('Error loading MediaPipe model:', error);
            this.updateStatus(`Error loading model: ${error.message}`, 'error');
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

            // Center crop on face
            const faceCenterX = face.x + face.width / 2;
            const faceCenterY = face.y + face.height / 2;

            const cropX = Math.max(0, Math.min(
                imageData.image.width - cropWidthSrc,
                faceCenterX - cropWidthSrc / 2
            ));
            const cropY = Math.max(0, Math.min(
                imageData.image.height - cropHeightSrc,
                faceCenterY - cropHeightSrc / 2
            ));

            const finalCropWidth = Math.min(cropWidthSrc, imageData.image.width - cropX);
            const finalCropHeight = Math.min(cropHeightSrc, imageData.image.height - cropY);

            // Crop and resize
            cropCtx.drawImage(
                tempCanvas,
                cropX, cropY, finalCropWidth, finalCropHeight,
                0, 0, outputWidth, outputHeight
            );

            const croppedDataUrl = cropCanvas.toDataURL('image/png');
            results.push({
                dataUrl: croppedDataUrl,
                faceIndex: i + 1,
                sourceImage: imageData.file.name
            });
        }

        return results;
    }

    downloadAllResults() {
        let totalDownloads = 0;

        for (const imageData of this.images.values()) {
            if (imageData.results.length > 0) {
                imageData.results.forEach((result, index) => {
                    const link = document.createElement('a');
                    const filename = `${imageData.file.name.split('.')[0]}_face_${result.faceIndex}.png`;
                    link.download = filename;
                    link.href = result.dataUrl;
                    document.body.appendChild(link);
                    link.click();
                    document.body.removeChild(link);
                    totalDownloads++;
                });
            }
        }

        this.updateStatus(`Downloaded ${totalDownloads} cropped faces!`, 'success');
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