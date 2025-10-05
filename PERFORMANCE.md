# Performance Optimizations

This document describes the performance optimizations implemented in Face Crop Forge.

## Worker Thread Optimizations

### 1. OffscreenCanvas in Web Worker

All image operations including rotation, resizing, **cropping**, and **enhancement** are now performed inside the Web Worker using OffscreenCanvas:

- **Zero main thread blocking** - ALL heavy image operations happen off the main thread
- **Batch processing** - Process hundreds of crops without UI jank
- **GPU acceleration** - OffscreenCanvas can leverage GPU when available

**Implementation**: See `src/face-detection-worker.ts` → `processImageInWorker()`, `cropFaceInWorker()`, `enhanceImageInWorker()`

```typescript
// Worker handles rotation, flipping, and resizing
const processedBitmap = await processImageInWorker(imageBitmap, {
    width: 512,
    height: 512,
    rotation: 90,
    flipH: false,
    flipV: false
});

// Worker handles face cropping (NEW!)
const croppedBitmap = await cropFaceInWorker(imageBitmap, {
    cropX: 100,
    cropY: 150,
    cropWidth: 200,
    cropHeight: 200,
    outputWidth: 512,
    outputHeight: 512
});

// Worker handles image enhancements (NEW!)
const enhancedBitmap = await enhanceImageInWorker(imageBitmap, {
    autoColorCorrection: true,
    exposure: 0.5,
    contrast: 1.2,
    sharpness: 0.8
});
```

### 2. ImageBitmap Transfers (Zero-Copy)

Instead of cloning image data, we use transferable ImageBitmap objects:

- **No serialization overhead** - ImageBitmap ownership transfers instantly
- **Memory efficient** - No duplicate copies in memory
- **Faster communication** - Structured cloning avoided

**Before** (slow, copies memory):

```typescript
// Old approach - copies ~4MB for a 1920x1080 image
const imageData = ctx.getImageData(0, 0, width, height);
worker.postMessage({ imageData });  // 🐌 Slow copy
```

**After** (fast, zero-copy):

```typescript
// New approach - transfers ownership, ~instant
const bitmap = await createImageBitmap(image);
worker.postMessage({ bitmap }, [bitmap]);  // ⚡ Zero-copy transfer
```

**Implementation**: See `src/base-face-cropper.ts` → `detectFacesWithWorker()`

## WASM Optimizations

### 3. COOP/COEP Headers for SharedArrayBuffer

The server sets Cross-Origin headers to enable SharedArrayBuffer, unlocking:

- **WASM SIMD** - Single Instruction Multiple Data for 2-4× faster math operations
- **WASM threads** - Multi-threaded WASM execution
- **1.5-3× performance boost** on bulk face detection jobs (Chromium browsers)

**Headers** (see `src/server.ts`):

```text
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
```

These headers enable `SharedArrayBuffer`, which MediaPipe Tasks Vision uses for:

- Parallel WASM execution
- SIMD vectorized operations
- Multi-threaded face detection

## Usage Recommendations

### For Single Image Processing

- Use the standard API - optimizations are automatic

### For Batch Processing (10+ images)

1. **Enable Web Workers** in settings (enabled by default)
2. **Use the batch processor** (`batch-processing.html`)
3. Images will automatically use ImageBitmap transfers

### For Maximum Performance

1. Serve via the included Node.js server (`npm start`)
2. Use Chrome/Edge browsers (best WASM support)
3. Enable Web Workers in settings
4. Process images in batches of 20-50 for optimal throughput

## Browser Support

| Feature | Chrome/Edge | Firefox | Safari |
|---------|------------|---------|---------|
| OffscreenCanvas | ✅ Full | ✅ Full | ✅ Full |
| ImageBitmap transfers | ✅ Full | ✅ Full | ✅ Full |
| WASM SIMD | ✅ Full | ⚠️ Partial | ❌ Limited |
| WASM threads | ✅ Full | ⚠️ Partial | ❌ No |

## Implementation Details

### Worker Message Protocol

The worker now accepts both legacy ImageData and modern ImageBitmap:

```typescript
// Legacy support (backward compatible)
worker.postMessage({
    type: 'detectFaces',
    data: { imageData: { data, width, height } }
});

// New optimized path
worker.postMessage({
    type: 'detectFaces',
    data: { imageBitmap: bitmap }
}, [bitmap]);  // Transfer list
```

### OffscreenCanvas Image Processing

All transformations happen in the worker:

- Rotation
- Flipping (horizontal/vertical)
- Resizing
- **Face cropping** (NEW!)
- **Image enhancement** (auto color, exposure, contrast, sharpness) (NEW!)
- Quality adjustments

The main thread only receives the final processed ImageBitmap.

## Future Optimizations

Potential improvements for consideration:

1. **Batch ImageBitmap creation** - Create multiple bitmaps in parallel
2. **Worker pool** - Multiple workers for CPU parallelism
3. ~~**Offscreen face cropping**~~ ✅ **DONE!**
4. ~~**Offscreen enhancement**~~ ✅ **DONE!**
5. **WebGPU integration** - GPU compute for image processing (experimental)

## Monitoring Performance

To measure performance in your application:

```javascript
// Enable performance logging
const startTime = performance.now();
const faces = await detectFaces(image);
console.log(`Detection took: ${performance.now() - startTime}ms`);
```

Check the browser console for automatic performance metrics when using the batch processor.
