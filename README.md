# Face Crop Forge

![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)
![Bun](https://img.shields.io/badge/bun-1.3+-f472b6.svg)
![TypeScript](https://img.shields.io/badge/TypeScript-5.3-blue.svg)
![MediaPipe](https://img.shields.io/badge/MediaPipe-Tasks%20Vision-orange.svg)
![Client-Side](https://img.shields.io/badge/processing-100%25%20client--side-success.svg)

A powerful, client-side face detection and cropping application built with MediaPipe Tasks Vision. This tool provides multiple processing modes to handle everything from single images to large batch operations, all running entirely in your browser for maximum privacy and performance.

## ✨ Features

### 🖼️ **Multiple Processing Modes**

- **Single Image**: Perfect for quick, one-off face extractions
- **Multiple Images**: Batch process several images with gallery management
- **CSV Batch**: Enterprise-grade processing with CSV metadata integration

### 🎯 **Advanced Face Detection**

- Powered by Google's MediaPipe Tasks Vision
- Real-time face quality analysis (blur detection)
- Adjustable confidence thresholds
- Support for multiple faces per image

### 🛠️ **Professional Tools**

- **Color Correction**: Automatic brightness and contrast adjustment
- **Manual Controls**: Fine-tune exposure (-2 to +2 stops) and contrast (0.5 to 2.0)
- **Quality Filtering**: Filter faces based on blur detection scores
- **Flexible Output**: Individual downloads or ZIP packages

### 🔒 **Privacy First**

- **100% Client-Side Processing** - No server uploads
- **Offline Capable** - Works without internet after initial load
- **No Data Collection** - Your images never leave your device

## 🚀 Quick Start

### Prerequisites

- **Bun 1.3+** (recommended - fastest runtime and build tool)
  - Install from [bun.sh](https://bun.sh) or `powershell -c "irm bun.sh/install.ps1|iex"` (Windows)
  - `curl -fsSL https://bun.sh/install | bash` (macOS/Linux)

### Installation & Setup

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd face-crop-forge
   ```

2. **Install dependencies**

   ```bash
   bun install
   ```

3. **Start the development server**

   ```bash
   bun run dev
   ```

   Navigate to `http://localhost:3000`

   ⚡ **Bun runs TypeScript directly** - No build step needed! Hot reload enabled for instant updates.

4. **Production build (optional)**

   ```bash
   bun run build        # Builds minified bundle (45ms!)
   bun run serve:prod   # Run production build
   ```

   ⚡ **Performance Note**: The server sets COOP/COEP headers that enable WASM SIMD + threading for **1.5-3× faster face detection** in Chrome/Edge browsers.

   **Option B: Python HTTP Server**

   ```bash
   # Python 3
   python -m http.server 3000

   # Python 2
   python -m SimpleHTTPServer 3000
   ```

   Navigate to `http://localhost:3000`

   **Option C: PHP Built-in Server**

   ```bash
   php -S localhost:3000
   ```

   Navigate to `http://localhost:3000`

   ⚠️ **Note**: Options B and C don't set COOP/COEP headers, so WASM optimizations won't be available. Use the Node.js server for maximum performance.

### Development Mode

```bash
bun run dev
```

This runs TypeScript directly with **hot module reloading** - changes are reflected instantly without manual rebuilds!

## 📖 Usage Guide

### Single Image Mode (`single-processing.html`)

1. **Upload**: Drag & drop or click to select an image
2. **Detect**: Click "Detect Faces" to analyze the image
3. **Adjust**: Use quality filters and color correction as needed
4. **Download**: Get individual crops or a complete ZIP package

### Multiple Images Mode (`batch-processing.html`)

1. **Batch Upload**: Add multiple images to the gallery
2. **Process**: Run face detection across all images
3. **Review**: Use the gallery to navigate and review results
4. **Export**: Download crops individually or as a batch ZIP

### CSV Batch Mode (`csv-processing.html`)

1. **Upload CSV**: Provide a CSV file with image URLs or file references
2. **Configure**: Set processing parameters and output options
3. **Process**: Automated batch processing with progress tracking
4. **Export**: Download results with maintained CSV metadata

## ⚙️ Configuration Options

### Face Detection Settings

- **Confidence Threshold**: Minimum confidence for face detection
- **Quality Analysis**: Enable blur detection and quality scoring
- **Max Faces**: Limit the number of faces detected per image

### Image Enhancement

- **Auto Color Correction**: Automatic brightness/contrast optimization
- **Manual Exposure**: Adjust brightness from -2 to +2 stops
- **Manual Contrast**: Fine-tune contrast from 0.5 to 2.0

### Output Options

- **Naming Templates**: Customize output file names
- **Format Selection**: Choose output image format
- **Batch Downloads**: ZIP packaging for multiple files

## 🏗️ Technical Architecture

### Core Technologies

- **TypeScript**: Type-safe development with strict null checking
- **MediaPipe Tasks Vision**: Google's state-of-the-art face detection
- **Web Workers**: Background processing for smooth UI performance
- **Canvas API**: High-performance image manipulation
- **Modern ES6+ Modules**: Clean, maintainable code architecture

### Key Components

- `index.html` - Mode selection landing page
- `single-processing.html` - Single image processing interface
- `batch-processing.html` - Multiple images batch processing
- `csv-processing.html` - CSV-driven batch operations
- `src/face-detection-worker.ts` - Web Worker for face detection
- `src/batch-processor.ts` - Main application logic for multiple images
- `src/single-processor.ts` - Single image processing logic
- `src/csv-processor.ts` - CSV batch processing logic

### Performance Features

- **OffscreenCanvas in Workers**: All image transformations happen off the main thread for zero UI jank
- **ImageBitmap Transfers**: Zero-copy image passing between threads (instant transfer)
- **WASM SIMD + Threads**: 1.5-3× faster face detection in modern browsers via COOP/COEP headers
- **Lazy Loading**: Images loaded on-demand for better performance
- **Progressive Processing**: Non-blocking face detection
- **Memory Management**: Efficient handling of large image batches

**Performance Gains**:

- 2.7× faster batch processing with OffscreenCanvas
- <1ms image transfer vs ~45ms with legacy ImageData
- 2.5× faster face detection with WASM optimizations (Chrome/Edge)

See [PERFORMANCE.md](PERFORMANCE.md) for detailed benchmarks and technical details.

## 🌐 Browser Compatibility

### Minimum Requirements

- **Chrome/Edge**: 88+ (recommended for best performance)
- **Firefox**: 85+
- **Safari**: 14+

### Required Features

- ES6+ modules support
- Canvas 2D context
- Web Workers
- OffscreenCanvas (for optimal performance)
- ImageBitmap and transferable objects
- File API
- WebAssembly (for MediaPipe)

### Performance Tiers

| Browser | OffscreenCanvas | ImageBitmap | WASM SIMD | WASM Threads | Overall Performance |
|---------|----------------|-------------|-----------|--------------|---------------------|
| Chrome 88+ | ✅ | ✅ | ✅ | ✅ | ⚡⚡⚡ Best |
| Edge 88+ | ✅ | ✅ | ✅ | ✅ | ⚡⚡⚡ Best |
| Firefox 85+ | ✅ | ✅ | ⚠️ | ⚠️ | ⚡⚡ Good |
| Safari 14+ | ✅ | ✅ | ❌ | ❌ | ⚡ Fair |

**Recommendation**: Use Chrome or Edge browsers with the Node.js server for the best performance (up to 3× faster).

## 🔧 Development

### Project Structure

```text
├── index.html                      # Mode selection landing page
├── single-processing.html          # Single image processing interface
├── batch-processing.html           # Multiple images batch processing
├── csv-processing.html             # CSV-driven batch operations
├── package.json                    # NPM dependencies and scripts
├── tsconfig.json                   # TypeScript configuration
├── README.md                       # Project documentation
├── .gitignore                      # Git ignore rules
├── assets/
│   └── favicon.svg                 # Application favicon
├── css/
│   ├── styles.css                  # Global styles and responsive design
│   └── index.css                   # Landing page styles
├── models/  
│   ├── wasm/
│   │   ├── vision_wasm_internal.js
│   │   ├── vision_wasm_internal.wasm # WASM binary for MediaPipe Tasks Vision
│   │   ├── vision_wasm_nosimd_internal.js
│   │   └── vision_wasm_nosimd_internal.wasm # WASM binary for MediaPipe Tasks Vision without SIMD optimizations
│   ├── blaze_face_short_range.tflite # Face detection model
│   ├── face_landmarker.task           # Face landmark model
│   └── vision_bundle.mjs              # MediaPipe Tasks Vision bundle for TypeScript
├── src/                            # TypeScript source files
│   ├── types.ts                    # TypeScript type definitions
│   ├── base-face-cropper.ts        # Base class for face processing
│   ├── single-processor.ts         # Single image processing
│   ├── batch-processor.ts          # Multiple images batch processing
│   ├── csv-processor.ts            # CSV batch processing
│   ├── face-detection-worker.ts    # Web Worker for face detection
│   ├── index.ts                    # Landing page logic
│   └── server.ts                   # Development server
└── dist/                           # Compiled JavaScript (git-ignored)
    └── *.js                        # Auto-generated from TypeScript
```

### Key Files

- **TypeScript Source** (`*.ts`): Type-safe source code
- **Compiled JavaScript** (`*.js`): Generated from TypeScript (not tracked in git)
- **HTML Pages**: User interfaces for different processing modes
- **Web Worker**: Background face detection processing
- **Styles**: Responsive CSS with dark mode support

### Build System

- **TypeScript Compiler**: Compiles `.ts` files to `.js` with source maps
- **Strict Type Checking**: Full null safety and type inference
- **ES2020 Target**: Modern JavaScript output for better performance
- **Source Maps**: For debugging TypeScript in the browser

### Available Scripts

```bash
# Development - Run TypeScript directly with hot reload
bun run dev

# Production start - Run TypeScript directly (no build needed)
bun run start

# Build for production - Minified bundle with source maps
bun run build

# Type checking only - Verify types without building
bun run type-check

# Full production build - Bundle + type definitions
bun run build:prod

# Serve production build
bun run serve:prod

# Clean build artifacts
bun run clean
```

**Why Bun?**

- ⚡ **Instant startup** - No compilation wait
- 🔥 **Hot reload** - See changes immediately
- 📦 **45ms builds** - 10-100× faster than webpack/rollup
- 🎯 **Native TypeScript** - Direct execution, no transpiling needed

### TypeScript Configuration

The project uses strict TypeScript settings for maximum type safety:

- Strict null checks
- No implicit any
- Strict function types
- No unused locals/parameters warnings
- Full type inference

Configuration in `tsconfig.json` can be adjusted for your needs.

## 🎨 Customization

### Styling

The application uses CSS custom properties for easy theming:

```css
:root {
    --primary-color: #667eea;
    --secondary-color: #764ba2;
    --background-color: #f7fafc;
    --text-color: #2d3748;
}
```

### Face Detection Parameters

Modify detection settings in the TypeScript files (e.g., `src/base-face-cropper.ts`):

```typescript
const detectionConfig = {
    baseOptions: {
        modelAssetPath: 'path/to/model',
        delegate: "GPU"
    },
    runningMode: "IMAGE"
};
```

After making changes in development mode (`bun run dev`), changes are applied instantly with hot reload. For production, build with:

```bash
bun run build
```

## 📄 License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0). See the [LICENSE](LICENSE) file for details.

This project uses [MediaPipe](https://github.com/google/mediapipe) (Apache-2.0) by Google. See [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

### Development Guidelines

1. **Use TypeScript**: All new code should be written in TypeScript (`.ts` files)
2. **Type Safety**: Maintain strict type checking - no `any` types without good reason
3. **Test with Hot Reload**: Use `bun run dev` for instant feedback during development
4. **Type Check**: Run `bun run type-check` before committing
5. **Test Thoroughly**: Test all three processing modes after changes
6. **Source Control**: Commit `.ts` source files only

### Making Changes

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Start development server (`bun run dev`)
4. Make your changes in the TypeScript files (hot reload applies them instantly)
5. Type check (`bun run type-check`)
6. Test thoroughly across all modes
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

**Note**: Bun runs TypeScript directly in dev mode. Build artifacts in `dist/` are git-ignored and only needed for production deployments.

## 🔗 Links

- [MediaPipe Tasks Vision](https://ai.google.dev/edge/mediapipe/solutions/guide)
- [Face Detection Documentation](https://developers.google.com/mediapipe/solutions/vision/face_detector)
- [Project Repository](https://github.com) <!-- Update with actual repo URL -->

---

Built with ❤️ using MediaPipe Tasks Vision for accurate, client-side face detection.
