class FaceCropper {
    constructor() {
        this.detector = null;
        this.currentImage = null;
        this.detectedFaces = [];
        this.croppedFaces = [];

        this.initializeElements();
        this.setupEventListeners();
        this.loadModel();
    }

    initializeElements() {
        this.imageInput = document.getElementById('imageInput');
        this.detectBtn = document.getElementById('detectBtn');
        this.cropBtn = document.getElementById('cropBtn');
        this.downloadBtn = document.getElementById('downloadBtn');
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
        this.imageInput.addEventListener('change', (e) => this.handleImageUpload(e));
        this.detectBtn.addEventListener('click', () => this.detectFaces());
        this.cropBtn.addEventListener('click', () => this.cropFaces());
        this.downloadBtn.addEventListener('click', () => this.downloadCroppedFaces());

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
            this.updateStatus('MediaPipe model loaded successfully. Ready to detect faces!', 'success');
        } catch (error) {
            console.error('Error loading MediaPipe model:', error);
            this.updateStatus(`Error loading model: ${error.message}`, 'error');
        }
    }

    handleImageUpload(event) {
        const file = event.target.files[0];
        if (!file) return;

        this.updateStatus('Loading image...', 'loading');

        const reader = new FileReader();
        reader.onload = (e) => {
            const img = new Image();
            img.onload = () => {
                this.currentImage = img;
                this.displayImage(img);
                this.resetDetection();
                this.detectBtn.disabled = false;
                this.updateStatus('Image loaded. Click "Detect Faces" to find faces.', 'success');
            };
            img.src = e.target.result;
        };
        reader.readAsDataURL(file);
    }

    displayImage(img) {
        const maxWidth = 800;
        const maxHeight = 600;

        let { width, height } = img;

        if (width > maxWidth || height > maxHeight) {
            const ratio = Math.min(maxWidth / width, maxHeight / height);
            width *= ratio;
            height *= ratio;
        }

        this.inputCanvas.width = width;
        this.inputCanvas.height = height;

        this.ctx.clearRect(0, 0, width, height);
        this.ctx.drawImage(img, 0, 0, width, height);
    }

    resetDetection() {
        this.detectedFaces = [];
        this.croppedFaces = [];
        this.faceCount.textContent = '0';
        this.croppedContainer.innerHTML = '';
        this.cropBtn.disabled = true;
        this.downloadBtn.disabled = true;
    }

    async detectFaces() {
        if (!this.currentImage || !this.detector) {
            this.updateStatus('Please load an image and wait for the model to load', 'error');
            return;
        }

        try {
            this.updateStatus('Detecting faces...', 'loading');
            this.detectBtn.disabled = true;

            const faces = await this.detector.estimateFaces(this.currentImage);

            this.detectedFaces = [];

            if (faces.length > 0) {
                for (const face of faces) {
                    const keypoints = face.keypoints;

                    const xs = keypoints.map(point => point.x);
                    const ys = keypoints.map(point => point.y);

                    const minX = Math.min(...xs);
                    const maxX = Math.max(...xs);
                    const minY = Math.min(...ys);
                    const maxY = Math.max(...ys);

                    // Scale to canvas coordinates for display
                    const scaleX = this.inputCanvas.width / this.currentImage.width;
                    const scaleY = this.inputCanvas.height / this.currentImage.height;

                    this.detectedFaces.push({
                        x: minX * scaleX,
                        y: minY * scaleY,
                        width: (maxX - minX) * scaleX,
                        height: (maxY - minY) * scaleY
                    });
                }
            }

            this.drawFaceBoxes();
            this.faceCount.textContent = this.detectedFaces.length;

            if (this.detectedFaces.length > 0) {
                this.cropBtn.disabled = false;
                this.updateStatus(`Detected ${this.detectedFaces.length} face(s). Click "Crop Selected Faces" to extract them.`, 'success');
            } else {
                this.updateStatus('No faces detected in the image.', 'error');
            }

            this.detectBtn.disabled = false;

        } catch (error) {
            console.error('Error detecting faces:', error);
            this.updateStatus(`Error detecting faces: ${error.message}`, 'error');
            this.detectBtn.disabled = false;
        }
    }

    drawFaceBoxes() {
        this.displayImage(this.currentImage);

        this.ctx.strokeStyle = '#00FF00';
        this.ctx.lineWidth = 3;
        this.ctx.font = '16px Arial';
        this.ctx.fillStyle = '#00FF00';

        this.detectedFaces.forEach((face, index) => {
            this.ctx.strokeRect(face.x, face.y, face.width, face.height);

            const label = `Face ${index + 1}`;
            const labelY = face.y > 20 ? face.y - 5 : face.y + face.height + 20;
            this.ctx.fillText(label, face.x, labelY);
        });
    }

    async cropFaces() {
        if (!this.currentImage || this.detectedFaces.length === 0) {
            this.updateStatus('No faces to crop', 'error');
            return;
        }

        try {
            this.updateStatus('Cropping faces...', 'loading');
            this.cropBtn.disabled = true;

            this.croppedFaces = [];
            this.croppedContainer.innerHTML = '';

            // Get crop settings
            const outputWidth = parseInt(this.outputWidth.value);
            const outputHeight = parseInt(this.outputHeight.value);
            const faceHeightPct = parseInt(this.faceHeightPct.value) / 100;

            // Create a temporary canvas to draw the displayed image
            const tempDisplayCanvas = document.createElement('canvas');
            const tempDisplayCtx = tempDisplayCanvas.getContext('2d');
            tempDisplayCanvas.width = this.inputCanvas.width;
            tempDisplayCanvas.height = this.inputCanvas.height;

            // Draw the image exactly as displayed on the canvas
            tempDisplayCtx.drawImage(this.currentImage, 0, 0, this.inputCanvas.width, this.inputCanvas.height);

            // Create crop canvas with fixed output dimensions
            const cropCanvas = document.createElement('canvas');
            const cropCtx = cropCanvas.getContext('2d');
            cropCanvas.width = outputWidth;
            cropCanvas.height = outputHeight;

            this.detectedFaces.forEach((face, index) => {
                // 1. Calculate target face height in output
                const targetFaceHeight = outputHeight * faceHeightPct;

                // 2. Find scale needed so detected face height becomes target height
                const scale = targetFaceHeight / face.height;

                // 3. Define crop box dimensions in source with output aspect ratio
                const cropWidthSrc = outputWidth / scale;
                const cropHeightSrc = outputHeight / scale;

                // 4. Center crop box on the face, clamped to canvas bounds
                const faceCenterX = face.x + face.width / 2;
                const faceCenterY = face.y + face.height / 2;

                const cropX = Math.max(0, Math.min(
                    this.inputCanvas.width - cropWidthSrc,
                    faceCenterX - cropWidthSrc / 2
                ));
                const cropY = Math.max(0, Math.min(
                    this.inputCanvas.height - cropHeightSrc,
                    faceCenterY - cropHeightSrc / 2
                ));

                // Ensure crop dimensions don't exceed canvas bounds
                const finalCropWidth = Math.min(cropWidthSrc, this.inputCanvas.width - cropX);
                const finalCropHeight = Math.min(cropHeightSrc, this.inputCanvas.height - cropY);

                // 5. Crop from source, then resize to output dimensions
                cropCtx.drawImage(
                    tempDisplayCanvas,
                    cropX, cropY, finalCropWidth, finalCropHeight,
                    0, 0, outputWidth, outputHeight
                );

                const croppedDataUrl = cropCanvas.toDataURL('image/png');
                this.croppedFaces.push(croppedDataUrl);

                this.displayCroppedFace(croppedDataUrl, index + 1);
            });

            this.downloadBtn.disabled = false;
            this.updateStatus(`Successfully cropped ${this.croppedFaces.length} faces!`, 'success');
            this.cropBtn.disabled = false;

        } catch (error) {
            console.error('Error cropping faces:', error);
            this.updateStatus(`Error cropping faces: ${error.message}`, 'error');
            this.cropBtn.disabled = false;
        }
    }

    displayCroppedFace(dataUrl, index) {
        const faceDiv = document.createElement('div');
        faceDiv.className = 'cropped-face';

        const img = document.createElement('img');
        img.src = dataUrl;
        img.alt = `Face ${index}`;

        const indexLabel = document.createElement('div');
        indexLabel.className = 'face-index';
        indexLabel.textContent = index;

        faceDiv.appendChild(img);
        faceDiv.appendChild(indexLabel);
        this.croppedContainer.appendChild(faceDiv);
    }

    downloadCroppedFaces() {
        if (this.croppedFaces.length === 0) {
            this.updateStatus('No cropped faces to download', 'error');
            return;
        }

        this.croppedFaces.forEach((dataUrl, index) => {
            const link = document.createElement('a');
            link.download = `cropped_face_${index + 1}.png`;
            link.href = dataUrl;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
        });

        this.updateStatus(`Downloaded ${this.croppedFaces.length} cropped faces!`, 'success');
    }
}

document.addEventListener('DOMContentLoaded', () => {
    new FaceCropper();
});