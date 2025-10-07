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

interface ImageBitmapMessage {
    bitmap: ImageBitmap;
    options?: DetectionOptions;
}

interface WorkerMessage {
    type: string;
    data?: {
        imageData?: ImageDataMessage;
        imageBitmap?: ImageBitmap;
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
            // @ts-ignore - Local module without type declarations
            vision = await import('/public/models/vision_bundle.mjs') as VisionModule;
        }

        console.log('Initializing MediaPipe Tasks Vision in worker...');

        // Initialize the MediaPipe Vision tasks
        const visionFileset = await vision.FilesetResolver.forVisionTasks(
            "/public/models/wasm"
        );

        // Create face detector with WebAssembly runtime
        detector = await vision.FaceDetector.createFromOptions(visionFileset, {
            baseOptions: {
                modelAssetPath: `/models/blaze_face_short_range.tflite`,
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

async function detectFaces(input: ImageDataMessage | ImageBitmap, options: DetectionOptions = {}): Promise<DetectedFace[]> {
    if (!isInitialized || !detector) {
        throw new Error('Detector not initialized');
    }

    try {
        let canvas: OffscreenCanvas;
        let ctx: OffscreenCanvasRenderingContext2D | null;

        // Handle ImageBitmap input (transferable, zero-copy)
        if (input instanceof ImageBitmap) {
            canvas = new OffscreenCanvas(input.width, input.height);
            ctx = canvas.getContext('2d', { willReadFrequently: true });

            if (!ctx) {
                throw new Error('Failed to get canvas context');
            }

            ctx.drawImage(input, 0, 0);
            // Close the bitmap to free memory
            input.close();
        } else {
            // Handle legacy ImageData input
            canvas = new OffscreenCanvas(input.width, input.height);
            ctx = canvas.getContext('2d', { willReadFrequently: true });

            if (!ctx) {
                throw new Error('Failed to get canvas context');
            }

            // Create ImageData and put it on canvas
            const imgData = new ImageData(
                new Uint8ClampedArray(input.data),
                input.width,
                input.height
            );
            ctx.putImageData(imgData, 0, 0);
        }

        // Detect faces using MediaPipe Tasks Vision
        const detectionResult = await detector.detect(canvas);
        const detectedFaces: DetectedFace[] = [];

        if (detectionResult.detections && detectionResult.detections.length > 0) {
            for (let i = 0; i < detectionResult.detections.length; i++) {
                const detection = detectionResult.detections[i];
                const {boundingBox} = detection;

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
        const {data} = imageData;

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

// Process image transformations in worker thread using OffscreenCanvas
async function processImageInWorker(
    bitmap: ImageBitmap,
    options: any = {}
): Promise<ImageBitmap> {
    const { width, height, rotation, flipH, flipV, quality } = options;

    // Use OffscreenCanvas for all transformations
    const canvas = new OffscreenCanvas(
        width || bitmap.width,
        height || bitmap.height
    );
    const ctx = canvas.getContext('2d', {
        willReadFrequently: false,
        alpha: true
    });

    if (!ctx) {
        throw new Error('Failed to get OffscreenCanvas context');
    }

    // Apply transformations
    ctx.save();

    // Handle rotation if specified
    if (rotation) {
        ctx.translate(canvas.width / 2, canvas.height / 2);
        ctx.rotate((rotation * Math.PI) / 180);
        ctx.translate(-canvas.width / 2, -canvas.height / 2);
    }

    // Handle flipping
    const scaleX = flipH ? -1 : 1;
    const scaleY = flipV ? -1 : 1;
    const translateX = flipH ? canvas.width : 0;
    const translateY = flipV ? canvas.height : 0;

    if (flipH || flipV) {
        ctx.translate(translateX, translateY);
        ctx.scale(scaleX, scaleY);
    }

    // Draw with high-quality settings
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';

    // Draw the bitmap
    ctx.drawImage(
        bitmap,
        0, 0, bitmap.width, bitmap.height,
        0, 0, canvas.width, canvas.height
    );

    ctx.restore();

    // Close original bitmap to free memory
    bitmap.close();

    // Convert back to ImageBitmap for transfer
    return await createImageBitmap(canvas);
}

// Crop a face from an ImageBitmap in worker thread
async function cropFaceInWorker(
    bitmap: ImageBitmap,
    cropParams: {
        cropX: number;
        cropY: number;
        cropWidth: number;
        cropHeight: number;
        outputWidth: number;
        outputHeight: number;
    }
): Promise<ImageBitmap> {
    const { cropX, cropY, cropWidth, cropHeight, outputWidth, outputHeight } = cropParams;

    // Create OffscreenCanvas for cropping
    const canvas = new OffscreenCanvas(outputWidth, outputHeight);
    const ctx = canvas.getContext('2d', {
        willReadFrequently: false,
        alpha: true
    });

    if (!ctx) {
        throw new Error('Failed to get OffscreenCanvas context');
    }

    // High-quality rendering
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';

    // Crop and resize in one operation
    ctx.drawImage(
        bitmap,
        cropX, cropY, cropWidth, cropHeight,
        0, 0, outputWidth, outputHeight
    );

    // Close original bitmap to free memory
    bitmap.close();

    // Convert to ImageBitmap for transfer
    return await createImageBitmap(canvas);
}

// Apply image enhancements in worker thread
async function enhanceImageInWorker(
    bitmap: ImageBitmap,
    enhancements: {
        autoColorCorrection?: boolean;
        exposure?: number;
        contrast?: number;
        sharpness?: number;
        skinSmoothing?: number;
        backgroundBlur?: number;
    }
): Promise<ImageBitmap> {
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const ctx = canvas.getContext('2d', { willReadFrequently: true });

    if (!ctx) {
        throw new Error('Failed to get OffscreenCanvas context');
    }

    // Draw original image
    ctx.drawImage(bitmap, 0, 0);

    // Get image data for pixel manipulation
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const {data} = imageData;

    // Apply auto color correction
    if (enhancements.autoColorCorrection) {
        applyAutoColorCorrection(data);
    }

    // Apply exposure adjustment
    if (enhancements.exposure && enhancements.exposure !== 0) {
        const exposureFactor = Math.pow(2, enhancements.exposure);
        for (let i = 0; i < data.length; i += 4) {
            data[i] = Math.min(255, data[i] * exposureFactor);
            data[i + 1] = Math.min(255, data[i + 1] * exposureFactor);
            data[i + 2] = Math.min(255, data[i + 2] * exposureFactor);
        }
    }

    // Apply contrast adjustment
    if (enhancements.contrast && enhancements.contrast !== 1) {
        const factor = enhancements.contrast;
        const intercept = 128 * (1 - factor);
        for (let i = 0; i < data.length; i += 4) {
            data[i] = Math.max(0, Math.min(255, data[i] * factor + intercept));
            data[i + 1] = Math.max(0, Math.min(255, data[i + 1] * factor + intercept));
            data[i + 2] = Math.max(0, Math.min(255, data[i + 2] * factor + intercept));
        }
    }

    // Put enhanced data back
    ctx.putImageData(imageData, 0, 0);

    // Apply sharpness if needed (using canvas filter for performance)
    if (enhancements.sharpness && enhancements.sharpness > 0) {
        ctx.filter = `contrast(${1 + enhancements.sharpness * 0.1})`;
        ctx.drawImage(canvas, 0, 0);
        ctx.filter = 'none';
    }

    // Close original bitmap
    bitmap.close();

    // Return enhanced ImageBitmap
    return await createImageBitmap(canvas);
}

// Auto color correction helper
function applyAutoColorCorrection(data: Uint8ClampedArray): void {
    let rSum = 0, gSum = 0, bSum = 0;
    const pixelCount = data.length / 4;

    // Calculate averages
    for (let i = 0; i < data.length; i += 4) {
        rSum += data[i];
        gSum += data[i + 1];
        bSum += data[i + 2];
    }

    const rAvg = rSum / pixelCount;
    const gAvg = gSum / pixelCount;
    const bAvg = bSum / pixelCount;
    const overallAvg = (rAvg + gAvg + bAvg) / 3;

    // Calculate correction factors
    const rFactor = overallAvg / rAvg;
    const gFactor = overallAvg / gAvg;
    const bFactor = overallAvg / bAvg;

    // Apply correction
    for (let i = 0; i < data.length; i += 4) {
        data[i] = Math.min(255, data[i] * rFactor);
        data[i + 1] = Math.min(255, data[i + 1] * gFactor);
        data[i + 2] = Math.min(255, data[i + 2] * bFactor);
    }
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

                // Prefer ImageBitmap if available, fallback to ImageData
                const input = data.imageBitmap || data.imageData;
                if (!input) {
                    throw new Error('No image data provided');
                }

                const faces = await detectFaces(input, data.options || {});
                self.postMessage({
                    type: 'faceDetectionResult',
                    id: id,
                    success: true,
                    faces: faces
                } as WorkerResponse);
                break;

            case 'processImage':
                if (!isInitialized) {
                    throw new Error('Worker not initialized');
                }

                if (!data || !data.imageBitmap) {
                    throw new Error('No ImageBitmap provided');
                }

                // Process image transformations on worker thread using OffscreenCanvas
                const processedBitmap = await processImageInWorker(
                    data.imageBitmap,
                    data.options || {}
                );

                // Use structured clone with transfer for zero-copy
                self.postMessage({
                    type: 'imageProcessed',
                    id: id,
                    success: true,
                    bitmap: processedBitmap
                }, { transfer: [processedBitmap] } as any);
                break;

            case 'cropFace':
                if (!data || !data.imageBitmap) {
                    throw new Error('No ImageBitmap provided for cropping');
                }

                const cropParams = data.options as any;
                if (!cropParams) {
                    throw new Error('No crop parameters provided');
                }

                // Crop face in worker thread using OffscreenCanvas
                const croppedBitmap = await cropFaceInWorker(
                    data.imageBitmap,
                    cropParams
                );

                self.postMessage({
                    type: 'faceCropped',
                    id: id,
                    success: true,
                    bitmap: croppedBitmap
                }, { transfer: [croppedBitmap] } as any);
                break;

            case 'enhanceImage':
                if (!data || !data.imageBitmap) {
                    throw new Error('No ImageBitmap provided for enhancement');
                }

                const enhancements = (data.options || {}) as any;

                // Enhance image in worker thread
                const enhancedBitmap = await enhanceImageInWorker(
                    data.imageBitmap,
                    enhancements
                );

                self.postMessage({
                    type: 'imageEnhanced',
                    id: id,
                    success: true,
                    bitmap: enhancedBitmap
                }, { transfer: [enhancedBitmap] } as any);
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
