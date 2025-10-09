// Simple component loader for control panels

interface ControlPanelConfig {
    cropSettings?: boolean;
    preprocessingSettings?: boolean;
    outputSettings?: 'csv' | 'batch' | boolean;
    imageGallery?: boolean;
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

    await Promise.all(promises);
}
