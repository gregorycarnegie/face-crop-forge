// Simple component loader for control panels

interface UploadCardConfig {
    containerId: string;
    inputId: string;
    config: {
        title?: string;
        subtitle?: string;
        buttonText?: string;
        accept?: string;
        multiple?: boolean;
    };
}

interface ControlPanelConfig {
    cropSettings?: boolean;
    preprocessingSettings?: boolean;
    outputSettings?: 'csv' | 'batch' | boolean;
    imageGallery?: boolean;
    uploadCard?: UploadCardConfig;
    imageUploadCard?: UploadCardConfig;
}

async function loadComponent(elementId: string, componentPath: string): Promise<void> {
    try {
        const response = await fetch(componentPath);
        if (!response.ok) {
            throw new Error(`Failed to load component: ${componentPath}`);
        }
        const html = await response.text();
        const element = document.getElementById(elementId);
        if (element) {
            element.innerHTML = html;
        } else {
            console.error(`Element with id '${elementId}' not found`);
        }
    } catch (error) {
        console.error('Error loading component:', error);
    }
}

function initUploadCard(containerId: string, inputId: string, config?: UploadCardConfig['config']): void {
    const container = document.getElementById(containerId);
    if (!container) {
        console.error('Upload card: Container not found with ID:', containerId);
        return;
    }

    const dropzone = container.querySelector('[data-upload-dropzone]') as HTMLElement;
    const titleEl = container.querySelector('[data-upload-title]') as HTMLElement;
    const subtitleEl = container.querySelector('[data-upload-subtitle]') as HTMLElement;
    const triggerBtn = container.querySelector('[data-upload-trigger]') as HTMLButtonElement;

    if (!dropzone) {
        console.error('Upload card: Dropzone not found in container:', containerId);
        return;
    }

    // Create and inject the file input with the correct ID
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.id = inputId;
    fileInput.className = 'upload-input';
    fileInput.accept = config?.accept || 'image/*';
    if (config?.multiple) {
        fileInput.setAttribute('multiple', '');
    }
    dropzone.appendChild(fileInput);

    // Apply configuration
    if (config) {
        if (config.title && titleEl) titleEl.textContent = config.title;
        if (config.subtitle && subtitleEl) subtitleEl.textContent = config.subtitle;
        if (config.buttonText && triggerBtn) triggerBtn.textContent = config.buttonText;
    }

    // Prevent default drag behaviors
    function preventDefaults(e: Event) {
        e.preventDefault();
        e.stopPropagation();
    }

    ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
        dropzone.addEventListener(eventName, preventDefaults, false);
    });

    // Highlight drop zone when item is dragged over it
    ['dragenter', 'dragover'].forEach(eventName => {
        dropzone.addEventListener(eventName, () => {
            dropzone.classList.add('drag-over');
        }, false);
    });

    ['dragleave', 'drop'].forEach(eventName => {
        dropzone.addEventListener(eventName, () => {
            dropzone.classList.remove('drag-over');
        }, false);
    });

    // Handle dropped files
    dropzone.addEventListener('drop', (e) => {
        const dt = (e as DragEvent).dataTransfer;
        const files = dt?.files;

        if (files && files.length > 0) {
            // Set files to input
            fileInput.files = files;
            // Trigger change event
            const event = new Event('change', { bubbles: true });
            fileInput.dispatchEvent(event);
        }
    }, false);

    // Handle click to open file dialog
    if (triggerBtn) {
        triggerBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            fileInput.click();
        });
    }

    dropzone.addEventListener('click', (e) => {
        if (e.target === dropzone || (e.target as HTMLElement).closest('.upload-content')) {
            fileInput.click();
        }
    });

    console.log('Upload card initialized:', inputId);
}

async function loadUploadCard(uploadConfig: UploadCardConfig): Promise<void> {
    // Load the component HTML
    await loadComponent(uploadConfig.containerId, '/components/upload-card.html');

    // Small delay to ensure DOM is ready
    await new Promise(resolve => setTimeout(resolve, 10));

    // Initialize the upload card
    initUploadCard(uploadConfig.containerId, uploadConfig.inputId, uploadConfig.config);
}

// Load all control panel components for a page
async function loadControlPanels(config: ControlPanelConfig): Promise<void> {
    const promises: Promise<void>[] = [];

    if (config.cropSettings) {
        promises.push(loadComponent('crop-settings-container', '/components/crop-settings.html'));
    }

    if (config.preprocessingSettings) {
        promises.push(loadComponent('preprocessing-settings-container', '/components/preprocessing-settings.html'));
    }

    if (config.outputSettings) {
        const outputFile = config.outputSettings === 'csv'
            ? '/components/output-settings-csv.html'
            : '/components/output-settings-batch.html';
        promises.push(loadComponent('output-settings-container', outputFile));
    }

    if (config.imageGallery) {
        promises.push(loadComponent('image-gallery-container', '/components/image-gallery.html'));
    }

    if (config.uploadCard) {
        promises.push(loadUploadCard(config.uploadCard));
    }

    if (config.imageUploadCard) {
        promises.push(loadUploadCard(config.imageUploadCard));
    }

    await Promise.all(promises);
}
