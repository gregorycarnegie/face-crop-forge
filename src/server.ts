// Use Bun's built-in file serving
const server = Bun.serve({
    port: 3000,
    async fetch(req) {
        const url = new URL(req.url);
        let filePath = url.pathname === '/' ? '/index.html' : url.pathname;
        
        try {
            const file = Bun.file('.' + filePath);
            
            // Check if file exists
            if (!(await file.exists())) {
                return new Response('<h1>404 Not Found</h1>', {
                    status: 404,
                    headers: { 'Content-Type': 'text/html' }
                });
            }

            // Get content type
            const mimeTypes: Record<string, string> = {
                '.html': 'text/html',
                '.js': 'text/javascript',
                '.mjs': 'text/javascript',
                '.css': 'text/css',
                '.json': 'application/json',
                '.png': 'image/png',
                '.jpg': 'image/jpg',
                '.gif': 'image/gif',
                '.svg': 'image/svg+xml',
                '.wav': 'audio/wav',
                '.mp4': 'video/mp4',
                '.woff': 'application/font-woff',
                '.ttf': 'application/font-ttf',
                '.eot': 'application/vnd.ms-fontobject',
                '.otf': 'application/font-otf',
                '.wasm': 'application/wasm',
                '.onnx': 'application/octet-stream'
            };

            const ext = filePath.substring(filePath.lastIndexOf('.')).toLowerCase();
            const contentType = mimeTypes[ext] || 'application/octet-stream';

            // Set COOP/COEP headers for WASM SIMD + threads support
            const headers: Record<string, string> = {
                'Content-Type': contentType,
                'Cross-Origin-Embedder-Policy': 'require-corp',
                'Cross-Origin-Opener-Policy': 'same-origin',
                'Cross-Origin-Resource-Policy': 'cross-origin',
            };

            // Cache control for WASM files
            if (ext === '.wasm') {
                headers['Cache-Control'] = 'public, max-age=31536000, immutable';
            }

            return new Response(file, { headers });

        } catch (error) {
            console.error('Server error:', error);
            return new Response('Internal Server Error', {
                status: 500,
                headers: { 'Content-Type': 'text/plain' }
            });
        }
    }
});

console.log(`Server running at http://localhost:${server.port}/`);