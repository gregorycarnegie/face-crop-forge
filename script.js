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
        this.faceCount = document.getElementById('faceCount');
        this.croppedContainer = document.getElementById('croppedContainer');
        this.status = document.getElementById('status');

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
        // Detect faces
        const faces = await this.detector.estimateFaces(imageData.image);
        imageData.faces = [];

        if (faces.length > 0) {
            for (const face of faces) {
                const keypoints = face.keypoints;
                const xs = keypoints.map(point => point.x);
                const ys = keypoints.map(point => point.y);
                const minX = Math.min(...xs);
                const maxX = Math.max(...xs);
                const minY = Math.min(...ys);
                const maxY = Math.max(...ys);

                imageData.faces.push({
                    x: minX,
                    y: minY,
                    width: maxX - minX,
                    height: maxY - minY
                });
            }
        }

        // Crop faces
        imageData.results = await this.cropFacesFromImageData(imageData);
    }

    async cropFacesFromImageData(imageData) {
        if (imageData.faces.length === 0) return [];

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

        for (let i = 0; i < imageData.faces.length; i++) {
            const face = imageData.faces[i];

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