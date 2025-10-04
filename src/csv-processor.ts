import { BaseFaceCropper } from './base-face-cropper.js';
import type {
    ImageData,
    FaceData,
    CropResult,
    Statistics,
    BoundingBox
} from './types.js';

interface CSVImageData extends ImageData {
    csvOutputName: string;
}

interface MatchedFile {
    file: File;
    outputName: string;
}

interface ImageLoadQueueItem {
    files: MatchedFile[];
    page: number;
}

interface StreamedResult {
    filename: string;
    csvOutputName: string;
    results: CropResult[];
    processedAt: string;
}

class CSVBatchFaceCropper extends BaseFaceCropper {
    // CSV data properties
    private csvData: string[][] = [];
    private csvHeaders: string[] = [];
    private csvMapping: Map<string, string> = new Map();

    // Image storage
    protected images: Map<string, CSVImageData> = new Map();
    private imageResults: Map<string, StreamedResult> = new Map();

    // Processing state
    private processingQueue: string[] = [];
    private isProcessing: boolean = false;
    private currentProcessingId: string | null = null;

    // UI state - inherited from BaseFaceCropper
    // currentImageIndex and currentFaceIndex are now in base class

    // Logs
    protected processingLog: string[] = [];
    protected loadingLog: string[] = [];
    protected errorLog: string[] = [];

    // Lazy loading state
    protected galleryPage: number = 0;
    protected galleryPageSize: number = 20;
    protected imageLoadQueue: ImageLoadQueueItem[] = [];
    protected isLoadingImages: boolean = false;
    private lazyLoadingObserver: IntersectionObserver | null = null;
    private lazyLoadingIndicator: HTMLElement | null = null;

    // CSV UI elements
    private csvInput!: HTMLInputElement;
    private csvMappingDiv!: HTMLElement;
    private filePathColumn!: HTMLSelectElement;
    private fileNameColumn!: HTMLSelectElement;
    private confirmMappingBtn!: HTMLButtonElement;
    private csvPreviewTable!: HTMLElement;
    private imageUploadCard!: HTMLElement;
    private imageInput!: HTMLInputElement;

    // Control elements
    protected processAllBtn!: HTMLButtonElement;
    protected processSelectedBtn!: HTMLButtonElement;
    protected clearAllBtn!: HTMLButtonElement;
    protected downloadAllBtn!: HTMLButtonElement;

    // Gallery elements
    protected progressSection!: HTMLElement;
    protected progressFill!: HTMLElement;
    protected progressText!: HTMLElement;
    private imageGallery!: HTMLElement;
    protected galleryGrid!: HTMLElement;
    private selectAllBtn!: HTMLButtonElement;
    private selectNoneBtn!: HTMLButtonElement;
    protected selectedCount!: HTMLElement;
    protected totalCount!: HTMLElement;

    // Canvas elements
    private canvasContainer!: HTMLElement;
    protected inputCanvas!: HTMLCanvasElement;
    private outputCanvas!: HTMLCanvasElement;
    private croppedFaces!: HTMLElement;
    private croppedContainer!: HTMLElement;
    protected status!: HTMLElement;
    protected faceOverlays!: HTMLElement;
    protected faceCount!: HTMLElement;
    protected selectedFaceCount!: HTMLElement;
    private selectAllFacesBtn!: HTMLButtonElement;
    private selectNoneFacesBtn!: HTMLButtonElement;
    private detectFacesBtn!: HTMLButtonElement;

    // Settings elements
    protected outputWidth!: HTMLInputElement;
    protected outputHeight!: HTMLInputElement;
    protected faceHeightPct!: HTMLInputElement;
    protected positioningMode!: HTMLSelectElement;
    protected verticalOffset!: HTMLInputElement;
    protected horizontalOffset!: HTMLInputElement;
    protected aspectRatioLock!: HTMLButtonElement;
    protected sizePreset!: HTMLSelectElement;
    protected outputFormat!: HTMLSelectElement;
    protected jpegQuality!: HTMLInputElement;
    protected namingTemplate!: HTMLInputElement;

    // Stats elements
    private totalRows!: HTMLElement;
    private imagesUploaded!: HTMLElement;
    private imagesMatched!: HTMLElement;
    protected processingStatus!: HTMLElement;
    protected totalFacesDetected!: HTMLElement;
    protected successRate!: HTMLElement;
    protected avgProcessingTime!: HTMLElement;
    protected imagesProcessed!: HTMLElement;

    // Log elements
    protected loadingLogElement!: HTMLElement;
    protected processingLogElement!: HTMLElement;
    protected errorLogElement!: HTMLElement;

    constructor() {
        super();
        this.initializeElements();
        this.setupEventListeners();
        this.setupDragAndDrop();
        this.loadModel();
    }

    private initializeElements(): void {
        // CSV elements
        this.csvInput = document.getElementById('csvInput') as HTMLInputElement;
        this.csvMappingDiv = document.getElementById('csvMapping') as HTMLElement;
        this.filePathColumn = document.getElementById('filePathColumn') as HTMLSelectElement;
        this.fileNameColumn = document.getElementById('fileNameColumn') as HTMLSelectElement;
        this.confirmMappingBtn = document.getElementById('confirmMappingBtn') as HTMLButtonElement;
        this.csvPreviewTable = document.getElementById('csvPreviewTable') as HTMLElement;
        this.imageUploadCard = document.getElementById('imageUploadCard') as HTMLElement;
        this.imageInput = document.getElementById('imageInput') as HTMLInputElement;

        // Control elements
        this.processAllBtn = document.getElementById('processAllBtn') as HTMLButtonElement;
        this.processSelectedBtn = document.getElementById('processSelectedBtn') as HTMLButtonElement;
        this.clearAllBtn = document.getElementById('clearAllBtn') as HTMLButtonElement;
        this.downloadAllBtn = document.getElementById('downloadAllBtn') as HTMLButtonElement;

        // Gallery elements
        this.progressSection = document.getElementById('progressSection') as HTMLElement;
        this.progressFill = document.getElementById('progressFill') as HTMLElement;
        this.progressText = document.getElementById('progressText') as HTMLElement;
        this.imageGallery = document.getElementById('imageGallery') as HTMLElement;
        this.galleryGrid = document.getElementById('galleryGrid') as HTMLElement;
        this.selectAllBtn = document.getElementById('selectAllBtn') as HTMLButtonElement;
        this.selectNoneBtn = document.getElementById('selectNoneBtn') as HTMLButtonElement;
        this.selectedCount = document.getElementById('selectedCount') as HTMLElement;
        this.totalCount = document.getElementById('totalCount') as HTMLElement;

        // Canvas elements
        this.canvasContainer = document.getElementById('canvasContainer') as HTMLElement;
        this.inputCanvas = document.getElementById('inputCanvas') as HTMLCanvasElement;
        this.outputCanvas = document.getElementById('outputCanvas') as HTMLCanvasElement;
        this.croppedFaces = document.getElementById('croppedFaces') as HTMLElement;
        this.croppedContainer = document.getElementById('croppedContainer') as HTMLElement;
        this.status = document.getElementById('status') as HTMLElement;
        this.faceOverlays = document.getElementById('faceOverlays') as HTMLElement;
        this.faceCount = document.getElementById('faceCount') as HTMLElement;
        this.selectedFaceCount = document.getElementById('selectedFaceCount') as HTMLElement;
        this.selectAllFacesBtn = document.getElementById('selectAllFacesBtn') as HTMLButtonElement;
        this.selectNoneFacesBtn = document.getElementById('selectNoneFacesBtn') as HTMLButtonElement;
        this.detectFacesBtn = document.getElementById('detectFacesBtn') as HTMLButtonElement;

        // Settings elements
        this.outputWidth = document.getElementById('outputWidth') as HTMLInputElement;
        this.outputHeight = document.getElementById('outputHeight') as HTMLInputElement;
        this.faceHeightPct = document.getElementById('faceHeightPct') as HTMLInputElement;
        this.positioningMode = document.getElementById('positioningMode') as HTMLSelectElement;
        this.verticalOffset = document.getElementById('verticalOffset') as HTMLInputElement;
        this.horizontalOffset = document.getElementById('horizontalOffset') as HTMLInputElement;
        this.aspectRatioLock = document.getElementById('aspectRatioLock') as HTMLButtonElement;
        this.sizePreset = document.getElementById('sizePreset') as HTMLSelectElement;
        this.outputFormat = document.getElementById('outputFormat') as HTMLSelectElement;
        this.jpegQuality = document.getElementById('jpegQuality') as HTMLInputElement;
        this.namingTemplate = document.getElementById('namingTemplate') as HTMLInputElement;

        // Stats elements
        this.totalRows = document.getElementById('totalRows') as HTMLElement;
        this.imagesUploaded = document.getElementById('imagesUploaded') as HTMLElement;
        this.imagesMatched = document.getElementById('imagesMatched') as HTMLElement;
        this.processingStatus = document.getElementById('processingStatus') as HTMLElement;
        this.totalFacesDetected = document.getElementById('totalFacesDetected') as HTMLElement;
        this.successRate = document.getElementById('successRate') as HTMLElement;
        this.avgProcessingTime = document.getElementById('avgProcessingTime') as HTMLElement;
        this.imagesProcessed = document.getElementById('imagesProcessed') as HTMLElement;

        // Log elements
        this.loadingLogElement = document.getElementById('loadingLog') as HTMLElement;
        this.processingLogElement = document.getElementById('processingLog') as HTMLElement;
        this.errorLogElement = document.getElementById('errorLog') as HTMLElement;

        this.updateUI();
    }

    private setupEventListeners(): void {
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
                window.location.href = 'batch-processing.html';
            });
        }

        if (singleImageModeBtn) {
            singleImageModeBtn.addEventListener('click', () => {
                window.location.href = 'single-processing.html';
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
                element.addEventListener('input', () => this.updateSliderValue(elementId.replace('Adjustment', '').replace('Control', '') as 'exposure' | 'contrast' | 'sharpness' | 'skinSmoothing' | 'backgroundBlur'));
            }
        });
    }

    setupDragAndDrop(): void {
        // Setup drag and drop for CSV upload card
        const csvUploadCard = this.csvInput.closest('.upload-card');
        if (csvUploadCard) {
            this.setupDropZone(csvUploadCard as HTMLElement, (files) => this.handleCSVDrop(files));
        }

        // Setup drag and drop for image upload card
        if (this.imageUploadCard) {
            this.setupDropZone(this.imageUploadCard, (files) => this.handleImageDrop(files));
        }
    }

    private setupDropZone(element: HTMLElement, dropHandler: (files: FileList) => void): void {
        ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
            element.addEventListener(eventName, (e) => this.preventDefaults(e), false);
        });

        ['dragenter', 'dragover'].forEach(eventName => {
            element.addEventListener(eventName, () => {
                element.classList.add('drag-over');
            }, false);
        });

        ['dragleave', 'drop'].forEach(eventName => {
            element.addEventListener(eventName, () => {
                element.classList.remove('drag-over');
            }, false);
        });

        element.addEventListener('drop', (e: DragEvent) => {
            if (e.dataTransfer?.files) {
                dropHandler(e.dataTransfer.files);
            }
        }, false);
    }

    private preventDefaults(e: Event): void {
        e.preventDefault();
        e.stopPropagation();
    }

    private handleCSVDrop(files: FileList): void {
        const csvFiles = Array.from(files).filter(file => file.name.toLowerCase().endsWith('.csv'));

        if (csvFiles.length === 0) {
            this.updateStatus('Please drop a CSV file');
            return;
        }

        if (csvFiles.length > 1) {
            this.updateStatus('Please drop only one CSV file at a time');
            return;
        }

        // Simulate file input
        const dt = new DataTransfer();
        dt.items.add(csvFiles[0]);
        this.csvInput.files = dt.files;

        // Trigger the upload handler
        this.handleCSVUpload({ target: { files: csvFiles } } as unknown as Event);
    }

    private handleImageDrop(files: FileList): void {
        if (!this.csvMapping.size) {
            this.updateStatus('Please upload and configure CSV first.');
            return;
        }

        const imageFiles = Array.from(files).filter(file => file.type.startsWith('image/'));

        if (imageFiles.length === 0) {
            this.updateStatus('Please drop image files');
            return;
        }

        // Simulate file input
        const dt = new DataTransfer();
        imageFiles.forEach(file => dt.items.add(file));
        this.imageInput.files = dt.files;

        // Trigger the upload handler
        this.handleMultipleImageUpload({ target: { files: imageFiles } } as unknown as Event);
    }

    private handleCSVUpload(event: Event): void {
        const target = event.target as HTMLInputElement;
        const file = target.files?.[0];
        if (!file) return;

        if (!file.name.toLowerCase().endsWith('.csv')) {
            this.updateStatus('Please select a valid CSV file.');
            return;
        }

        const reader = new FileReader();
        reader.onload = (e: ProgressEvent<FileReader>) => {
            try {
                this.parseCSV(e.target?.result as string);
            } catch (error) {
                console.error('Error parsing CSV:', error);
                this.updateStatus('Error parsing CSV file.');
                this.addToErrorLog('Error parsing CSV: ' + (error as Error).message);
            }
        };
        reader.readAsText(file);
    }

    private parseCSV(csvText: string): void {
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

    private parseCSVLine(line: string): string[] {
        const result: string[] = [];
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

    private populateColumnSelectors(): void {
        this.filePathColumn.innerHTML = '<option value="">Select column...</option>';
        this.fileNameColumn.innerHTML = '<option value="">Select column...</option>';

        this.csvHeaders.forEach((header, index) => {
            const option1 = new Option(header, index.toString());
            const option2 = new Option(header, index.toString());
            this.filePathColumn.appendChild(option1);
            this.fileNameColumn.appendChild(option2);
        });
    }

    private displayCSVPreview(): void {
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

    private validateMapping(): void {
        const hasFilePathColumn = this.filePathColumn.value !== '';
        const hasFileNameColumn = this.fileNameColumn.value !== '';
        const differentColumns = this.filePathColumn.value !== this.fileNameColumn.value;

        this.confirmMappingBtn.disabled = !(hasFilePathColumn && hasFileNameColumn && differentColumns);
    }

    private confirmMapping(): void {
        this.filePathColumn.disabled = true;
        this.fileNameColumn.disabled = true;
        this.confirmMappingBtn.disabled = true;

        // Build filename to output name mapping
        this.buildCSVMapping();

        this.imageUploadCard.classList.remove('hidden');
        this.addToLoadingLog(`Column mapping confirmed: File paths from "${this.csvHeaders[parseInt(this.filePathColumn.value)]}", Names from "${this.csvHeaders[parseInt(this.fileNameColumn.value)]}"`);
        this.updateStatus('Column mapping confirmed. Upload images to process.');
    }

    private buildCSVMapping(): void {
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

    private async handleMultipleImageUpload(event: Event): Promise<void> {
        const target = event.target as HTMLInputElement;
        const files = Array.from(target.files || []);
        if (files.length === 0) return;

        this.updateStatus('Filtering images against CSV entries...');
        this.addToLoadingLog(`Processing ${files.length} uploaded files...`);

        // First, filter files to only those matching CSV entries
        const matchedFiles: MatchedFile[] = [];
        for (const file of files) {
            if (!file.type.startsWith('image/')) continue;

            const filename = file.name.toLowerCase();
            const filenameNoExt = filename.replace(/\.[^/.]+$/, '');

            // Check if this filename exists in our CSV mapping
            let outputName: string | null = null;
            if (this.csvMapping.has(filename)) {
                outputName = this.csvMapping.get(filename)!;
            } else if (this.csvMapping.has(filenameNoExt)) {
                outputName = this.csvMapping.get(filenameNoExt)!;
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
                matchedFiles.push({ file, outputName });
                this.addToLoadingLog(`✓ Matched: ${file.name} → ${outputName}`);
            } else {
                this.addToLoadingLog(`⚠ Skipped: ${file.name} (not found in CSV)`);
            }
        }

        if (matchedFiles.length === 0) {
            this.updateStatus('No uploaded images matched CSV entries');
            return;
        }

        // Use lazy loading for large batches
        if (matchedFiles.length > this.galleryPageSize) {
            await this.handleLargeImageBatch(matchedFiles);
        } else {
            await this.handleStandardImageBatch(matchedFiles);
        }

        this.updateFileStats();
        this.displayImageGallery();

        if (this.images.size > 0) {
            this.processAllBtn.disabled = false;
            this.processSelectedBtn.disabled = false;
        }
    }

    protected async handleLargeImageBatch(matchedFiles: MatchedFile[]): Promise<void> {
        this.addToLoadingLog(`Loading large batch: ${matchedFiles.length} images`);

        // Load first page immediately
        const firstPage = matchedFiles.slice(0, this.galleryPageSize);
        await this.loadImagePage(firstPage, 0);

        // Queue remaining files
        for (let i = this.galleryPageSize; i < matchedFiles.length; i += this.galleryPageSize) {
            const page = matchedFiles.slice(i, i + this.galleryPageSize);
            this.imageLoadQueue.push({ files: page, page: Math.floor(i / this.galleryPageSize) });
        }

        this.setupLazyLoading();
        this.updateStatus(`Loaded first ${firstPage.length} images. ${matchedFiles.length - firstPage.length} queued for lazy loading.`);
    }

    private async handleStandardImageBatch(matchedFiles: MatchedFile[]): Promise<void> {
        await this.loadImagePage(matchedFiles, 0);
        this.updateStatus(`Loaded ${matchedFiles.length} images.`);
    }

    protected async loadImagePage(matchedFiles: MatchedFile[], pageIndex: number): Promise<void> {
        let imageId = this.images.size; // Continue from current count

        for (const { file, outputName } of matchedFiles) {
            try {
                const image = await this.loadImageFile(file);
                const id = `img_${imageId}`;
                this.images.set(id, {
                    id: id,
                    file: file,
                    image: image,
                    faces: [],
                    results: [],
                    selected: true,
                    processed: false,
                    csvOutputName: outputName
                });
                imageId++;
            } catch (error) {
                this.addToErrorLog(`Failed to load image ${file.name}: ${(error as Error).message}`);
            }
        }
    }

    protected setupLazyLoading(): void {
        // Add lazy loading indicator to gallery
        const indicator = document.createElement('div');
        indicator.className = 'gallery-lazy-loading';
        indicator.innerHTML = `
            <div class="lazy-loading-content">
                <span>Scroll to load more images...</span>
                <div class="lazy-loading-stats">
                    ${this.imageLoadQueue.length} pages remaining
                </div>
            </div>
        `;
        this.galleryGrid.appendChild(indicator);

        // Setup intersection observer for lazy loading
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting && !this.isLoadingImages && this.imageLoadQueue.length > 0) {
                    this.loadNextImagePage();
                }
            });
        }, {
            rootMargin: '100px' // Start loading 100px before the indicator is visible
        });

        observer.observe(indicator);
        this.lazyLoadingObserver = observer;
        this.lazyLoadingIndicator = indicator;
    }

    protected async loadNextImagePage(): Promise<void> {
        if (this.isLoadingImages || this.imageLoadQueue.length === 0) return;

        this.isLoadingImages = true;
        const nextPage = this.imageLoadQueue.shift()!;

        // Update indicator
        if (this.lazyLoadingIndicator) {
            this.lazyLoadingIndicator.innerHTML = `
                <div class="lazy-loading-content">
                    <span>Loading more images...</span>
                    <div class="lazy-loading-stats">
                        ${this.imageLoadQueue.length} pages remaining
                    </div>
                </div>
            `;
        }

        try {
            await this.loadImagePage(nextPage.files, nextPage.page);
            this.displayImageGallery(); // Refresh gallery to show new images
            this.addToLoadingLog(`Loaded page ${nextPage.page + 1} (${nextPage.files.length} images)`);
        } catch (error) {
            this.addToErrorLog(`Failed to load image page: ${(error as Error).message}`);
        }

        this.isLoadingImages = false;

        // Update or remove indicator
        if (this.imageLoadQueue.length === 0) {
            if (this.lazyLoadingIndicator) {
                this.lazyLoadingIndicator.remove();
                this.lazyLoadingObserver?.disconnect();
            }
            this.updateStatus(`All ${this.images.size} matched images loaded successfully`);
        } else if (this.lazyLoadingIndicator) {
            this.lazyLoadingIndicator.innerHTML = `
                <div class="lazy-loading-content">
                    <span>Scroll to load more images...</span>
                    <div class="lazy-loading-stats">
                        ${this.imageLoadQueue.length} pages remaining
                    </div>
                </div>
            `;
        }
    }

    private displayImageGallery(): void {
        this.galleryGrid.innerHTML = '';

        if (this.images.size === 0) {
            this.imageGallery.classList.add('hidden');
            return;
        }

        this.images.forEach((imageData) => {
            const galleryItem = this.createGalleryItem(imageData);
            this.galleryGrid.appendChild(galleryItem);
        });

        this.imageGallery.classList.remove('hidden');
        this.updateSelectionCount();
    }

    createGalleryItem(imageData: CSVImageData): HTMLDivElement {
        const item = super.createGalleryItem(imageData);
        item.addEventListener('click', () => this.toggleSelection(imageData.id));
        return item;
    }

    private toggleSelection(imageId: string): void {
        const imageData = this.images.get(imageId);
        if (imageData) {
            imageData.selected = !imageData.selected;
            this.displayImageGallery();
            this.updateSelectionCount();
        }
    }

    private selectImage(imageId: string): void {
        const imageData = this.images.get(imageId);
        if (!imageData) return;

        this.displayImageInCanvas(imageData);
        this.canvasContainer.classList.remove('hidden');
    }

    private displayImageInCanvas(imageData: CSVImageData): void {
        const canvas = this.inputCanvas;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

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

    selectAll(): void {
        this.images.forEach(imageData => {
            imageData.selected = true;
        });
        this.displayImageGallery();
    }

    selectNone(): void {
        this.images.forEach(imageData => {
            imageData.selected = false;
        });
        this.displayImageGallery();
    }

    updateSelectionCount(): void {
        const selected = Array.from(this.images.values()).filter(img => img.selected).length;
        const loaded = this.images.size;
        const queued = this.imageLoadQueue.length * this.galleryPageSize;
        const total = loaded + queued;

        this.selectedCount.textContent = selected.toString();

        if (queued > 0) {
            this.totalCount.textContent = `${loaded} (${total} total)`;
        } else {
            this.totalCount.textContent = loaded.toString();
        }
    }

    private async processAll(): Promise<void> {
        // Process both loaded images and queued file references (like batch-processing.html)
        const loadedImages = Array.from(this.images.values());
        const queuedFiles = this.imageLoadQueue.flatMap(batch => batch.files);

        if (queuedFiles.length > 0) {
            this.updateStatus(`Processing ${loadedImages.length} loaded images + ${queuedFiles.length} queued files...`);
            await this.processImagesAndFiles(loadedImages, queuedFiles);
        } else {
            await this.processImages(loadedImages);
        }
    }

    private async processSelected(): Promise<void> {
        const selectedImages = Array.from(this.images.values()).filter(img => img.selected);

        // If no images are selected but there are queued images, inform the user
        if (selectedImages.length === 0 && this.imageLoadQueue.length > 0) {
            this.updateStatus('No images selected. Use "Process All" to process all images including queued ones.');
            return;
        }

        await this.processImages(selectedImages);
    }

    private async processImagesAndFiles(loadedImages: CSVImageData[], queuedFiles: MatchedFile[]): Promise<void> {
        if (!this.detector) {
            this.updateStatus('Face detection model not loaded. Please wait and try again.');
            return;
        }

        this.isProcessing = true;
        const totalItems = loadedImages.length + queuedFiles.length;
        this.showProgress();
        this.addToProcessingLog(`Starting batch processing of ${totalItems} images (${loadedImages.length} loaded + ${queuedFiles.length} queued)`);

        let successCount = 0;
        let errorCount = 0;
        let processedCount = 0;

        try {
            // Process loaded images first
            for (const imageData of loadedImages) {
                if (imageData.selected) {
                    processedCount++;
                    this.updateProgress((processedCount / totalItems) * 100,
                        `Processing loaded image ${processedCount}/${totalItems}: ${imageData.csvOutputName}`);

                    try {
                        const success = await this.processImage(imageData.id || `img_${processedCount}`, imageData);
                        if (success) successCount++;
                        else errorCount++;
                    } catch (error) {
                        errorCount++;
                        this.addToErrorLog(`Error processing ${imageData.csvOutputName}: ${(error as Error).message}`);
                    }
                }
            }

            // Process queued files
            for (const { file, outputName } of queuedFiles) {
                processedCount++;
                this.updateProgress((processedCount / totalItems) * 100,
                    `Processing queued file ${processedCount}/${totalItems}: ${outputName}`);

                try {
                    // Load and process the queued file
                    const image = await this.loadImageFile(file);
                    const tempImageData: CSVImageData = {
                        file: file,
                        image: image,
                        faces: [],
                        results: [],
                        selected: true,
                        processed: false,
                        csvOutputName: outputName,
                        id: `queued_${processedCount}`
                    };

                    const success = await this.processImage(tempImageData.id, tempImageData);

                    // Store results from streamed processing (like batch-processing.html)
                    if (tempImageData.results.length > 0) {
                        this.imageResults.set(tempImageData.id, {
                            filename: file.name,
                            csvOutputName: outputName,
                            results: tempImageData.results,
                            processedAt: new Date().toISOString()
                        });
                    }

                    if (success) successCount++;
                    else errorCount++;
                } catch (error) {
                    errorCount++;
                    this.addToErrorLog(`Error processing queued file ${outputName}: ${(error as Error).message}`);
                }
            }

        } catch (error) {
            this.addToErrorLog(`Fatal error during batch processing: ${(error as Error).message}`);
        }

        this.isProcessing = false;
        this.hideProgress();

        // Update statistics
        this.statistics.imagesProcessed += successCount;
        this.statistics.successfulProcessing += successCount;
        this.updateStatistics();

        this.updateStatus(`Processing complete: ${successCount} successful, ${errorCount} failed`);
        this.addToProcessingLog(`Batch processing completed: ${successCount}/${totalItems} successful`);

        if (successCount > 0) {
            this.downloadAllBtn.disabled = false;
        }
    }

    private async processImages(imageDataArray: CSVImageData[]): Promise<void> {
        if (!this.detector) {
            this.updateStatus('Face detection model not loaded. Please wait and try again.');
            return;
        }

        this.isProcessing = true;
        this.updateStatus('Processing images...');
        this.addToProcessingLog(`Starting batch processing of ${imageDataArray.length} images...`);
        this.showProgress();

        let successCount = 0;
        const processingTimes: number[] = [];

        for (let i = 0; i < imageDataArray.length; i++) {
            const imageData = imageDataArray[i];
            const imageId = `processed_${i}`;

            this.updateProgress((i / imageDataArray.length) * 100, `Processing ${i + 1}/${imageDataArray.length}: ${imageData.csvOutputName}`);

            try {
                const startTime = Date.now();
                const success = await this.processImage(imageId, imageData);
                const processingTime = Date.now() - startTime;

                if (success) {
                    successCount++;
                    processingTimes.push(processingTime);
                    this.addToProcessingLog(`✓ Processed ${imageData.csvOutputName} in ${processingTime}ms`);
                } else {
                    this.addToErrorLog(`✗ Failed to process ${imageData.csvOutputName}`);
                }
            } catch (error) {
                this.addToErrorLog(`✗ Error processing ${imageData.csvOutputName}: ${(error as Error).message}`);
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

        this.updateStatus(`Processing complete: ${successCount}/${imageDataArray.length} images processed successfully`);
        this.addToProcessingLog(`Batch processing completed: ${successCount}/${imageDataArray.length} successful, avg time: ${avgTime}ms`);

        this.updateStatistics();
        this.isProcessing = false;

        if (successCount > 0) {
            this.downloadAllBtn.disabled = false;
        }
    }

    private async processImage(imageId: string, imageData: CSVImageData): Promise<boolean> {
        try {
            // Detect faces if not already done
            if (imageData.faces.length === 0) {
                const detectionResult: FaceDetectorResult = await this.detector!.detect(imageData.image);

                imageData.faces = [];
                if (detectionResult.detections && detectionResult.detections.length > 0) {
                    imageData.faces = detectionResult.detections
                        .map((detection, index) => {
                            const bbox = detection.boundingBox;
                            const box = this.convertBoundingBoxToPixels(bbox, imageData.image.width, imageData.image.height);
                            if (!box) {
                                return null;
                            }

                            const { x, y, width, height } = box;

                            return {
                                id: index,
                                box: {
                                    xMin: x,
                                    yMin: y,
                                    width: width,
                                    height: height
                                },
                                confidence: detection.categories && detection.categories.length > 0
                                    ? detection.categories[0].score
                                    : 0.8,
                                selected: true,
                                bbox: { x, y, width, height }
                            };
                        })
                        .filter((face): face is NonNullable<typeof face> => face !== null) as FaceData[];
                }
                this.statistics.totalFacesDetected += imageData.faces.length;
            }

            if (imageData.faces.length === 0) {
                this.addToErrorLog(`No faces detected in ${imageData.csvOutputName}`);
                return false;
            }

            // Process selected faces
            const selectedFaces = imageData.faces.filter(face => face.selected);
            const results: CropResult[] = [];

            for (const face of selectedFaces) {
                const croppedImage = await this.cropFace(imageData.image, face as any);
                results.push({
                    face,
                    image: croppedImage,
                    fileName: imageData.csvOutputName,
                    canvas: croppedImage,
                    bbox: face.bbox
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

    async downloadAllResults(): Promise<void> {
        // Collect results from both loaded images and streamed processing (like batch-processing.html)
        const allResults: Array<{ result: CropResult; imageData: CSVImageData | { csvOutputName: string; file: { name: string } }; faceIndex: number }> = [];

        // Collect results from loaded images
        for (const imageData of this.images.values()) {
            if (imageData.results && imageData.results.length > 0) {
                for (let i = 0; i < imageData.results.length; i++) {
                    allResults.push({
                        result: imageData.results[i],
                        imageData: imageData,
                        faceIndex: i
                    });
                }
            }
        }

        // Collect results from streamed processing
        for (const streamedResult of this.imageResults.values()) {
            if (streamedResult.results && streamedResult.results.length > 0) {
                for (let i = 0; i < streamedResult.results.length; i++) {
                    allResults.push({
                        result: streamedResult.results[i],
                        imageData: {
                            csvOutputName: streamedResult.csvOutputName,
                            file: { name: streamedResult.filename }
                        },
                        faceIndex: i
                    });
                }
            }
        }

        if (allResults.length === 0) {
            this.updateStatus('No processed results to download.');
            return;
        }

        const zip = new JSZip();
        const settings = this.getSettings();

        for (const { result, imageData, faceIndex } of allResults) {
            const filename = this.generateFilename(imageData as CSVImageData, faceIndex);

            const blob = await new Promise<Blob>((resolve) => {
                result.canvas?.toBlob((b) => resolve(b!), `image/${settings.format}`, settings.quality);
            });

            zip.file(filename, blob);
        }

        const zipBlob = await zip.generateAsync({ type: 'blob' });
        const url = URL.createObjectURL(zipBlob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'csv_batch_cropped_faces.zip';
        a.click();
        URL.revokeObjectURL(url);

        this.addToProcessingLog(`Downloaded ${allResults.length} processed face crops as ZIP`);
    }

    generateFilename(imageData: CSVImageData | { csvOutputName?: string; file?: { name: string } }, faceIndex: number): string {
        const settings = this.getSettings();
        const template = this.namingTemplate.value || '{csv_name}';
        const extension = settings.outputFormat === 'jpeg' ? 'jpg' : settings.outputFormat;
        const originalName = imageData.file ? imageData.file.name.replace(/\.[^/.]+$/, '') : 'image';

        return template
            .replace('{csv_name}', imageData.csvOutputName || originalName)
            .replace('{original}', originalName)
            .replace('{index}', (faceIndex + 1).toString())
            .replace('{timestamp}', Date.now().toString())
            .replace('{width}', settings.outputWidth.toString())
            .replace('{height}', settings.outputHeight.toString()) + '.' + extension;
    }

    clearAll(): void {
        this.images.clear();
        this.imageResults.clear(); // Clear streamed processing results
        this.csvData = [];
        this.csvHeaders = [];
        this.csvMapping.clear();
        this.csvInput.value = '';
        this.imageInput.value = '';

        // Clear lazy loading state
        this.imageLoadQueue = [];
        this.isLoadingImages = false;
        this.galleryPage = 0;

        if (this.lazyLoadingObserver) {
            this.lazyLoadingObserver.disconnect();
            this.lazyLoadingObserver = null;
        }

        if (this.lazyLoadingIndicator) {
            this.lazyLoadingIndicator.remove();
            this.lazyLoadingIndicator = null;
        }

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

    updateUI(): void {
        const hasCSVData = this.csvData.length > 0;
        const hasMappedColumns = this.filePathColumn?.value && this.fileNameColumn?.value;
        const hasImages = this.images.size > 0;
        const hasSelectedImages = Array.from(this.images.values()).some(img => img.selected);

        this.processAllBtn.disabled = !hasImages || !hasSelectedImages;
        this.processSelectedBtn.disabled = !hasImages || !hasSelectedImages;
        this.clearAllBtn.disabled = !hasCSVData && !hasImages;
        this.downloadAllBtn.disabled = true; // Enabled after processing
    }

    updateFileStats(): void {
        this.totalRows.textContent = this.csvData.length.toString();
        this.imagesUploaded.textContent = (this.imageInput?.files?.length || 0).toString();
        this.imagesMatched.textContent = this.images.size.toString();
    }

    updateStatistics(): void {
        this.totalFacesDetected.textContent = this.statistics.totalFacesDetected.toString();
        this.imagesProcessed.textContent = this.statistics.imagesProcessed.toString();

        const successRate = this.statistics.imagesProcessed > 0 ?
            Math.round((this.statistics.successfulProcessing / this.statistics.imagesProcessed) * 100) : 0;
        this.successRate.textContent = successRate + '%';

        const avgTime = this.statistics.processingTimes.length > 0 ?
            Math.round(this.statistics.processingTimes.reduce((a, b) => a + b, 0) / this.statistics.processingTimes.length) : 0;
        this.avgProcessingTime.textContent = avgTime + 'ms';
    }

    resetStatistics(): void {
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
    updateStatus(message: string): void {
        this.status.textContent = message;
        this.processingStatus.textContent = message.split('.')[0];
    }

    // Face overlay methods (simplified versions)
    private updateFaceOverlays(imageData: CSVImageData): void {
        // This would be implemented similar to other files
        // For brevity, keeping it simple here
    }

    private detectCurrentImageFaces(): void {
        // Implementation for detecting faces in currently displayed image
    }
}

// Initialize the application when the page loads
document.addEventListener('DOMContentLoaded', () => {
    new CSVBatchFaceCropper();
});

export { CSVBatchFaceCropper };
