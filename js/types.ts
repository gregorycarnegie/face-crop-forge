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
    bbox: PixelBoundingBox;
    selected: boolean;
    canvas?: HTMLCanvasElement;
    id?: number | string;
    box?: {
        xMin: number;
        yMin: number;
        width: number;
        height: number;
    };
    confidence?: number;
}

export interface CropResult {
    canvas: HTMLCanvasElement;
    bbox: PixelBoundingBox;
    face?: FaceData;
    image?: HTMLCanvasElement;
    fileName?: string;
}

export interface ImageData {
    id: string;
    file: File;
    image: HTMLImageElement;
    faces: FaceData[];
    results: CropResult[];
    selected: boolean;
    processed: boolean;
    status?: string;
    csvOutputName?: string;
    page?: number;
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

export interface ProcessingState {
    images: Map<string, ImageData>;
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
