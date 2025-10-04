// Type definitions for the Face Cropper application

export interface BoundingBox {
    originX: number;
    originY: number;
    width: number;
    height: number;
}

export interface PixelBoundingBox {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface FaceDetection {
    boundingBox: BoundingBox;
    score?: number;
    categories?: Array<{
        score: number;
        index?: number;
        categoryName?: string;
        displayName?: string;
    }>;
}

export interface FaceData {
    id: number | string;
    bbox: PixelBoundingBox;
    box?: {
        xMin: number;
        yMin: number;
        width: number;
        height: number;
    };
    selected: boolean;
    confidence?: number;
    // Extended properties for batch processing
    x?: number;
    y?: number;
    width?: number;
    height?: number;
    index?: number;
    quality?: {
        score: number;
        level: string;
    };
    // Optional properties
    canvas?: HTMLCanvasElement;
    score?: number;
    landmarks?: Array<{ x: number; y: number }>;
}

export interface CropResult {
    canvas?: HTMLCanvasElement;
    bbox?: PixelBoundingBox;
    face?: FaceData;
    image?: HTMLCanvasElement;
    fileName?: string;
    // Extended properties for batch processing
    dataUrl?: string;
    faceIndex?: number;
    faceId?: string;
    sourceImage?: string;
    filename?: string;
    format?: string;
    quality?: number;
    // Batch processor properties
    blobUrl?: string;
    width?: number;
    height?: number;
}

export interface ProcessorImageData {
    id: string;
    file: File;
    image: HTMLImageElement;
    faces: FaceData[];
    results: CropResult[];
    selected: boolean;
    processed: boolean;
    status?: string | 'loaded' | 'processing' | 'processed' | 'error';
    csvOutputName?: string;
    page?: number;
    // Batch processor extended properties
    enhancedImage?: HTMLImageElement | null;
    processedAt?: number;
    memoryCleanedUp?: boolean;
}

export interface Statistics {
    totalFacesDetected: number;
    imagesProcessed: number;
    successfulProcessing: number;
    processingTimes: number[];
    startTime: number | null;
}

export interface CropSettings {
    outputWidth: number;
    outputHeight: number;
    faceHeightPct: number;
    positioningMode: string;
    verticalOffset: number;
    horizontalOffset: number;
    outputFormat: string;
    jpegQuality: number;
    namingTemplate: string;
    format?: string; // Alias for outputFormat
    quality?: number; // Alias for jpegQuality
}

// Log types
export type LogLevel = 'info' | 'success' | 'warning' | 'error';
export type ErrorSeverity = 'error' | 'critical' | 'warning' | 'info';

export interface ProcessingLogEntry {
    timestamp: string;
    message: string;
    type: LogLevel;
}

export interface ErrorLogEntry {
    timestamp: string;
    title: string;
    details: string;
    severity: ErrorSeverity;
}

export interface ProcessingState {
    images: Map<string, ProcessorImageData>;
    settings: CropSettings;
    selectedImages: string[];
}

// MediaPipe Vision types
declare global {
    interface Window {
        vision?: {
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
        };
    }

    interface FaceDetector {
        detect(image: HTMLImageElement): FaceDetectorResult;
        detectForVideo(
            video: HTMLVideoElement,
            timestamp: number
        ): FaceDetectorResult;
    }

    interface FaceDetectorResult {
        detections: FaceDetection[];
    }

    // JSZip library (loaded via CDN)
    class JSZip {
        constructor();
        file(name: string, data: Blob | string, options?: { base64?: boolean }): void;
        generateAsync(options: { type: string }): Promise<Blob>;
    }
}

export {};
