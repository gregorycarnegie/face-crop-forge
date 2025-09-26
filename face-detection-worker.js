// Face Detection Web Worker
// This worker handles face detection in a separate thread to prevent UI blocking

let detector = null;
let isInitialized = false;

// Import TensorFlow.js and MediaPipe dependencies
self.importScripts('https://cdn.jsdelivr.net/npm/@tensorflow/tfjs-core');
self.importScripts('https://cdn.jsdelivr.net/npm/@tensorflow/tfjs-converter');
self.importScripts('https://cdn.jsdelivr.net/npm/@tensorflow/tfjs-backend-webgl');
self.importScripts('https://cdn.jsdelivr.net/npm/@tensorflow-models/face-landmarks-detection');

async function initializeDetector() {
    try {
        // Wait for TensorFlow to be ready
        await tf.ready();

        const model = faceLandmarksDetection.SupportedModels.MediaPipeFaceMesh;
        const detectorConfig = {
            runtime: 'tfjs',
            refineLandmarks: false,
            maxFaces: 20,
            solutionPath: 'https://cdn.jsdelivr.net/npm/@mediapipe/face_mesh'
        };

        detector = await faceLandmarksDetection.createDetector(model, detectorConfig);
        isInitialized = true;

        self.postMessage({
            type: 'initialized',
            success: true
        });
    } catch (error) {
        self.postMessage({
            type: 'initialized',
            success: false,
            error: error.message
        });
    }
}

async function detectFaces(imageData, options = {}) {
    if (!isInitialized || !detector) {
        throw new Error('Detector not initialized');
    }

    try {
        // Create canvas from image data
        const canvas = new OffscreenCanvas(imageData.width, imageData.height);
        const ctx = canvas.getContext('2d');

        // Create ImageData and put it on canvas
        const imgData = new ImageData(imageData.data, imageData.width, imageData.height);
        ctx.putImageData(imgData, 0, 0);

        // Detect faces
        const faces = await detector.estimateFaces(canvas);
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

                // Calculate confidence score
                const confidence = calculateConfidenceScore(keypoints);

                // Calculate face quality if requested
                let quality = null;
                if (options.includeQuality) {
                    quality = await calculateFaceQuality(canvas, minX, minY, maxX - minX, maxY - minY);
                }

                detectedFaces.push({
                    id: `face_${i}`,
                    x: minX,
                    y: minY,
                    width: maxX - minX,
                    height: maxY - minY,
                    confidence: confidence,
                    quality: quality,
                    selected: true,
                    index: i + 1
                });
            }
        }

        return detectedFaces;
    } catch (error) {
        throw new Error(`Face detection failed: ${error.message}`);
    }
}

function calculateConfidenceScore(keypoints) {
    if (keypoints.length === 0) return 0;

    const xs = keypoints.map(p => p.x);
    const ys = keypoints.map(p => p.y);
    const width = Math.max(...xs) - Math.min(...xs);
    const height = Math.max(...ys) - Math.min(...ys);

    const score = Math.min(0.95, (width * height) / 50000 + 0.3);
    return Math.max(0.1, score);
}

async function calculateFaceQuality(canvas, x, y, width, height) {
    try {
        const ctx = canvas.getContext('2d');
        const imageData = ctx.getImageData(x, y, width, height);
        const data = imageData.data;

        // Calculate Laplacian variance for blur detection
        const laplacianVariance = calculateLaplacianVariance(data, width, height);

        if (laplacianVariance > 1000) return { score: laplacianVariance, level: 'high' };
        if (laplacianVariance > 300) return { score: laplacianVariance, level: 'medium' };
        return { score: laplacianVariance, level: 'low' };
    } catch (error) {
        return { score: 0, level: 'unknown' };
    }
}

function calculateLaplacianVariance(data, width, height) {
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

// Message handler
self.onmessage = async function(e) {
    const { type, data, id } = e.data;

    try {
        switch (type) {
            case 'initialize':
                await initializeDetector();
                break;

            case 'detectFaces':
                if (!isInitialized) {
                    throw new Error('Worker not initialized');
                }

                const faces = await detectFaces(data.imageData, data.options || {});
                self.postMessage({
                    type: 'faceDetectionResult',
                    id: id,
                    success: true,
                    faces: faces
                });
                break;

            default:
                throw new Error(`Unknown message type: ${type}`);
        }
    } catch (error) {
        self.postMessage({
            type: 'error',
            id: id,
            error: error.message
        });
    }
};

// Initialize on worker startup
initializeDetector();