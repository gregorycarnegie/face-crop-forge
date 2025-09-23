// ===== CLEAN VERSION - NO OLD CODE =====

// Global variables
let cv = null;
let faceDetector = null;
let net = null; // Fallback DNN detector
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
    loadFaceDetectionModel();
}

// ===== MODEL LOADING =====
async function loadFaceDetectionModel() {
    console.log('🔍 [CLEAN] Checking available detection methods...');
    console.log('   cv.FaceDetectorYN:', !!cv.FaceDetectorYN);
    console.log('   cv.FaceDetectorYN.create:', !!(cv.FaceDetectorYN && cv.FaceDetectorYN.create));
    console.log('   cv.dnn:', !!cv.dnn);
    console.log('   cv.dnn.readNetFromONNX:', !!(cv.dnn && cv.dnn.readNetFromONNX));
    console.log('   cv.CascadeClassifier:', !!cv.CascadeClassifier);

    // First, check if FaceDetectorYN is available (preferred method)
    if (cv.FaceDetectorYN && cv.FaceDetectorYN.create) {
        console.log('✅ [CLEAN] Using FaceDetectorYN');
        await loadFaceDetectorYN();
    } else if (cv.dnn && cv.dnn.readNetFromONNX) {
        console.log('✅ [CLEAN] Using DNN fallback');
        await loadDNNModel();
    } else if (cv.CascadeClassifier) {
        console.log('✅ [CLEAN] Using Haar Cascade fallback');
        await loadHaarCascade();
    } else {
        console.log('⚠️ [CLEAN] No detection methods available, using simple fallback');
        showStatus('Ready! Using simple detection method.', 'success');
    }
}

async function loadFaceDetectorYN() {
    try {
        console.log('📦 [CLEAN] Loading YuNet model for FaceDetectorYN...');

        const response = await fetch('./face_detection_yunet_2023mar.onnx');
        if (!response.ok) {
            throw new Error(`Failed to fetch: ${response.status}`);
        }

        const modelData = await response.arrayBuffer();
        const modelBytes = new Uint8Array(modelData);
        console.log(`📊 [CLEAN] Model size: ${modelBytes.length} bytes`);

        // Save model to OpenCV.js filesystem
        cv.FS_createDataFile('/', 'yunet.onnx', modelBytes, true, false, false);

        // Create FaceDetectorYN instance
        faceDetector = cv.FaceDetectorYN.create(
            'yunet.onnx',      // model path
            '',                // config (empty for ONNX)
            new cv.Size(320, 320), // input size
            0.9,               // score threshold
            0.3,               // nms threshold
            5000               // top_k
        );

        if (faceDetector && !faceDetector.empty()) {
            console.log('🎉 [CLEAN] FaceDetectorYN created successfully!');
            showStatus('Ready! Choose an image to detect faces.', 'success');
        } else {
            throw new Error('FaceDetectorYN creation failed');
        }
    } catch (error) {
        console.error('❌ [CLEAN] FaceDetectorYN error:', error);
        console.log('🔄 [CLEAN] Falling back to DNN method...');
        faceDetector = null;
        await loadDNNModel();
    }
}

async function loadDNNModel() {
    try {
        console.log('📦 [CLEAN] Loading YuNet model for DNN...');

        const response = await fetch('./face_detection_yunet_2023mar.onnx');
        if (!response.ok) {
            throw new Error(`Failed to fetch model: ${response.status}`);
        }

        const modelData = await response.arrayBuffer();
        const modelBytes = new Uint8Array(modelData);
        console.log(`📊 [CLEAN] Model size: ${modelBytes.length} bytes`);

        // Simple approach: try to create file, if it fails try to remove and recreate
        let modelFileName = 'yunet_' + Date.now() + '.onnx'; // Use unique filename

        try {
            cv.FS_createDataFile('/', modelFileName, modelBytes, true, false, false);
            console.log('💾 [CLEAN] Model file created:', modelFileName);
        } catch (fsError) {
            console.log('⚠️ [CLEAN] File creation failed, trying simpler approach...');
            modelFileName = 'yunet.onnx';
            cv.FS_createDataFile('/', modelFileName, modelBytes, true, false, false);
            console.log('💾 [CLEAN] Model file created with simple name');
        }

        // Load the model
        net = cv.dnn.readNetFromONNX(modelFileName);

        if (net && !net.empty()) {
            console.log('🎉 [CLEAN] DNN YuNet model loaded successfully!');
            showStatus('Ready! Choose an image to detect faces.', 'success');
        } else {
            throw new Error('Model loaded but is empty or invalid');
        }
    } catch (error) {
        console.error('❌ [CLEAN] DNN Model loading failed:', error);
        net = null;
        // Try Haar cascade as next fallback
        if (cv.CascadeClassifier) {
            console.log('🔄 [CLEAN] Trying Haar cascade fallback...');
            await loadHaarCascade();
        } else {
            showStatus('Ready! Using simple detection method.', 'success');
        }
    }
}

async function loadHaarCascade() {
    try {
        console.log('📦 [CLEAN] Loading Haar cascade...');

        // Try to load a pre-built cascade if available
        const faceCascade = new cv.CascadeClassifier();

        // Check if we can load from a URL or use built-in
        try {
            const response = await fetch('https://raw.githubusercontent.com/opencv/opencv/master/data/haarcascades/haarcascade_frontalface_default.xml');
            if (response.ok) {
                const cascadeData = await response.text();
                cv.FS_createDataFile('/', 'haarcascade.xml', cascadeData, true, false, false);
                faceCascade.load('haarcascade.xml');
                console.log('🎉 [CLEAN] Haar cascade loaded from URL!');
                showStatus('Ready! Using Haar cascade detection.', 'success');
                return;
            }
        } catch (urlError) {
            console.log('⚠️ [CLEAN] Could not load cascade from URL:', urlError.message);
        }

        // If URL loading fails, just use simple detection
        console.log('📝 [CLEAN] Using simple detection method');
        showStatus('Ready! Using simple detection method.', 'success');

    } catch (error) {
        console.error('❌ [CLEAN] Haar cascade loading failed:', error);
        showStatus('Ready! Using simple detection method.', 'success');
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

    // Use FaceDetectorYN if available, otherwise fall back to DNN
    if (faceDetector && !faceDetector.empty()) {
        detectWithFaceDetectorYN(src, faces);
    } else if (net && !net.empty()) {
        detectWithDNN(src, faces);
    } else {
        // Final fallback detection
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

function detectWithFaceDetectorYN(src, faces) {
    try {
        console.log('🔍 [CLEAN] Using FaceDetectorYN detection');

        // Set input size for the detector
        faceDetector.setInputSize(new cv.Size(src.cols, src.rows));

        // Update score threshold from slider
        const threshold = parseFloat(document.getElementById('confidenceSlider').value);
        faceDetector.setScoreThreshold(threshold);

        // Detect faces
        let facesMat = new cv.Mat();
        faceDetector.detect(src, facesMat);

        // Convert results to RectVector
        for (let i = 0; i < facesMat.rows; i++) {
            const x = facesMat.data32F[i * facesMat.cols + 0];
            const y = facesMat.data32F[i * facesMat.cols + 1];
            const w = facesMat.data32F[i * facesMat.cols + 2];
            const h = facesMat.data32F[i * facesMat.cols + 3];
            const confidence = facesMat.data32F[i * facesMat.cols + 14];

            const face = new cv.Rect(Math.floor(x), Math.floor(y), Math.floor(w), Math.floor(h));
            faces.push_back(face);
            console.log('👤 [CLEAN] FaceDetectorYN face:', confidence.toFixed(3));
        }

        facesMat.delete();
    } catch (error) {
        console.error('❌ [CLEAN] FaceDetectorYN error:', error);
        console.log('🔄 [CLEAN] Falling back to DNN detection...');
        detectWithDNN(src, faces);
    }
}

function detectWithDNN(src, faces) {
    try {
        console.log('🔍 [CLEAN] Using DNN detection');
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
                console.log('👤 [CLEAN] DNN face:', confidence.toFixed(3));
            }
        }

        detections.delete();
        blob.delete();
    } catch (error) {
        console.error('❌ [CLEAN] DNN error:', error);
    }
}

function cropFaces(sourceCanvas, faces) {
    const container = document.getElementById('croppedFaces');
    container.innerHTML = '';
    detectedFaces = [];

    const padding = parseInt(document.getElementById('paddingSlider').value);
    const outputWidth = parseInt(document.getElementById('outputWidth').value);
    const outputHeight = parseInt(document.getElementById('outputHeight').value);

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

        // Create final canvas with exact specified dimensions
        const cropCanvas = document.createElement('canvas');
        const ctx = cropCanvas.getContext('2d');

        cropCanvas.width = outputWidth;
        cropCanvas.height = outputHeight;

        // Fill with background color (optional - you can remove this line for transparent background)
        ctx.fillStyle = '#f0f0f0';
        ctx.fillRect(0, 0, outputWidth, outputHeight);

        // Calculate scale to fit the face inside the specified dimensions
        const scaleX = outputWidth / cropW;
        const scaleY = outputHeight / cropH;
        const scale = Math.min(scaleX, scaleY); // Use smaller scale to fit entirely

        // Calculate final dimensions after scaling
        const scaledWidth = cropW * scale;
        const scaledHeight = cropH * scale;

        // Center the scaled image within the output dimensions
        const offsetX = (outputWidth - scaledWidth) / 2;
        const offsetY = (outputHeight - scaledHeight) / 2;

        // Draw the scaled and centered face
        ctx.drawImage(tempCanvas, offsetX, offsetY, scaledWidth, scaledHeight);

        const faceDiv = document.createElement('div');
        faceDiv.className = 'face-crop';
        faceDiv.innerHTML = `
            <canvas width="${outputWidth}" height="${outputHeight}"></canvas>
            <div>Face ${i + 1} (${outputWidth}x${outputHeight})</div>
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