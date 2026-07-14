export async function readImageToByteBuffer(imagePath: string): Promise<{ pixels: Uint8ClampedArray, width: number, height: number }> {
    let attempts = 8;
    let delay = 100;
    let lastError: unknown = new Error("image load did not start");
    while (attempts > 0) {
        try {
            const response = await fetch(imagePath);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status} ${response.statusText}`.trim());
            }
            const blob = await response.blob();
            const img = await createImageBitmap(blob);
            try {
                const canvas = new OffscreenCanvas(img.width, img.height);
                const ctx = canvas.getContext('2d');
                if (ctx == null) {
                    throw new Error("unable to create image decode canvas");
                }
                ctx.drawImage(img, 0, 0, img.width, img.height);
                const imageData = ctx.getImageData(0, 0, img.width, img.height);
                return { pixels: imageData.data, width: img.width, height: img.height };
            } finally {
                img.close();
            }
        } catch (error) {
            lastError = error;
            attempts--;
            if (attempts === 0) {
                break;
            }
            await new Promise(resolve => setTimeout(resolve, delay));
            delay *= 2;
        }
    }
    throw new Error(`Failed to load image "${imagePath}" after maximum retries: ${String(lastError)}`);
}

export const HIDDEN_TAB_FRAME_FALLBACK_MS = 250;

export function waitForDocumentFrame(
    targetDocument: Document = document,
    hiddenDelayMs: number = HIDDEN_TAB_FRAME_FALLBACK_MS,
): Promise<void> {
    const view = targetDocument.defaultView;
    const hidden = targetDocument.hidden || targetDocument.visibilityState === "hidden";
    return new Promise<void>((resolve) => {
        if (!hidden && view != null && typeof view.requestAnimationFrame === "function") {
            view.requestAnimationFrame(() => resolve());
            return;
        }
        const scheduleTimeout = view?.setTimeout?.bind(view) ?? globalThis.setTimeout.bind(globalThis);
        scheduleTimeout(() => resolve(), hiddenDelayMs);
    });
}

//Required due to Safari bug, unable to clip DOM elements to SVG=>`transform: matrix(...)` elements; see https://bugs.webkit.org/show_bug.cgi?id=126207
//  and repro in this repo: `878576bf0e9`
//Work-around is to manually affine-multiply coordinates of relevant elements and plot as `Path`s (without `transform`) in SVG.
//
//For the point V [x,y]
//And the affine coefficients in column-major order, (a,b,c,d,e,f) representing the matrix M:
//  | a c e |
//  | b d f |
//  | 0 0 1 |
//Return the product `V * M`
// Given a matrix A∈ℝm×n and vector x∈ℝn the matrix-vector multiplication of A and x is defined as
// Ax:=x1a∗,1+x2a∗,2+⋯+xna∗,n
// where a∗,i is the ith column vector of A.
export function affineMultiply(point: number[], matrix: number[]) : number[] {
    let x = point[0];
    let y = point[1];
    let a = matrix[0];
    let b = matrix[1];
    let c = matrix[2];
    let d = matrix[3];
    let e = matrix[4];
    let f = matrix[5];
    let xOut = a*x + c*y + e;
    let yOut = b*x + d*y + f;
    return [xOut, yOut];
}

export function invertAffineCoeffs(coeffs: number[]): number[] | undefined {
    if (coeffs.length < 6) {
        return undefined;
    }

    const [a, b, c, d, e, f] = coeffs;
    const determinant = a * d - b * c;
    if (Math.abs(determinant) < 1e-8) {
        return undefined;
    }

    const inverseDeterminant = 1 / determinant;
    const na = d * inverseDeterminant;
    const nb = -b * inverseDeterminant;
    const nc = -c * inverseDeterminant;
    const nd = a * inverseDeterminant;
    const ne = -(na * e + nc * f);
    const nf = -(nb * e + nd * f);
    return [na, nb, nc, nd, ne, nf];
}

/// Our 2D affine transform comes across the wire as an array of
/// floats in column-major order, (a,b,c,d,e,f) representing the
/// augmented matrix:
///  | a c e |
///  | b d f |
///  | 0 0 1 |
///
///  In order to pack this into a CSS-ready matrix3d format, we must
///  imagine packing into the following matrix for a "dont-care Z"
///
///  | a c 0 e |
///  | b d 0 f |
///  | 0 0 1 0 | //note that 1 will preserve a dont-care z, vs 0 will 'flatten' it
///  | 0 0 0 1 |
///
///  and then unroll into a comma-separated list, following column-major order
///
export function packAffineCoeffsIntoMatrix3DString(coeffs: number[]) : string {
    return "matrix3d(" + [
        //begin column 0
        coeffs[0].toFixed(6),
        coeffs[1].toFixed(6),
        0,
        0,
        //begin column 1
        coeffs[2].toFixed(6),
        coeffs[3].toFixed(6),
        0,
        0,
        //begin column 2
        0,
        0,
        1,
        0,
        //begin column 3
        coeffs[4].toFixed(6),
        coeffs[5].toFixed(6),
        0,
        1
    ].join(",") + ")";
}


//Rectilinear-affine alternative to `clip-path: path(...)` clipping.  Might be faster than `path`
export function getQuadClipPolygonCommand(width: number, height: number, transform: number[]) {
    let point0 = affineMultiply([0, 0], transform);
    let point1 = affineMultiply([width, 0], transform);
    let point2 = affineMultiply([width, height], transform);
    let point3 = affineMultiply([0, height], transform);

    let polygon = `polygon(${point0[0]}px ${point0[1]}px, ${point1[0]}px ${point1[1]}px, ${point2[0]}px ${point2[1]}px, ${point3[0]}px ${point3[1]}px)`
    return polygon;
}

export function arrayToKey(arr: number[]): string {
    return arr.join(',');
}
