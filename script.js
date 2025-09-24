// ===== CLEAN VERSION - NO OLD CODE =====

// Global variables
let cv = null;
let faceDetector = null;
let net = null; // Fallback DNN detector
let faceCascade = null; // Haar cascade detector
let isOpenCvReady = false;
let currentImage = null;
let detectedFaces = [];

// YuNet model constants
const INPUT_W = 320, INPUT_H = 320;

// ===== YUNET POST-PROCESSING HELPERS =====
const sigmoid = x => 1 / (1 + Math.exp(-x));

function nms(dets, iouThresh = 0.3) {
    // dets: [{x,y,w,h,score, landmarks:[...]}] -> indices to keep
    const area = dets.map(d => d.w * d.h);
    const order = dets.map((d, i) => [i, d.score]).sort((a, b) => b[1] - a[1]).map(a => a[0]);
    const keep = [];
    while (order.length) {
        const i = order.shift();
        keep.push(i);
        const ox = dets[i].x, oy = dets[i].y, ow = dets[i].w, oh = dets[i].h;
        for (let k = order.length - 1; k >= 0; --k) {
            const j = order[k];
            const xx1 = Math.max(ox, dets[j].x);
            const yy1 = Math.max(oy, dets[j].y);
            const xx2 = Math.min(ox + ow, dets[j].x + dets[j].w);
            const yy2 = Math.min(oy + oh, dets[j].y + dets[j].h);
            const w = Math.max(0, xx2 - xx1), h = Math.max(0, yy2 - yy1);
            const inter = w * h, uni = area[i] + area[j] - inter;
            const iou = uni ? (inter / uni) : 0;
            if (iou >= iouThresh) order.splice(k, 1);
        }
    }
    return keep;
}

function postprocessYuNet(outputs, inW, inH, outW, outH, scoreThresh = 0.6, iouNms = 0.3) {
    // outputs: array of cv.Mat from net.forward()
    let dets = [];

    if (outputs.length === 1) {
        // Case B: already N×15 (x,y,w,h, 5*2 landmarks, score)
        const out = outputs[0];
        const rows = out.rows;
        for (let r = 0; r < rows; r++) {
            const row = out.data32F.subarray(r * out.cols, (r + 1) * out.cols);
            const score = row[14];
            if (score < scoreThresh) continue;
            dets.push({
                x: row[0], y: row[1], w: row[2], h: row[3],
                landmarks: [row[4], row[5], row[6], row[7], row[8], row[9], row[10], row[11], row[12], row[13]],
                score
            });
        }
    } else {
        // Case A: 3 blobs: loc (N×14), conf (N×2), iou (N×1)
        const loc = outputs[0], conf = outputs[1], iou = outputs[2];
        const N = loc.rows;
        for (let r = 0; r < N; r++) {
            const bb = loc.data32F.subarray(r * loc.cols, (r + 1) * loc.cols); // 14 numbers: x,y,w,h + 10 landmark coords
            const cf = conf.data32F.subarray(r * conf.cols, (r + 1) * conf.cols); // 2-class: [bg, face]
            const iq = iou.data32F[r]; // scalar
            const faceProb = sigmoid(cf[1]); // class prob for "face"
            const iouQual = sigmoid(iq); // quality from IoU head
            const score = Math.sqrt(faceProb * iouQual); // common fusion heuristic

            if (score < scoreThresh) continue;
            dets.push({
                x: bb[0], y: bb[1], w: bb[2], h: bb[3],
                landmarks: [bb[4], bb[5], bb[6], bb[7], bb[8], bb[9], bb[10], bb[11], bb[12], bb[13]],
                score
            });
        }
    }

    // NMS
    const keepIdx = nms(dets, iouNms);
    dets = keepIdx.map(i => dets[i]);

    // Map from model space (inW×inH) back to original (outW×outH)
    const sx = outW / inW, sy = outH / inH;
    for (const d of dets) {
        d.x *= sx; d.y *= sy; d.w *= sx; d.h *= sy;
        for (let k = 0; k < 10; k += 2) { d.landmarks[k] *= sx; d.landmarks[k + 1] *= sy; }
    }
    return dets;
}

// ===== OPENCV LOADING =====
function loadOpenCV() {
    console.log('🚀 [CLEAN] Loading OpenCV.js with DNN support...');
    showStatus('Loading OpenCV.js library with DNN support...', 'info');

    // Set up OpenCV module configuration before loading
    window.Module = {
        onRuntimeInitialized: function() {
            console.log('🎉 [CLEAN] WebAssembly runtime initialized!');
            cv = window.cv;
            onOpenCvReady();
        },
        locateFile: function(path, scriptDirectory) {
            // Help OpenCV find the .wasm file
            if (path.endsWith('.wasm')) {
                console.log('📁 [CLEAN] Locating WASM file:', path);
                return scriptDirectory + path;
            }
            return scriptDirectory + path;
        }
    };

    const script = document.createElement('script');
    script.type = 'text/javascript';

    // Use local DNN-enabled opencv.js
    script.src = './opencv.js';

    script.onload = function() {
        console.log('✅ [CLEAN] OpenCV script loaded');
        console.log('   window.cv exists:', !!window.cv);
        console.log('   window.Module exists:', !!window.Module);

        if (window.cv) {
            console.log('   cv.Mat exists:', !!window.cv.Mat);
            console.log('   cv.dnn exists:', !!window.cv.dnn);

            // Check if already initialized
            if (window.cv.Mat && window.cv.dnn) {
                console.log('🎉 [CLEAN] OpenCV already ready with DNN!');
                cv = window.cv;
                onOpenCvReady();
                return;
            }

            console.log('🔄 [CLEAN] Waiting for WebAssembly initialization...');
            // The Module.onRuntimeInitialized callback should handle this now

        } else {
            console.error('❌ [CLEAN] OpenCV object not found');
            showStatus('OpenCV failed to load. Please refresh.', 'error');
        }
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
    console.log('   cv.dnn:', !!cv.dnn);
    console.log('   cv.dnn.readNetFromONNX:', !!(cv.dnn && cv.dnn.readNetFromONNX));

    if (cv.dnn && cv.dnn.readNetFromONNX) {
        console.log('✅ [CLEAN] DNN module found! Loading YuNet model...');
        await loadDNNModel();
    } else {
        console.error('❌ [CLEAN] DNN module not available in this OpenCV.js build');
        showStatus('DNN module not available. Please use a DNN-enabled OpenCV.js build.', 'error');
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

        // Create new cascade classifier
        faceCascade = new cv.CascadeClassifier();

        // Check if we can load from a URL or use built-in
        try {
            const response = await fetch('https://raw.githubusercontent.com/opencv/opencv/master/data/haarcascades/haarcascade_frontalface_default.xml');
            if (response.ok) {
                const cascadeData = await response.text();
                cv.FS_createDataFile('/', 'haarcascade.xml', cascadeData, true, false, false);
                const loaded = faceCascade.load('haarcascade.xml');

                if (loaded && !faceCascade.empty()) {
                    console.log('🎉 [CLEAN] Haar cascade loaded from URL!');
                    showStatus('Ready! Using Haar cascade detection.', 'success');
                    return;
                } else {
                    throw new Error('Cascade loaded but is empty');
                }
            }
        } catch (urlError) {
            console.log('⚠️ [CLEAN] Could not load cascade from URL:', urlError.message);
        }

        // If URL loading fails, just use simple detection
        faceCascade = null;
        console.log('📝 [CLEAN] Using simple detection method');
        showStatus('Ready! Using simple detection method.', 'success');

    } catch (error) {
        console.error('❌ [CLEAN] Haar cascade loading failed:', error);
        faceCascade = null;
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
    if (!net || net.empty()) {
        showStatus('YuNet model not loaded. Please refresh the page.', 'error');
        return;
    }

    const inputCanvas = document.getElementById('inputCanvas');
    const outputCanvas = document.getElementById('outputCanvas');

    const src = cv.imread(inputCanvas);
    const faces = new cv.RectVector();

    // Use only DNN with YuNet
    detectWithDNN(src, faces);

    if (faces.size() === 0) {
        showStatus('No faces detected. Try adjusting confidence threshold.', 'error');
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
        console.log('🔍 [CLEAN] Using DNN detection with YuNet post-processing');

        // Convert RGBA to RGB for YuNet
        let rgb = new cv.Mat();
        cv.cvtColor(src, rgb, cv.COLOR_RGBA2RGB);

        // Create blob: YuNet expects RGB, 0–255 scale, resized to INPUT_W×INPUT_H
        const inputSize = new cv.Size(INPUT_W, INPUT_H);
        const blob = cv.dnn.blobFromImage(rgb, 1.0, inputSize, new cv.Scalar(), false, false);

        net.setInput(blob);

        // Get outputs - handle both single Mat and multiple outputs
        let outputs = [];
        try {
            const out = net.forward();  // Mat or MatVector in newer builds
            if (out instanceof cv.Mat) {
                outputs = [out];
            } else {
                // MatVector case
                for (let i = 0; i < out.size(); ++i) {
                    outputs.push(out.get(i));
                }
                out.delete();
            }
        } catch {
            // Fallback: try layer names if needed
            const names = net.getUnconnectedOutLayersNames();
            for (const name of names) {
                outputs.push(net.forward(name));
            }
        }

        console.log(`📊 [CLEAN] Got ${outputs.length} output blobs`);
        outputs.forEach((out, i) => {
            console.log(`   Output ${i}: ${out.rows}×${out.cols}`);
        });

        // Use our YuNet post-processing
        const threshold = parseFloat(document.getElementById('confidenceSlider').value);
        const detections = postprocessYuNet(outputs, INPUT_W, INPUT_H, src.cols, src.rows, threshold, 0.3);

        console.log(`👤 [CLEAN] Found ${detections.length} faces after post-processing`);

        // Convert to OpenCV Rect format
        for (const det of detections) {
            const face = new cv.Rect(Math.floor(det.x), Math.floor(det.y), Math.floor(det.w), Math.floor(det.h));
            faces.push_back(face);
            console.log(`   Face: score=${det.score.toFixed(3)}, box=[${det.x.toFixed(1)}, ${det.y.toFixed(1)}, ${det.w.toFixed(1)}, ${det.h.toFixed(1)}]`);
        }

        // Cleanup
        blob.delete();
        rgb.delete();
        outputs.forEach(out => out.delete && out.delete());

    } catch (error) {
        console.error('❌ [CLEAN] DNN error:', error);
    }
}

function detectWithHaarCascade(src, faces) {
    try {
        console.log('🔍 [CLEAN] Using Haar cascade detection');

        // Convert to grayscale for Haar cascade
        let gray = new cv.Mat();
        cv.cvtColor(src, gray, cv.COLOR_RGBA2GRAY);

        // Detect faces
        const scaleFactor = 1.1;
        const minNeighbors = 3;
        const minSize = new cv.Size(30, 30);
        const maxSize = new cv.Size();

        faceCascade.detectMultiScale(gray, faces, scaleFactor, minNeighbors, 0, minSize, maxSize);

        console.log(`👤 [CLEAN] Haar cascade found ${faces.size()} faces`);

        // Log face details
        for (let i = 0; i < faces.size(); i++) {
            const face = faces.get(i);
            console.log(`   Face ${i}: [${face.x}, ${face.y}, ${face.width}, ${face.height}]`);
        }

        gray.delete();
    } catch (error) {
        console.error('❌ [CLEAN] Haar cascade error:', error);
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