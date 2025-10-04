// Face Detection Web Worker
// This worker handles face detection in a separate thread to prevent UI blocking

interface FaceQuality {
    score: number;
    level: 'high' | 'medium' | 'low' | 'unknown';
}

interface DetectedFace {
    id: string;
    x: number;
    y: number;
    width: number;
    height: number;
    confidence: number;
    quality: FaceQuality | null;
    selected: boolean;
    index: number;
}

interface DetectionOptions {
    includeQuality?: boolean;
}

interface ImageDataMessage {
    data: Uint8ClampedArray;
    width: number;
    height: number;
}

interface WorkerMessage {
    type: string;
    data?: {
        imageData: ImageDataMessage;
        options?: DetectionOptions;
    };
    id?: string;
}

interface WorkerResponse {
    type: string;
    id?: string;
    success?: boolean;
    error?: string;
    faces?: DetectedFace[];
}

// MediaPipe Vision types
interface BoundingBox {
    originX: number;
    originY: number;
    width: number;
    height: number;
}

interface Detection {
    boundingBox: BoundingBox;
    categories: Array<{ score: number }>;
}

interface DetectionResult {
    detections: Detection[];
}

interface FaceDetector {
    detect(image: OffscreenCanvas): Promise<DetectionResult>;
}

interface VisionModule {
    FilesetResolver: {
        forVisionTasks(wasmPath: string): Promise<any>;
    };
    FaceDetector: {
        createFromOptions(
            fileset: any,
            options: {
                baseOptions: {
                    modelAssetPath: string;
                    delegate: string;
                };
                runningMode: string;
            }
        ): Promise<FaceDetector>;
    };
}

let detector: FaceDetector | null = null;
let isInitialized: boolean = false;

// Import MediaPipe Tasks Vision using dynamic import
let vision: VisionModule | null = null;

async function initializeDetector(): Promise<void> {
    try {
        // Dynamic import of MediaPipe Tasks Vision
        if (!vision) {
            // @ts-ignore - CDN module without type declarations
            vision = await import('https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/vision_bundle.mjs') as VisionModule;
        }

        console.log('Initializing MediaPipe Tasks Vision in worker...');

        // Initialize the MediaPipe Vision tasks
        const visionFileset = await vision.FilesetResolver.forVisionTasks(
            "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm"
        );

        // Create face detector with WebAssembly runtime
        detector = await vision.FaceDetector.createFromOptions(visionFileset, {
            baseOptions: {
                modelAssetPath: `https://storage.googleapis.com/mediapipe-models/face_detector/blaze_face_short_range/float16/1/blaze_face_short_range.tflite`,
                delegate: "GPU"
            },
            runningMode: "IMAGE"
        });

        isInitialized = true;

        self.postMessage({
            type: 'initialized',
            success: true
        } as WorkerResponse);
    } catch (error) {
        self.postMessage({
            type: 'initialized',
            success: false,
            error: (error as Error).message
        } as WorkerResponse);
    }
}

async function detectFaces(imageData: ImageDataMessage, options: DetectionOptions = {}): Promise<DetectedFace[]> {
    if (!isInitialized || !detector) {
        throw new Error('Detector not initialized');
    }

    try {
        // Create canvas from image data
        const canvas = new OffscreenCanvas(imageData.width, imageData.height);
        const ctx = canvas.getContext('2d', { willReadFrequently: true });

        if (!ctx) {
            throw new Error('Failed to get canvas context');
        }

        // Create ImageData and put it on canvas
        const imgData = new ImageData(
            new Uint8ClampedArray(imageData.data),
            imageData.width,
            imageData.height
        );
        ctx.putImageData(imgData, 0, 0);

        // Detect faces using MediaPipe Tasks Vision
        const detectionResult = await detector.detect(canvas);
        const detectedFaces: DetectedFace[] = [];

        if (detectionResult.detections && detectionResult.detections.length > 0) {
            for (let i = 0; i < detectionResult.detections.length; i++) {
                const detection = detectionResult.detections[i];
                const boundingBox = detection.boundingBox;

                // Calculate face quality if requested
                let quality: FaceQuality | null = null;
                if (options.includeQuality) {
                    quality = await calculateFaceQuality(ctx, boundingBox.originX, boundingBox.originY, boundingBox.width, boundingBox.height);
                }

                detectedFaces.push({
                    id: `face_${i}`,
                    x: boundingBox.originX,
                    y: boundingBox.originY,
                    width: boundingBox.width,
                    height: boundingBox.height,
                    confidence: detection.categories[0]?.score || 0.5,
                    quality: quality,
                    selected: true,
                    index: i + 1
                });
            }
        }

        return detectedFaces;
    } catch (error) {
        throw new Error(`Face detection failed: ${(error as Error).message}`);
    }
}

async function calculateFaceQuality(
    ctx: OffscreenCanvasRenderingContext2D,
    x: number,
    y: number,
    width: number,
    height: number
): Promise<FaceQuality> {
    try {
        const sourceCanvas = ctx.canvas;

        const safeX = Math.max(0, Math.floor(x));
        const safeY = Math.max(0, Math.floor(y));
        const safeWidth = Math.max(1, Math.floor(Math.min(width, sourceCanvas.width - safeX)));
        const safeHeight = Math.max(1, Math.floor(Math.min(height, sourceCanvas.height - safeY)));

        const qualityCanvas = new OffscreenCanvas(safeWidth, safeHeight);
        const qualityCtx = qualityCanvas.getContext('2d', { willReadFrequently: true });

        if (!qualityCtx) {
            return { score: 0, level: 'unknown' };
        }

        qualityCtx.drawImage(
            sourceCanvas,
            safeX,
            safeY,
            safeWidth,
            safeHeight,
            0,
            0,
            safeWidth,
            safeHeight
        );

        const imageData = qualityCtx.getImageData(0, 0, safeWidth, safeHeight);
        const data = imageData.data;

        // Calculate Laplacian variance for blur detection
        const laplacianVariance = calculateLaplacianVariance(data, safeWidth, safeHeight);

        if (laplacianVariance > 1000) return { score: laplacianVariance, level: 'high' };
        if (laplacianVariance > 300) return { score: laplacianVariance, level: 'medium' };
        return { score: laplacianVariance, level: 'low' };
    } catch (error) {
        return { score: 0, level: 'unknown' };
    }
}

function calculateLaplacianVariance(data: Uint8ClampedArray, width: number, height: number): number {
    const gray: number[] = [];
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
self.onmessage = async function(e: MessageEvent<WorkerMessage>): Promise<void> {
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

                if (!data) {
                    throw new Error('No data provided');
                }

                const faces = await detectFaces(data.imageData, data.options || {});
                self.postMessage({
                    type: 'faceDetectionResult',
                    id: id,
                    success: true,
                    faces: faces
                } as WorkerResponse);
                break;

            default:
                throw new Error(`Unknown message type: ${type}`);
        }
    } catch (error) {
        self.postMessage({
            type: 'error',
            id: id,
            error: (error as Error).message
        } as WorkerResponse);
    }
};

// Initialize on worker startup
initializeDetector();
