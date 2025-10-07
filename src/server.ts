import * as http from 'http';
import * as fs from 'fs';
import * as path from 'path';

interface MimeTypes {
    [key: string]: string;
}

const server = http.createServer((req: http.IncomingMessage, res: http.ServerResponse): void => {
    let filePath: string = '.' + req.url;
    if (filePath === './') {
        filePath = './index.html';
    }

    const extname: string = String(path.extname(filePath)).toLowerCase();
    const mimeTypes: MimeTypes = {
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

    const contentType: string = mimeTypes[extname] || 'application/octet-stream';

    fs.readFile(filePath, (error: NodeJS.ErrnoException | null, content: Buffer): void => {
        if (error) {
            if (error.code === 'ENOENT') {
                res.writeHead(404, { 'Content-Type': 'text/html' });
                res.end('<h1>404 Not Found</h1>', 'utf-8');
            } else {
                res.writeHead(500);
                res.end('Sorry, check with the site admin for error: ' + error.code + ' ..\n');
            }
        } else {
            // Set COOP/COEP headers for WASM SIMD + threads support
            // These enable SharedArrayBuffer for 1.5-3× performance boost in modern Chromium
            res.writeHead(200, {
                'Content-Type': contentType,
                'Cross-Origin-Embedder-Policy': 'require-corp',
                'Cross-Origin-Opener-Policy': 'same-origin',
                // Additional security headers
                'Cross-Origin-Resource-Policy': 'cross-origin',
                // Cache control for WASM files
                ...(extname === '.wasm' && {
                    'Cache-Control': 'public, max-age=31536000, immutable'
                })
            });
            res.end(content, 'utf-8');
        }
    });
});

const PORT: number = 3000;
server.listen(PORT, (): void => {
    console.log(`Server running at http://localhost:${PORT}/`);
});
