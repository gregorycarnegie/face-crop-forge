// ===== CLEAN VERSION - NO OLD CODE =====

// Global variables
let cv = null;
let net = null;
let isOpenCvReady = false;
let currentImage = null;
let detectedFaces = [];

// ===== OPENCV LOADING =====
function loadOpenCV() {
    console.log('🚀 [CLEAN] Loading OpenCV.js...');
    showStatus('Loading OpenCV.js library...', 'info');

    const script = document.createElement('script');
    script.type = 'text/javascript';
    script.src = './opencv.js';

    script.onload = function() {
        console.log('✅ [CLEAN] OpenCV script loaded');

        // Check immediately
        if (window.cv && window.cv.Mat) {
            console.log('🎉 [CLEAN] OpenCV ready immediately!');
            cv = window.cv;
            onOpenCvReady();
            return;
        }

        // Poll for readiness
        let attempts = 0;
        const maxAttempts = 20;

        const checkCV = () => {
            attempts++;
            console.log(`🔄 [CLEAN] Check ${attempts}/${maxAttempts}`);

            if (window.cv && window.cv.Mat) {
                console.log('🎉 [CLEAN] OpenCV ready after polling!');
                cv = window.cv;
                onOpenCvReady();
            } else if (window.cv && window.cv.onRuntimeInitialized) {
                console.log('🔄 [CLEAN] Setting runtime callback...');
                window.cv.onRuntimeInitialized = () => {
                    console.log('🚀 [CLEAN] Runtime callback fired!');
                    cv = window.cv;
                    onOpenCvReady();
                };
            } else if (attempts < maxAttempts) {
                setTimeout(checkCV, 500);
            } else {
                console.error('❌ [CLEAN] OpenCV failed to load');
                showStatus('OpenCV failed to load. Please refresh.', 'error');
            }
        };

        setTimeout(checkCV, 200);
    };

    script.onerror = function() {
        console.error('❌ [CLEAN] Failed to load opencv.js');
        showStatus('Failed to load opencv.js file', 'error');
    };

    document.head.appendChild(script);
}

function onOpenCvReady() {
    console.log('🎉 [CLEAN] OpenCV is ready!');
    isOpenCvReady = true;
    showStatus('OpenCV.js loaded! Loading face detection model...', 'success');
    loadDNNModel();
}

// ===== MODEL LOADING =====
async function loadDNNModel() {
    try {
        console.log('📦 [CLEAN] Loading YuNet model...');

        const response = await fetch('./face_detection_yunet_2023mar.onnx');
        if (!response.ok) {
            throw new Error(`Failed to fetch: ${response.status}`);
        }

        const modelData = await response.arrayBuffer();
        const modelBytes = new Uint8Array(modelData);
        console.log(`📊 [CLEAN] Model size: ${modelBytes.length} bytes`);

        cv.FS_createDataFile('/', 'yunet.onnx', modelBytes, true, false, false);
        net = cv.dnn.readNetFromONNX('yunet.onnx');

        if (net && !net.empty()) {
            console.log('🎉 [CLEAN] YuNet model loaded!');
            showStatus('Ready! Choose an image to detect faces.', 'success');
        } else {
            throw new Error('Model failed to initialize');
        }
    } catch (error) {
        console.error('❌ [CLEAN] Model error:', error);
        net = null;
        showStatus('Model failed. Using fallback detection.', 'error');
    }
}

// ===== IMAGE HANDLING =====
document.getElementById('imageInput').addEventListener('change', function(e) {
    const file = e.target.files[0];
    if (!file) return;

    console.log('📷 [CLEAN] Image selected:', file.name);

    const reader = new FileReader();
    reader.onload = function(event) {
        const img = new Image();
        img.onload = function() {
            console.log('🖼️ [CLEAN] Image loaded:', this.width, 'x', this.height);
            currentImage = img;
            displayImage(img, 'inputCanvas');
            document.getElementById('processBtn').disabled = false;
            showStatus('Image loaded! Click "Detect & Crop Faces".', 'info');
        };
        img.src = event.target.result;
    };
    reader.readAsDataURL(file);
});

function displayImage(img, canvasId) {
    const canvas = document.getElementById(canvasId);
    const ctx = canvas.getContext('2d');

    const maxWidth = 400;
    const maxHeight = 400;
    let { width, height } = img;

    if (width > maxWidth || height > maxHeight) {
        const ratio = Math.min(maxWidth / width, maxHeight / height);
        width *= ratio;
        height *= ratio;
    }

    canvas.width = width;
    canvas.height = height;
    ctx.drawImage(img, 0, 0, width, height);
}

// ===== FACE DETECTION =====
function processImage() {
    console.log('🔄 [CLEAN] Processing image...');
    console.log('   OpenCV ready:', isOpenCvReady);
    console.log('   Image exists:', !!currentImage);

    if (!isOpenCvReady) {
        showStatus('OpenCV not ready. Please wait.', 'error');
        return;
    }

    if (!currentImage) {
        showStatus('No image selected.', 'error');
        return;
    }

    showLoading(true);

    setTimeout(() => {
        try {
            detectFaces();
        } catch (error) {
            console.error('❌ [CLEAN] Detection error:', error);
            showStatus('Detection failed: ' + error.message, 'error');
        } finally {
            showLoading(false);
        }
    }, 100);
}

function detectFaces() {
    const inputCanvas = document.getElementById('inputCanvas');
    const outputCanvas = document.getElementById('outputCanvas');

    const src = cv.imread(inputCanvas);
    const faces = new cv.RectVector();

    // Simple face detection for testing
    if (net && !net.empty()) {
        detectWithYuNet(src, faces);
    } else {
        // Fallback detection
        const faceWidth = Math.min(src.cols * 0.3, 200);
        const faceHeight = faceWidth * 1.2;
        const x = (src.cols - faceWidth) / 2;
        const y = (src.rows - faceHeight) / 2;
        const face = new cv.Rect(x, y, faceWidth, faceHeight);
        faces.push_back(face);
        console.log('🔄 [CLEAN] Using fallback detection');
    }

    if (faces.size() === 0) {
        showStatus('No faces detected.', 'error');
        src.delete();
        faces.delete();
        return;
    }

    // Draw rectangles
    for (let i = 0; i < faces.size(); i++) {
        const face = faces.get(i);
        const point1 = new cv.Point(face.x, face.y);
        const point2 = new cv.Point(face.x + face.width, face.y + face.height);
        cv.rectangle(src, point1, point2, [0, 255, 0, 255], 3);
    }

    cv.imshow('outputCanvas', src);
    cropFaces(inputCanvas, faces);

    src.delete();
    faces.delete();

    showStatus(`Found ${detectedFaces.length} face(s)!`, 'success');
}

function detectWithYuNet(src, faces) {
    try {
        const blob = cv.dnn.blobFromImage(src, 1.0, new cv.Size(320, 320), new cv.Scalar(0, 0, 0), true, false);
        net.setInput(blob);
        const detections = net.forward();

        const threshold = parseFloat(document.getElementById('confidenceSlider').value);

        for (let i = 0; i < detections.rows; i++) {
            const data = detections.data32F.subarray(i * detections.cols, (i + 1) * detections.cols);
            const confidence = data[14];

            if (confidence > threshold) {
                const x = data[0] * src.cols;
                const y = data[1] * src.rows;
                const w = data[2] * src.cols;
                const h = data[3] * src.rows;

                const face = new cv.Rect(Math.floor(x), Math.floor(y), Math.floor(w), Math.floor(h));
                faces.push_back(face);
                console.log('👤 [CLEAN] YuNet face:', confidence.toFixed(3));
            }
        }

        detections.delete();
        blob.delete();
    } catch (error) {
        console.error('❌ [CLEAN] YuNet error:', error);
    }
}

function cropFaces(sourceCanvas, faces) {
    const container = document.getElementById('croppedFaces');
    container.innerHTML = '';
    detectedFaces = [];

    const padding = parseInt(document.getElementById('paddingSlider').value);
    const outputWidth = parseInt(document.getElementById('outputWidth').value);
    const outputHeight = parseInt(document.getElementById('outputHeight').value);
    const maintainAspectRatio = document.getElementById('maintainAspectRatio').checked;

    for (let i = 0; i < faces.size(); i++) {
        const face = faces.get(i);

        // Create temporary canvas for initial crop with padding
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');

        const cropX = Math.max(0, face.x - padding);
        const cropY = Math.max(0, face.y - padding);
        const cropW = Math.min(sourceCanvas.width - cropX, face.width + 2 * padding);
        const cropH = Math.min(sourceCanvas.height - cropY, face.height + 2 * padding);

        tempCanvas.width = cropW;
        tempCanvas.height = cropH;
        tempCtx.drawImage(sourceCanvas, cropX, cropY, cropW, cropH, 0, 0, cropW, cropH);

        // Create final canvas with custom dimensions
        const cropCanvas = document.createElement('canvas');
        const ctx = cropCanvas.getContext('2d');

        let finalWidth = outputWidth;
        let finalHeight = outputHeight;

        if (maintainAspectRatio) {
            const aspectRatio = cropW / cropH;
            if (aspectRatio > 1) {
                // Width is larger, adjust height
                finalHeight = Math.round(finalWidth / aspectRatio);
            } else {
                // Height is larger, adjust width
                finalWidth = Math.round(finalHeight * aspectRatio);
            }
        }

        cropCanvas.width = finalWidth;
        cropCanvas.height = finalHeight;

        // Draw resized image
        ctx.drawImage(tempCanvas, 0, 0, finalWidth, finalHeight);

        const faceDiv = document.createElement('div');
        faceDiv.className = 'face-crop';
        faceDiv.innerHTML = `
            <canvas width="${finalWidth}" height="${finalHeight}"></canvas>
            <div>Face ${i + 1} (${finalWidth}x${finalHeight})</div>
            <button class="download-btn" onclick="downloadFace(${i})">💾 Download</button>
        `;

        const displayCanvas = faceDiv.querySelector('canvas');
        const displayCtx = displayCanvas.getContext('2d');
        displayCtx.drawImage(cropCanvas, 0, 0);

        container.appendChild(faceDiv);
        detectedFaces.push(cropCanvas);
    }
}

function downloadFace(index) {
    if (index >= 0 && index < detectedFaces.length) {
        const canvas = detectedFaces[index];
        const link = document.createElement('a');
        link.download = `face_${index + 1}.png`;
        link.href = canvas.toDataURL();
        link.click();
    }
}

// ===== UTILITY FUNCTIONS =====
function showLoading(show) {
    document.getElementById('loading').style.display = show ? 'block' : 'none';
}

function showStatus(message, type = 'info') {
    const statusDiv = document.getElementById('status');
    statusDiv.textContent = message;
    statusDiv.className = `status ${type}`;
    statusDiv.style.display = 'block';

    if (type !== 'error') {
        setTimeout(() => {
            statusDiv.style.display = 'none';
        }, 5000);
    }
}

// ===== INITIALIZATION =====
window.addEventListener('load', () => {
    console.log('🌐 [CLEAN] Page loaded');
    loadOpenCV();
});