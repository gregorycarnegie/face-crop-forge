// Simple component loader for control panels
async function loadComponent(elementId, componentPath) {
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
async function loadControlPanels(config) {
    const promises = [];

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

    await Promise.all(promises);
}
