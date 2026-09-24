// Associated only with /blog and /blog/*, after CloudFront selects the origin.
// The bucket contains Zola's output at its root (no second blog/ directory).
function handler(event) {
    var request = event.request;
    if (request.uri !== '/blog' && !request.uri.startsWith('/blog/')) return request;
    var path = request.uri.slice('/blog'.length) || '/';
    if (path.endsWith('/')) {
        path += 'index.html';
    } else if (!path.slice(path.lastIndexOf('/') + 1).includes('.')) {
        path += '/index.html';
    }
    request.uri = path;
    return request;
}
