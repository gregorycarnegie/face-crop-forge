# Face Cropping Tool

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

### Option 1: Direct File Access

1. Download or clone this repository
2. Open `home.html` in any modern web browser
3. Choose your processing mode and start uploading images

### Option 2: Local Server (Recommended)

```bash
# Using Node.js (if server.js is present)
node server.js

# Using Python
python -m http.server 8000

# Using PHP
php -S localhost:8000
```

Then navigate to `http://localhost:8000`

## 📖 Usage Guide

### Single Image Mode (`single.html`)

1. **Upload**: Drag & drop or click to select an image
2. **Detect**: Click "Detect Faces" to analyze the image
3. **Adjust**: Use quality filters and color correction as needed
4. **Download**: Get individual crops or a complete ZIP package

### Multiple Images Mode (`index.html`)

1. **Batch Upload**: Add multiple images to the gallery
2. **Process**: Run face detection across all images
3. **Review**: Use the gallery to navigate and review results
4. **Export**: Download crops individually or as a batch ZIP

### CSV Batch Mode (`csv-batch.html`)

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

- **MediaPipe Tasks Vision**: Google's state-of-the-art face detection
- **Web Workers**: Background processing for smooth UI performance
- **Canvas API**: High-performance image manipulation
- **Modern ES6+**: Clean, maintainable JavaScript

### Key Components

- `home.html` - Mode selection landing page
- `single.html` - Single image processing interface
- `index.html` - Multiple image batch processing
- `csv-batch.html` - CSV-driven batch operations
- `face-detection-worker.js` - Web Worker for face detection
- `script.js` - Main application logic for multiple images
- `single-script.js` - Single image processing logic
- `csv-batch-script.js` - CSV batch processing logic

### Performance Features

- **Lazy Loading**: Images loaded on-demand for better performance
- **Progressive Processing**: Non-blocking face detection
- **Memory Management**: Efficient handling of large image batches
- **Canvas Optimization**: `willReadFrequently` attribute for better performance

## 🌐 Browser Compatibility

### Minimum Requirements

- **Chrome/Edge**: 88+ (recommended for best performance)
- **Firefox**: 85+
- **Safari**: 14+

### Required Features

- ES6+ modules support
- Canvas 2D context
- Web Workers
- File API
- WebAssembly (for MediaPipe)

## 🔧 Development

### Project Structure

```text
├── home.html              # Mode selection page
├── index.html             # Multiple images interface
├── single.html            # Single image interface
├── csv-batch.html         # CSV batch interface
├── script.js              # Main application logic
├── single-script.js       # Single image logic
├── csv-batch-script.js    # CSV batch logic
├── face-detection-worker.js # Web Worker for face detection
├── styles.css             # Global styles
└── server.js              # Optional local server
```

### Key Files

- **HTML Pages**: User interfaces for different processing modes
- **JavaScript Modules**: Core application logic and face detection
- **Web Worker**: Background face detection processing
- **Styles**: Responsive CSS with dark mode support

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

Modify detection settings in the JavaScript files:

```javascript
const detectionConfig = {
    baseOptions: {
        modelAssetPath: 'path/to/model',
        delegate: "GPU"
    },
    runningMode: "IMAGE"
};
```

## 📄 License

This project is open source. See the repository for license details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## 🔗 Links

- [MediaPipe Tasks Vision](https://ai.google.dev/edge/mediapipe/solutions/guide)
- [Face Detection Documentation](https://developers.google.com/mediapipe/solutions/vision/face_detector)
- [Project Repository](https://github.com) <!-- Update with actual repo URL -->

---

Built with ❤️ using MediaPipe Tasks Vision for accurate, client-side face detection.
