// ===== HAAR CASCADE ONLY VERSION =====

// Global variables
let cv = null;
let faceCascade = null;
let profileCascade = null;
let isOpenCvReady = false;
let currentImage = null;
let detectedFaces = [];

// ===== OPENCV LOADING =====
function loadOpenCV() {
    console.log('🚀 Loading OpenCV.js (CDN version for better compatibility)...');
    showStatus('Loading OpenCV.js library...', 'info');

    // Set up OpenCV module configuration before loading
    window.Module = {
        onRuntimeInitialized: function() {
            console.log('🎉 WebAssembly runtime initialized!');
            console.log('   Checking cv object...');

            // Wait a bit for cv to be fully available
            setTimeout(() => {
                if (window.cv && window.cv.Mat) {
                    cv = window.cv;
                    console.log('   cv.Mat:', !!cv.Mat);
                    console.log('   cv.CascadeClassifier:', !!cv.CascadeClassifier);
                    onOpenCvReady();
                } else {
                    console.error('   cv object still not ready, trying again...');
                    setTimeout(() => {
                        if (window.cv && window.cv.Mat) {
                            cv = window.cv;
                            onOpenCvReady();
                        } else {
                            showStatus('OpenCV initialization failed', 'error');
                        }
                    }, 1000);
                }
            }, 100);
        },
        locateFile: function(path, scriptDirectory) {
            // For CDN version, return as-is
            console.log('📁 Locating file:', path);
            return path;
        }
    };

    const script = document.createElement('script');
    script.type = 'text/javascript';
    // Use CDN version which is more reliable
    script.src = 'https://docs.opencv.org/4.8.0/opencv.js';

    script.onload = function() {
        console.log('✅ OpenCV script loaded from CDN');
        console.log('   window.cv exists:', !!window.cv);

        // Sometimes cv is immediately available
        if (window.cv && window.cv.Mat && window.cv.CascadeClassifier) {
            console.log('🎉 OpenCV already ready!');
            cv = window.cv;
            onOpenCvReady();
            return;
        }

        console.log('🔄 Waiting for WebAssembly initialization...');
    };

    script.onerror = function() {
        console.error('❌ Failed to load opencv.js from CDN, trying local version...');

        // Fallback to local version
        const localScript = document.createElement('script');
        localScript.type = 'text/javascript';
        localScript.src = './opencv.js';

        localScript.onload = function() {
            console.log('✅ Local OpenCV script loaded');
        };

        localScript.onerror = function() {
            showStatus('Failed to load OpenCV.js. Please check your internet connection.', 'error');
        };

        document.head.appendChild(localScript);
    };

    document.head.appendChild(script);
}

function onOpenCvReady() {
    console.log('🎉 OpenCV is ready!');
    isOpenCvReady = true;
    showStatus('OpenCV.js loaded! Loading face detection models...', 'success');
    loadHaarCascades();
}

// ===== HAAR CASCADE LOADING =====
async function loadHaarCascades() {
    try {
        console.log('📦 Loading Haar cascade models...');
        showStatus('Loading face detection models...', 'info');

        // Load frontal face cascade
        await loadCascadeFromURL(
            'https://raw.githubusercontent.com/opencv/opencv/master/data/haarcascades/haarcascade_frontalface_default.xml',
            'frontal_face.xml',
            'frontal face'
        ).then(() => {
            faceCascade = new cv.CascadeClassifier();
            const loaded = faceCascade.load('frontal_face.xml');
            if (!loaded || faceCascade.empty()) {
                throw new Error('Failed to load frontal face cascade');
            }
            console.log('✅ Frontal face cascade loaded');
        });

        // Load profile face cascade (optional)
        try {
            await loadCascadeFromURL(
                'https://raw.githubusercontent.com/opencv/opencv/master/data/haarcascades/haarcascade_profileface.xml',
                'profile_face.xml',
                'profile face'
            );
            profileCascade = new cv.CascadeClassifier();
            const profileLoaded = profileCascade.load('profile_face.xml');
            if (profileLoaded && !profileCascade.empty()) {
                console.log('✅ Profile face cascade loaded');
            } else {
                profileCascade = null;
            }
        } catch (profileError) {
            console.log('⚠️ Profile cascade not available:', profileError.message);
            profileCascade = null;
        }


        showStatus('Ready! Choose an image to detect faces.', 'success');

    } catch (error) {
        console.error('❌ Haar cascade loading failed:', error);
        showStatus('Face detection models failed to load. Limited functionality available.', 'error');
    }
}

async function loadCascadeFromURL(url, filename, description) {
    try {
        console.log(`📥 Loading ${description} cascade from URL...`);
        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const cascadeData = await response.text();
        cv.FS_createDataFile('/', filename, cascadeData, true, false, false);
        console.log(`💾 ${description} cascade file created: ${filename}`);

    } catch (error) {
        console.error(`❌ Failed to load ${description} cascade:`, error);
        throw error;
    }
}

// ===== IMAGE HANDLING =====
document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('imageInput').addEventListener('change', function(e) {
        const file = e.target.files[0];
        if (!file) return;

        console.log('📷 Image selected:', file.name);

        const reader = new FileReader();
        reader.onload = function(event) {
            const img = new Image();
            img.onload = function() {
                console.log('🖼️ Image loaded:', this.width, 'x', this.height);
                currentImage = img;
                displayImage(img, 'inputCanvas');
                document.getElementById('processBtn').disabled = false;
                showStatus('Image loaded! Click "Detect & Crop Faces".', 'info');
            };
            img.src = event.target.result;
        };
        reader.readAsDataURL(file);
    });
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
    console.log('🔄 Processing image...');

    if (!isOpenCvReady) {
        showStatus('OpenCV not ready. Please wait.', 'error');
        return;
    }

    if (!currentImage) {
        showStatus('No image selected.', 'error');
        return;
    }

    if (!faceCascade || faceCascade.empty()) {
        showStatus('Face detection model not loaded. Please refresh the page.', 'error');
        return;
    }

    showLoading(true);

    setTimeout(() => {
        try {
            detectFaces();
        } catch (error) {
            console.error('❌ Detection error:', error);
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

    // Use Haar cascade detection
    detectWithHaarCascade(src, faces);

    if (faces.size() === 0) {
        showStatus('No faces detected. Try adjusting the detection settings.', 'warning');
        src.delete();
        faces.delete();
        return;
    }

    // Draw rectangles around detected faces
    for (let i = 0; i < faces.size(); i++) {
        const face = faces.get(i);
        const point1 = new cv.Point(face.x, face.y);
        const point2 = new cv.Point(face.x + face.width, face.y + face.height);
        cv.rectangle(src, point1, point2, [0, 255, 0, 255], 3);

        // Add face number label
        const labelPoint = new cv.Point(face.x, face.y - 10);
        cv.putText(src, `Face ${i + 1}`, labelPoint, cv.FONT_HERSHEY_SIMPLEX, 0.6, [0, 255, 0, 255], 2);
    }

    cv.imshow('outputCanvas', src);
    cropFaces(inputCanvas, faces);

    src.delete();
    faces.delete();

    showStatus(`Found ${detectedFaces.length} face(s)!`, 'success');
}

function detectWithHaarCascade(src, faces) {
    try {
        console.log('🔍 Using Haar cascade detection');

        // Convert to grayscale for Haar cascade
        let gray = new cv.Mat();
        cv.cvtColor(src, gray, cv.COLOR_RGBA2GRAY);

        // Apply histogram equalization for better detection
        let equalizedGray = new cv.Mat();
        cv.equalizeHist(gray, equalizedGray);

        // Detection parameters - optimized for better results
        // Adjust based on confidence slider value
        const confidence = parseFloat(document.getElementById('confidenceSlider').value);

        // Scale factor: smaller values = more thorough but slower
        const scaleFactor = confidence > 0.7 ? 1.05 : 1.1;

        // Min neighbors: higher values = fewer false positives but may miss faces
        const minNeighbors = confidence > 0.7 ? 5 : 3;

        const flags = 0;

        // Adaptive size limits based on image dimensions
        const imageSize = Math.min(src.cols, src.rows);
        const minFaceSize = Math.max(20, Math.floor(imageSize * 0.05));  // 5% of image size
        const maxFaceSize = Math.min(500, Math.floor(imageSize * 0.8));   // 80% of image size

        const minSize = new cv.Size(minFaceSize, minFaceSize);
        const maxSize = new cv.Size(maxFaceSize, maxFaceSize);

        console.log(`   Detection params: scale=${scaleFactor}, neighbors=${minNeighbors}, size=${minFaceSize}-${maxFaceSize}`);

        // Try multiple detection passes for better results
        let totalDetected = 0;

        // First pass: Standard detection
        faceCascade.detectMultiScale(equalizedGray, faces, scaleFactor, minNeighbors, flags, minSize, maxSize);
        totalDetected += faces.size();
        console.log(`👤 First pass found ${faces.size()} faces`);

        // Second pass: More sensitive detection if first pass found few faces
        if (faces.size() < 2) {
            const sensitiveFaces = new cv.RectVector();
            const sensitiveNeighbors = Math.max(2, minNeighbors - 1);
            const sensitiveScale = scaleFactor + 0.05; // Slightly less sensitive

            faceCascade.detectMultiScale(equalizedGray, sensitiveFaces, sensitiveScale, sensitiveNeighbors, flags, minSize, maxSize);

            // Merge results, avoiding duplicates
            for (let i = 0; i < sensitiveFaces.size(); i++) {
                const newFace = sensitiveFaces.get(i);
                let isDuplicate = false;

                // Check for overlap with existing faces
                for (let j = 0; j < faces.size(); j++) {
                    const existingFace = faces.get(j);
                    const overlapX = Math.max(0, Math.min(existingFace.x + existingFace.width, newFace.x + newFace.width) - Math.max(existingFace.x, newFace.x));
                    const overlapY = Math.max(0, Math.min(existingFace.y + existingFace.height, newFace.y + newFace.height) - Math.max(existingFace.y, newFace.y));
                    const overlapArea = overlapX * overlapY;
                    const newFaceArea = newFace.width * newFace.height;

                    if (overlapArea / newFaceArea > 0.5) { // 50% overlap threshold
                        isDuplicate = true;
                        break;
                    }
                }

                if (!isDuplicate) {
                    faces.push_back(newFace);
                }
            }

            console.log(`👤 Second pass added ${faces.size() - totalDetected} more faces`);
            totalDetected = faces.size();
            sensitiveFaces.delete();
        }

        // If enabled and available, also try profile detection
        const multiFaceEnabled = document.getElementById('multiFaceCheckbox').checked;
        if (multiFaceEnabled && profileCascade && !profileCascade.empty()) {
            const profileFaces = new cv.RectVector();
            profileCascade.detectMultiScale(equalizedGray, profileFaces, scaleFactor, minNeighbors, flags, minSize, maxSize);

            console.log(`👤 Profile cascade found ${profileFaces.size()} additional faces`);

            // Merge profile faces with frontal faces (simple approach - could be improved with NMS)
            for (let i = 0; i < profileFaces.size(); i++) {
                const profileFace = profileFaces.get(i);
                let isOverlapping = false;

                // Check for overlap with existing frontal faces
                for (let j = 0; j < faces.size(); j++) {
                    const frontalFace = faces.get(j);
                    const overlapX = Math.max(0, Math.min(frontalFace.x + frontalFace.width, profileFace.x + profileFace.width) - Math.max(frontalFace.x, profileFace.x));
                    const overlapY = Math.max(0, Math.min(frontalFace.y + frontalFace.height, profileFace.y + profileFace.height) - Math.max(frontalFace.y, profileFace.y));
                    const overlapArea = overlapX * overlapY;
                    const profileArea = profileFace.width * profileFace.height;

                    if (overlapArea / profileArea > 0.3) { // 30% overlap threshold
                        isOverlapping = true;
                        break;
                    }
                }

                if (!isOverlapping) {
                    faces.push_back(profileFace);
                }
            }

            profileFaces.delete();
        }

        // Log face details
        for (let i = 0; i < faces.size(); i++) {
            const face = faces.get(i);
            console.log(`   Face ${i + 1}: [${face.x}, ${face.y}, ${face.width}, ${face.height}]`);
        }

        gray.delete();
        equalizedGray.delete();

    } catch (error) {
        console.error('❌ Haar cascade error:', error);
        throw error;
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

        // Create final canvas with exact specified dimensions
        const cropCanvas = document.createElement('canvas');
        const ctx = cropCanvas.getContext('2d');

        cropCanvas.width = outputWidth;
        cropCanvas.height = outputHeight;

        // Fill with background color
        ctx.fillStyle = '#f0f0f0';
        ctx.fillRect(0, 0, outputWidth, outputHeight);

        let scale, scaledWidth, scaledHeight;

        if (maintainAspectRatio) {
            // Calculate scale to fit the face inside the specified dimensions
            const scaleX = outputWidth / cropW;
            const scaleY = outputHeight / cropH;
            scale = Math.min(scaleX, scaleY);
            scaledWidth = cropW * scale;
            scaledHeight = cropH * scale;
        } else {
            // Stretch to fill entire output dimensions
            scaledWidth = outputWidth;
            scaledHeight = outputHeight;
        }

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
    console.log('🌐 Page loaded - Haar cascade version');
    loadOpenCV();
});