class CSVBatchFaceCropper {
    constructor() {
        this.detector = null;
        this.csvData = [];
        this.csvHeaders = [];
        this.filePathColumn = null;
        this.fileNameColumn = null;
        this.csvMapping = new Map(); // filename -> output name
        this.images = new Map(); // imageId -> { file, image, faces, results, selected, processed }
        this.imageResults = new Map();
        this.processingQueue = [];
        this.isProcessing = false;
        this.currentProcessingId = null;
        this.aspectRatioLocked = false;
        this.currentAspectRatio = 1;

        // UI state
        this.currentImageIndex = 0;
        this.currentFaceIndex = 0;
        this.isDarkMode = false;

        // Statistics
        this.statistics = {
            totalFacesDetected: 0,
            imagesProcessed: 0,
            successfulProcessing: 0,
            processingTimes: [],
            startTime: null
        };

        this.processingLog = [];
        this.loadingLog = [];
        this.errorLog = [];

        this.initializeElements();
        this.setupEventListeners();
        this.loadModel();
    }

    initializeElements() {
        // CSV elements
        this.csvInput = document.getElementById('csvInput');
        this.csvMappingDiv = document.getElementById('csvMapping');
        this.filePathColumn = document.getElementById('filePathColumn');
        this.fileNameColumn = document.getElementById('fileNameColumn');
        this.confirmMappingBtn = document.getElementById('confirmMappingBtn');
        this.csvPreviewTable = document.getElementById('csvPreviewTable');
        this.imageUploadCard = document.getElementById('imageUploadCard');
        this.imageInput = document.getElementById('imageInput');

        // Control elements
        this.processAllBtn = document.getElementById('processAllBtn');
        this.processSelectedBtn = document.getElementById('processSelectedBtn');
        this.clearAllBtn = document.getElementById('clearAllBtn');
        this.downloadAllBtn = document.getElementById('downloadAllBtn');

        // Gallery elements
        this.progressSection = document.getElementById('progressSection');
        this.progressFill = document.getElementById('progressFill');
        this.progressText = document.getElementById('progressText');
        this.imageGallery = document.getElementById('imageGallery');
        this.galleryGrid = document.getElementById('galleryGrid');
        this.selectAllBtn = document.getElementById('selectAllBtn');
        this.selectNoneBtn = document.getElementById('selectNoneBtn');
        this.selectedCount = document.getElementById('selectedCount');
        this.totalCount = document.getElementById('totalCount');

        // Canvas elements
        this.canvasContainer = document.getElementById('canvasContainer');
        this.inputCanvas = document.getElementById('inputCanvas');
        this.outputCanvas = document.getElementById('outputCanvas');
        this.croppedFaces = document.getElementById('croppedFaces');
        this.croppedContainer = document.getElementById('croppedContainer');
        this.status = document.getElementById('status');
        this.faceOverlays = document.getElementById('faceOverlays');
        this.faceCount = document.getElementById('faceCount');
        this.selectedFaceCount = document.getElementById('selectedFaceCount');
        this.selectAllFacesBtn = document.getElementById('selectAllFacesBtn');
        this.selectNoneFacesBtn = document.getElementById('selectNoneFacesBtn');
        this.detectFacesBtn = document.getElementById('detectFacesBtn');

        // Settings elements
        this.outputWidth = document.getElementById('outputWidth');
        this.outputHeight = document.getElementById('outputHeight');
        this.faceHeightPct = document.getElementById('faceHeightPct');
        this.positioningMode = document.getElementById('positioningMode');
        this.verticalOffset = document.getElementById('verticalOffset');
        this.horizontalOffset = document.getElementById('horizontalOffset');
        this.aspectRatioLock = document.getElementById('aspectRatioLock');
        this.sizePreset = document.getElementById('sizePreset');
        this.outputFormat = document.getElementById('outputFormat');
        this.jpegQuality = document.getElementById('jpegQuality');
        this.namingTemplate = document.getElementById('namingTemplate');

        // Stats elements
        this.totalRows = document.getElementById('totalRows');
        this.imagesUploaded = document.getElementById('imagesUploaded');
        this.imagesMatched = document.getElementById('imagesMatched');
        this.processingStatus = document.getElementById('processingStatus');
        this.totalFacesDetected = document.getElementById('totalFacesDetected');
        this.successRate = document.getElementById('successRate');
        this.avgProcessingTime = document.getElementById('avgProcessingTime');
        this.imagesProcessed = document.getElementById('imagesProcessed');

        // Log elements
        this.loadingLogElement = document.getElementById('loadingLog');
        this.processingLogElement = document.getElementById('processingLog');
        this.errorLogElement = document.getElementById('errorLog');

        this.updateUI();
    }

    setupEventListeners() {
        // CSV upload and mapping
        this.csvInput.addEventListener('change', (e) => this.handleCSVUpload(e));
        this.filePathColumn.addEventListener('change', () => this.validateMapping());
        this.fileNameColumn.addEventListener('change', () => this.validateMapping());
        this.confirmMappingBtn.addEventListener('click', () => this.confirmMapping());

        // Image upload
        this.imageInput.addEventListener('change', (e) => this.handleMultipleImageUpload(e));
        this.processAllBtn.addEventListener('click', () => this.processAll());
        this.processSelectedBtn.addEventListener('click', () => this.processSelected());
        this.clearAllBtn.addEventListener('click', () => this.clearAll());
        this.downloadAllBtn.addEventListener('click', () => this.downloadAllResults());

        // Gallery controls
        this.selectAllBtn.addEventListener('click', () => this.selectAll());
        this.selectNoneBtn.addEventListener('click', () => this.selectNone());

        // Face selection
        this.selectAllFacesBtn.addEventListener('click', () => this.selectAllFaces());
        this.selectNoneFacesBtn.addEventListener('click', () => this.selectNoneFaces());
        this.detectFacesBtn.addEventListener('click', () => this.detectCurrentImageFaces());

        // Settings
        this.outputWidth.addEventListener('input', () => this.updatePreview());
        this.outputHeight.addEventListener('input', () => this.updatePreview());
        this.faceHeightPct.addEventListener('input', () => this.updatePreview());
        this.positioningMode.addEventListener('change', () => this.updateAdvancedPositioning());
        this.verticalOffset.addEventListener('input', () => this.updateOffsetDisplay('vertical'));
        this.horizontalOffset.addEventListener('input', () => this.updateOffsetDisplay('horizontal'));
        this.aspectRatioLock.addEventListener('click', () => this.toggleAspectRatioLock());
        this.sizePreset.addEventListener('change', () => this.applySizePreset());
        this.outputFormat.addEventListener('change', () => this.updateFormatSettings());

        // Navigation
        const multipleImageModeBtn = document.getElementById('multipleImageModeBtn');
        const singleImageModeBtn = document.getElementById('singleImageModeBtn');
        const darkModeBtn = document.getElementById('darkModeBtn');

        if (multipleImageModeBtn) {
            multipleImageModeBtn.addEventListener('click', () => {
                window.location.href = 'index.html';
            });
        }

        if (singleImageModeBtn) {
            singleImageModeBtn.addEventListener('click', () => {
                window.location.href = 'single.html';
            });
        }

        if (darkModeBtn) {
            darkModeBtn.addEventListener('click', () => this.toggleDarkMode());
        }

        // Enhancement listeners
        const enhancementElements = [
            'exposureAdjustment', 'contrastAdjustment', 'sharpnessControl',
            'skinSmoothing', 'backgroundBlur'
        ];

        enhancementElements.forEach(elementId => {
            const element = document.getElementById(elementId);
            if (element) {
                element.addEventListener('input', () => this.updateSliderValue(elementId.replace('Adjustment', '').replace('Control', '')));
            }
        });

        // Drag and drop for image files
        document.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.stopPropagation();
        });

        document.addEventListener('drop', (e) => {
            e.preventDefault();
            e.stopPropagation();

            if (!this.csvMapping.size) {
                this.updateStatus('Please upload and configure CSV first.');
                return;
            }

            const files = Array.from(e.dataTransfer.files);
            const imageFiles = files.filter(file => file.type.startsWith('image/'));

            if (imageFiles.length > 0) {
                // Simulate file input change
                const dt = new DataTransfer();
                imageFiles.forEach(file => dt.items.add(file));
                this.imageInput.files = dt.files;

                // Trigger the upload handler
                this.handleMultipleImageUpload({ target: { files: imageFiles } });
            }
        });
    }

    async loadModel() {
        try {
            this.updateStatus('Loading face detection model...');
            await tf.ready();
            this.detector = await faceLandmarksDetection.createDetector(
                faceLandmarksDetection.SupportedModels.MediaPipeFaceMesh,
                {
                    runtime: 'tfjs',
                    refineLandmarks: true,
                    maxFaces: 10
                }
            );
            this.updateStatus('Model loaded successfully. Ready to process CSV file.');
            this.addToLoadingLog('Face detection model loaded successfully');
        } catch (error) {
            console.error('Error loading model:', error);
            this.updateStatus('Error loading model. Please refresh the page.');
            this.addToErrorLog('Error loading face detection model: ' + error.message);
        }
    }

    handleCSVUpload(event) {
        const file = event.target.files[0];
        if (!file) return;

        if (!file.name.toLowerCase().endsWith('.csv')) {
            this.updateStatus('Please select a valid CSV file.');
            return;
        }

        const reader = new FileReader();
        reader.onload = (e) => {
            try {
                this.parseCSV(e.target.result);
            } catch (error) {
                console.error('Error parsing CSV:', error);
                this.updateStatus('Error parsing CSV file.');
                this.addToErrorLog('Error parsing CSV: ' + error.message);
            }
        };
        reader.readAsText(file);
    }

    parseCSV(csvText) {
        const lines = csvText.split('\n').filter(line => line.trim());
        if (lines.length < 2) {
            this.updateStatus('CSV file must have at least a header row and one data row.');
            return;
        }

        // Parse headers
        this.csvHeaders = this.parseCSVLine(lines[0]);

        // Parse data rows
        this.csvData = [];
        for (let i = 1; i < lines.length; i++) {
            const row = this.parseCSVLine(lines[i]);
            if (row.length === this.csvHeaders.length) {
                this.csvData.push(row);
            }
        }

        this.populateColumnSelectors();
        this.displayCSVPreview();
        this.csvMappingDiv.classList.remove('hidden');
        this.updateFileStats();
        this.updateStatus(`CSV loaded: ${this.csvData.length} rows found`);
        this.addToLoadingLog(`CSV parsed successfully: ${this.csvData.length} data rows, ${this.csvHeaders.length} columns`);
    }

    parseCSVLine(line) {
        const result = [];
        let current = '';
        let inQuotes = false;

        for (let i = 0; i < line.length; i++) {
            const char = line[i];

            if (char === '"') {
                inQuotes = !inQuotes;
            } else if (char === ',' && !inQuotes) {
                result.push(current.trim());
                current = '';
            } else {
                current += char;
            }
        }

        result.push(current.trim());
        return result;
    }

    populateColumnSelectors() {
        this.filePathColumn.innerHTML = '<option value="">Select column...</option>';
        this.fileNameColumn.innerHTML = '<option value="">Select column...</option>';

        this.csvHeaders.forEach((header, index) => {
            const option1 = new Option(header, index);
            const option2 = new Option(header, index);
            this.filePathColumn.appendChild(option1);
            this.fileNameColumn.appendChild(option2);
        });
    }

    displayCSVPreview() {
        const previewRows = Math.min(5, this.csvData.length);
        let html = '<table class="csv-preview-table"><thead><tr>';

        // Headers
        this.csvHeaders.forEach(header => {
            html += `<th>${this.escapeHtml(header)}</th>`;
        });
        html += '</tr></thead><tbody>';

        // Data rows
        for (let i = 0; i < previewRows; i++) {
            html += '<tr>';
            this.csvData[i].forEach(cell => {
                html += `<td>${this.escapeHtml(cell)}</td>`;
            });
            html += '</tr>';
        }

        html += '</tbody></table>';
        this.csvPreviewTable.innerHTML = html;
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    validateMapping() {
        const hasFilePathColumn = this.filePathColumn.value !== '';
        const hasFileNameColumn = this.fileNameColumn.value !== '';
        const differentColumns = this.filePathColumn.value !== this.fileNameColumn.value;

        this.confirmMappingBtn.disabled = !(hasFilePathColumn && hasFileNameColumn && differentColumns);
    }

    confirmMapping() {
        this.filePathColumn.disabled = true;
        this.fileNameColumn.disabled = true;
        this.confirmMappingBtn.disabled = true;

        // Build filename to output name mapping
        this.buildCSVMapping();

        this.imageUploadCard.classList.remove('hidden');
        this.addToLoadingLog(`Column mapping confirmed: File paths from "${this.csvHeaders[this.filePathColumn.value]}", Names from "${this.csvHeaders[this.fileNameColumn.value]}"`);
        this.updateStatus('Column mapping confirmed. Upload images to process.');
    }

    buildCSVMapping() {
        const filePathColIndex = parseInt(this.filePathColumn.value);
        const fileNameColIndex = parseInt(this.fileNameColumn.value);

        this.csvMapping.clear();

        this.csvData.forEach(row => {
            const filePath = row[filePathColIndex];
            const outputName = row[fileNameColIndex];

            if (filePath && outputName) {
                const filename = this.extractFileName(filePath);
                this.csvMapping.set(filename.toLowerCase(), outputName);
            }
        });

        this.addToLoadingLog(`Built mapping for ${this.csvMapping.size} filename entries`);
    }

    async handleMultipleImageUpload(event) {
        const files = Array.from(event.target.files);
        if (files.length === 0) return;

        this.updateStatus('Filtering images against CSV entries...');
        this.addToLoadingLog(`Processing ${files.length} uploaded files...`);

        let matchedCount = 0;
        let imageId = 0;

        for (const file of files) {
            if (!file.type.startsWith('image/')) continue;

            const filename = file.name.toLowerCase();
            const filenameNoExt = filename.replace(/\.[^/.]+$/, '');

            // Check if this filename exists in our CSV mapping
            let outputName = null;
            if (this.csvMapping.has(filename)) {
                outputName = this.csvMapping.get(filename);
            } else if (this.csvMapping.has(filenameNoExt)) {
                outputName = this.csvMapping.get(filenameNoExt);
            } else {
                // Check for partial matches
                for (const [csvFilename, csvOutputName] of this.csvMapping.entries()) {
                    const csvFilenameNoExt = csvFilename.replace(/\.[^/.]+$/, '');
                    if (csvFilenameNoExt === filenameNoExt) {
                        outputName = csvOutputName;
                        break;
                    }
                }
            }

            if (outputName) {
                try {
                    const image = await this.loadImageFile(file);
                    this.images.set(`img_${imageId}`, {
                        file: file,
                        image: image,
                        faces: [],
                        results: [],
                        selected: true,
                        processed: false,
                        csvOutputName: outputName
                    });
                    matchedCount++;
                    imageId++;
                    this.addToLoadingLog(`✓ Matched: ${file.name} → ${outputName}`);
                } catch (error) {
                    this.addToErrorLog(`Failed to load image ${file.name}: ${error.message}`);
                }
            } else {
                this.addToLoadingLog(`⚠ Skipped: ${file.name} (not found in CSV)`);
            }
        }

        this.updateStatus(`Loaded ${matchedCount} images matching CSV entries`);
        this.addToLoadingLog(`Image filtering completed: ${matchedCount} matched images loaded`);
        this.updateFileStats();
        this.displayImageGallery();

        if (matchedCount > 0) {
            this.processAllBtn.disabled = false;
            this.processSelectedBtn.disabled = false;
        }
    }

    async loadImageFile(file) {
        return new Promise((resolve, reject) => {
            const img = new Image();
            img.onload = () => {
                URL.revokeObjectURL(img.src);
                resolve(img);
            };
            img.onerror = () => {
                URL.revokeObjectURL(img.src);
                reject(new Error(`Failed to load image: ${file.name}`));
            };
            img.src = URL.createObjectURL(file);
        });
    }

    extractFileName(filePath) {
        // Extract filename from various path formats (Windows, Unix, URL)
        return filePath.split(/[\\/]/).pop();
    }


    displayImageGallery() {
        this.galleryGrid.innerHTML = '';

        if (this.images.size === 0) {
            this.imageGallery.classList.add('hidden');
            return;
        }

        this.images.forEach((imageData, imageId) => {
            const galleryItem = document.createElement('div');
            galleryItem.className = 'gallery-item';
            if (imageData.selected) galleryItem.classList.add('selected');

            const canvas = document.createElement('canvas');
            canvas.width = 150;
            canvas.height = 150;
            const ctx = canvas.getContext('2d');

            // Draw thumbnail
            const scale = Math.min(150 / imageData.image.width, 150 / imageData.image.height);
            const width = imageData.image.width * scale;
            const height = imageData.image.height * scale;
            const x = (150 - width) / 2;
            const y = (150 - height) / 2;

            ctx.fillStyle = '#f0f0f0';
            ctx.fillRect(0, 0, 150, 150);
            ctx.drawImage(imageData.image, x, y, width, height);

            const checkbox = document.createElement('input');
            checkbox.type = 'checkbox';
            checkbox.checked = imageData.selected;
            checkbox.addEventListener('change', () => this.toggleImageSelection(imageId));

            const label = document.createElement('div');
            label.className = 'gallery-label';
            label.textContent = imageData.csvOutputName || imageData.file.name;

            const status = document.createElement('div');
            status.className = 'gallery-status';
            status.textContent = imageData.processed ? 'Processed' : 'Pending';

            galleryItem.appendChild(canvas);
            galleryItem.appendChild(checkbox);
            galleryItem.appendChild(label);
            galleryItem.appendChild(status);

            galleryItem.addEventListener('click', (e) => {
                if (e.target !== checkbox) {
                    this.selectImage(imageId);
                }
            });

            this.galleryGrid.appendChild(galleryItem);
        });

        this.imageGallery.classList.remove('hidden');
        this.updateSelectionCount();
    }

    selectImage(imageId) {
        const imageData = this.images.get(imageId);
        if (!imageData) return;

        this.displayImageInCanvas(imageData);
        this.canvasContainer.classList.remove('hidden');
    }

    displayImageInCanvas(imageData) {
        const canvas = this.inputCanvas;
        const ctx = canvas.getContext('2d');

        // Set canvas size
        const maxWidth = 600;
        const maxHeight = 400;
        const scale = Math.min(maxWidth / imageData.image.width, maxHeight / imageData.image.height);

        canvas.width = imageData.image.width * scale;
        canvas.height = imageData.image.height * scale;

        // Draw image
        ctx.drawImage(imageData.image, 0, 0, canvas.width, canvas.height);

        // Display faces if already detected
        this.updateFaceOverlays(imageData);
    }

    toggleImageSelection(imageId) {
        const imageData = this.images.get(imageId);
        if (imageData) {
            imageData.selected = !imageData.selected;
            this.displayImageGallery(); // Refresh display
        }
    }

    selectAll() {
        this.images.forEach(imageData => {
            imageData.selected = true;
        });
        this.displayImageGallery();
    }

    selectNone() {
        this.images.forEach(imageData => {
            imageData.selected = false;
        });
        this.displayImageGallery();
    }

    updateSelectionCount() {
        const selected = Array.from(this.images.values()).filter(img => img.selected).length;
        this.selectedCount.textContent = selected;
        this.totalCount.textContent = this.images.size;
    }

    async processAll() {
        const selectedImages = Array.from(this.images.entries()).filter(([id, data]) => data.selected);
        if (selectedImages.length === 0) {
            this.updateStatus('No images selected for processing.');
            return;
        }

        await this.processImages(selectedImages);
    }

    async processSelected() {
        await this.processAll(); // Same as processAll since we're filtering by selected
    }

    async processImages(imageEntries) {
        this.isProcessing = true;
        this.updateStatus('Processing images...');
        this.addToProcessingLog(`Starting batch processing of ${imageEntries.length} images...`);
        this.showProgress();

        let successCount = 0;
        const processingTimes = [];

        for (let i = 0; i < imageEntries.length; i++) {
            const [imageId, imageData] = imageEntries[i];

            this.updateProgress((i / imageEntries.length) * 100, `Processing ${i + 1}/${imageEntries.length}: ${imageData.csvFileName}`);

            try {
                const startTime = Date.now();
                const success = await this.processImage(imageId, imageData);
                const processingTime = Date.now() - startTime;

                if (success) {
                    successCount++;
                    processingTimes.push(processingTime);
                    this.addToProcessingLog(`✓ Processed ${imageData.csvFileName} in ${processingTime}ms`);
                } else {
                    this.addToErrorLog(`✗ Failed to process ${imageData.csvFileName}`);
                }
            } catch (error) {
                this.addToErrorLog(`✗ Error processing ${imageData.csvFileName}: ${error.message}`);
            }
        }

        this.updateProgress(100, 'Processing complete!');
        this.hideProgress();

        // Update statistics
        this.statistics.imagesProcessed += successCount;
        this.statistics.successfulProcessing += successCount;
        this.statistics.processingTimes.push(...processingTimes);

        const avgTime = processingTimes.length > 0 ?
            Math.round(processingTimes.reduce((a, b) => a + b, 0) / processingTimes.length) : 0;

        this.updateStatus(`Processing complete: ${successCount}/${imageEntries.length} images processed successfully`);
        this.addToProcessingLog(`Batch processing completed: ${successCount}/${imageEntries.length} successful, avg time: ${avgTime}ms`);

        this.updateStatistics();
        this.isProcessing = false;

        if (successCount > 0) {
            this.downloadAllBtn.disabled = false;
        }
    }

    async processImage(imageId, imageData) {
        try {
            // Detect faces if not already done
            if (imageData.faces.length === 0) {
                const faces = await this.detector.estimateFaces(imageData.image);
                imageData.faces = faces.map((face, index) => ({
                    ...face,
                    id: index,
                    selected: true
                }));
                this.statistics.totalFacesDetected += faces.length;
            }

            if (imageData.faces.length === 0) {
                this.addToErrorLog(`No faces detected in ${imageData.csvFileName}`);
                return false;
            }

            // Process selected faces
            const selectedFaces = imageData.faces.filter(face => face.selected);
            const results = [];

            for (const face of selectedFaces) {
                const croppedImage = await this.cropFace(imageData.image, face);
                results.push({
                    face,
                    image: croppedImage,
                    fileName: imageData.csvFileName
                });
            }

            imageData.results = results;
            imageData.processed = true;
            return true;

        } catch (error) {
            console.error('Error processing image:', error);
            return false;
        }
    }

    async cropFace(image, face) {
        const settings = this.getSettings();
        const box = face.box;

        // Calculate crop dimensions (same logic as other files)
        const faceHeight = settings.outputHeight * (settings.faceHeightPct / 100);
        const scale = faceHeight / box.height;

        let cropWidth = settings.outputWidth / scale;
        let cropHeight = settings.outputHeight / scale;

        let centerX = box.xMin + (box.width / 2);
        let centerY = box.yMin + (box.height / 2);

        if (settings.positioningMode === 'rule-of-thirds') {
            centerY = box.yMin + (box.height * 0.33);
        } else if (settings.positioningMode === 'custom') {
            centerX += (settings.horizontalOffset / 100) * cropWidth;
            centerY += (settings.verticalOffset / 100) * cropHeight;
        }

        let cropX = centerX - (cropWidth / 2);
        let cropY = centerY - (cropHeight / 2);

        cropX = Math.max(0, Math.min(cropX, image.width - cropWidth));
        cropY = Math.max(0, Math.min(cropY, image.height - cropHeight));

        const canvas = document.createElement('canvas');
        canvas.width = settings.outputWidth;
        canvas.height = settings.outputHeight;
        const ctx = canvas.getContext('2d');

        ctx.drawImage(
            image,
            cropX, cropY, cropWidth, cropHeight,
            0, 0, settings.outputWidth, settings.outputHeight
        );

        return canvas;
    }

    async downloadAllResults() {
        const processedImages = Array.from(this.images.values()).filter(img => img.processed && img.results.length > 0);

        if (processedImages.length === 0) {
            this.updateStatus('No processed results to download.');
            return;
        }

        const zip = new JSZip();
        const settings = this.getSettings();

        for (const imageData of processedImages) {
            for (let i = 0; i < imageData.results.length; i++) {
                const result = imageData.results[i];
                const filename = this.generateFilename(imageData, i);

                const blob = await new Promise(resolve => {
                    result.image.toBlob(resolve, `image/${settings.format}`, settings.quality);
                });

                zip.file(filename, blob);
            }
        }

        const zipBlob = await zip.generateAsync({ type: 'blob' });
        const url = URL.createObjectURL(zipBlob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'csv_batch_cropped_faces.zip';
        a.click();
        URL.revokeObjectURL(url);

        this.addToProcessingLog(`Downloaded ${processedImages.length} processed images as ZIP`);
    }

    generateFilename(imageData, faceIndex) {
        const settings = this.getSettings();
        const template = this.namingTemplate.value || '{csv_name}';
        const extension = settings.format === 'jpeg' ? 'jpg' : settings.format;
        const originalName = imageData.file ? imageData.file.name.replace(/\.[^/.]+$/, '') : 'image';

        return template
            .replace('{csv_name}', imageData.csvOutputName || originalName)
            .replace('{original}', originalName)
            .replace('{index}', faceIndex + 1)
            .replace('{timestamp}', Date.now())
            .replace('{width}', settings.outputWidth)
            .replace('{height}', settings.outputHeight) + '.' + extension;
    }

    getSettings() {
        return {
            outputWidth: parseInt(this.outputWidth.value),
            outputHeight: parseInt(this.outputHeight.value),
            faceHeightPct: parseInt(this.faceHeightPct.value),
            positioningMode: this.positioningMode.value,
            verticalOffset: parseInt(this.verticalOffset.value),
            horizontalOffset: parseInt(this.horizontalOffset.value),
            format: this.outputFormat.value,
            quality: this.outputFormat.value === 'jpeg' ? (parseInt(this.jpegQuality.value) / 100) : 1
        };
    }

    clearAll() {
        this.images.clear();
        this.csvData = [];
        this.csvHeaders = [];
        this.filePathColumn = null;
        this.fileNameColumn = null;
        this.csvInput.value = '';

        this.csvMappingDiv.classList.add('hidden');
        this.imageUploadCard.classList.add('hidden');
        this.imageGallery.classList.add('hidden');
        this.canvasContainer.classList.add('hidden');
        this.croppedFaces.classList.add('hidden');

        this.galleryGrid.innerHTML = '';
        this.croppedContainer.innerHTML = '';
        this.csvPreviewTable.innerHTML = '';

        this.resetStatistics();
        this.updateUI();
        this.updateStatus('Ready to load CSV file');
        this.addToLoadingLog('All data cleared');
    }

    updateUI() {
        const hasCSVData = this.csvData.length > 0;
        const hasMappedColumns = this.filePathColumn?.value && this.fileNameColumn?.value;
        const hasImages = this.images.size > 0;
        const hasSelectedImages = Array.from(this.images.values()).some(img => img.selected);

        this.processAllBtn.disabled = !hasImages || !hasSelectedImages;
        this.processSelectedBtn.disabled = !hasImages || !hasSelectedImages;
        this.clearAllBtn.disabled = !hasCSVData && !hasImages;
        this.downloadAllBtn.disabled = true; // Enabled after processing
    }

    updateFileStats() {
        this.totalRows.textContent = this.csvData.length;
        this.imagesUploaded.textContent = this.imageInput?.files?.length || 0;
        this.imagesMatched.textContent = this.images.size;
    }

    updateStatistics() {
        this.totalFacesDetected.textContent = this.statistics.totalFacesDetected;
        this.imagesProcessed.textContent = this.statistics.imagesProcessed;

        const successRate = this.statistics.imagesProcessed > 0 ?
            Math.round((this.statistics.successfulProcessing / this.statistics.imagesProcessed) * 100) : 0;
        this.successRate.textContent = successRate + '%';

        const avgTime = this.statistics.processingTimes.length > 0 ?
            Math.round(this.statistics.processingTimes.reduce((a, b) => a + b, 0) / this.statistics.processingTimes.length) : 0;
        this.avgProcessingTime.textContent = avgTime + 'ms';
    }

    resetStatistics() {
        this.statistics = {
            totalFacesDetected: 0,
            imagesProcessed: 0,
            successfulProcessing: 0,
            processingTimes: [],
            startTime: null
        };
        this.updateStatistics();
        this.updateFileStats();
    }

    // Utility methods
    updateStatus(message) {
        this.status.textContent = message;
        this.processingStatus.textContent = message.split('.')[0];
    }

    addToLoadingLog(message) {
        this.addLogEntry(this.loadingLogElement, message);
    }

    addToProcessingLog(message) {
        this.addLogEntry(this.processingLogElement, message);
    }

    addToErrorLog(message) {
        this.addLogEntry(this.errorLogElement, message, 'error');
    }

    addLogEntry(logElement, message, type = 'info') {
        const logEntry = document.createElement('div');
        logEntry.className = `log-entry ${type}`;
        logEntry.textContent = `${new Date().toLocaleTimeString()}: ${message}`;
        logElement.appendChild(logEntry);
        logElement.scrollTop = logElement.scrollHeight;
    }

    showProgress() {
        this.progressSection.classList.remove('hidden');
    }

    hideProgress() {
        setTimeout(() => {
            this.progressSection.classList.add('hidden');
        }, 1000);
    }

    updateProgress(percent, text) {
        this.progressFill.style.width = percent + '%';
        this.progressText.textContent = text;
    }

    // Settings methods (similar to other files)
    updatePreview() {
        const width = this.outputWidth.value;
        const height = this.outputHeight.value;
        const faceHeight = this.faceHeightPct.value;
        const format = this.outputFormat.value.toUpperCase();

        document.getElementById('previewText').textContent =
            `${width}×${height}px, face at ${faceHeight}% height, ${format} format`;

        const ratio = width / height;
        document.getElementById('aspectRatioText').textContent =
            `${ratio.toFixed(2)}:1 ratio`;

        if (this.aspectRatioLocked) {
            this.maintainAspectRatio();
        }
    }

    updateAdvancedPositioning() {
        const mode = this.positioningMode.value;
        const advancedPositioning = document.getElementById('advancedPositioning');

        if (mode === 'custom') {
            advancedPositioning.style.display = 'block';
        } else {
            advancedPositioning.style.display = 'none';
        }
    }

    updateOffsetDisplay(type) {
        const value = type === 'vertical' ? this.verticalOffset.value : this.horizontalOffset.value;
        const displayElement = document.getElementById(`${type}OffsetValue`);
        if (displayElement) {
            displayElement.textContent = value + '%';
        }
    }

    toggleAspectRatioLock() {
        this.aspectRatioLocked = !this.aspectRatioLocked;
        this.aspectRatioLock.textContent = this.aspectRatioLocked ? '🔒' : '🔓';

        if (this.aspectRatioLocked) {
            this.currentAspectRatio = this.outputWidth.value / this.outputHeight.value;
        }
    }

    maintainAspectRatio() {
        if (!this.aspectRatioLocked) return;

        const currentRatio = this.outputWidth.value / this.outputHeight.value;
        if (Math.abs(currentRatio - this.currentAspectRatio) > 0.01) {
            this.outputHeight.value = Math.round(this.outputWidth.value / this.currentAspectRatio);
        }
    }

    applySizePreset() {
        const preset = this.sizePreset.value;
        const presets = {
            linkedin: { width: 400, height: 400 },
            passport: { width: 413, height: 531 },
            instagram: { width: 1080, height: 1080 },
            idcard: { width: 332, height: 498 },
            avatar: { width: 512, height: 512 },
            headshot: { width: 600, height: 800 }
        };

        if (presets[preset]) {
            this.outputWidth.value = presets[preset].width;
            this.outputHeight.value = presets[preset].height;
            this.updatePreview();
        }
    }

    updateFormatSettings() {
        const format = this.outputFormat.value;
        const jpegQualityGroup = document.getElementById('jpegQualityGroup');

        if (format === 'jpeg') {
            jpegQualityGroup.classList.remove('hidden');
        } else {
            jpegQualityGroup.classList.add('hidden');
        }

        this.updatePreview();
    }

    updateSliderValue(type) {
        const sliderMap = {
            exposure: { element: this.exposureAdjustment, display: 'exposureValue' },
            contrast: { element: this.contrastAdjustment, display: 'contrastValue' },
            sharpness: { element: this.sharpnessControl, display: 'sharpnessValue' },
            skinSmoothing: { element: this.skinSmoothing, display: 'skinSmoothingValue' },
            backgroundBlur: { element: this.backgroundBlur, display: 'backgroundBlurValue' }
        };

        const config = sliderMap[type];
        if (config && config.element) {
            const value = config.element.value;
            const displayElement = document.getElementById(config.display);
            if (displayElement) {
                if (type === 'backgroundBlur') {
                    displayElement.textContent = value + 'px';
                } else {
                    displayElement.textContent = value;
                }
            }
        }
    }

    toggleDarkMode() {
        this.isDarkMode = !this.isDarkMode;
        document.body.classList.toggle('dark-mode', this.isDarkMode);

        const darkModeBtn = document.getElementById('darkModeBtn');
        if (darkModeBtn) {
            darkModeBtn.textContent = this.isDarkMode ? '☀️' : '🌙';
            darkModeBtn.setAttribute('aria-pressed', this.isDarkMode.toString());
        }
    }

    // Face overlay methods (simplified versions)
    updateFaceOverlays(imageData) {
        // This would be implemented similar to other files
        // For brevity, keeping it simple here
    }

    selectAllFaces() {
        // Implementation for selecting all faces in current image
    }

    selectNoneFaces() {
        // Implementation for deselecting all faces in current image
    }

    detectCurrentImageFaces() {
        // Implementation for detecting faces in currently displayed image
    }
}

// Initialize the application when the page loads
document.addEventListener('DOMContentLoaded', () => {
    new CSVBatchFaceCropper();
});