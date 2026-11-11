import {invertAffineCoeffs, packAffineCoeffsIntoMatrix3DString} from '../utils/helpers';

// The nearest Scroller supplies a local native chart. World-positioned controls
// use its inverse; retained text does not depend on the changing world pose.
export const NATIVE_SURFACE = 'var(--pax-native-surface, matrix(1,0,0,1,0,0))';
export const NATIVE_SURFACE_INVERSE = 'var(--pax-native-surface-inverse, matrix(1,0,0,1,0,0))';

export function nativeWorldTransform(matrix: number[]): string {
    return `${NATIVE_SURFACE_INVERSE} ${packAffineCoeffsIntoMatrix3DString(matrix)}`;
}

export function setNativeSurface(host: HTMLElement, matrix: number[]) {
    const inverse = invertAffineCoeffs(matrix);
    host.style.setProperty('--pax-native-surface', packAffineCoeffsIntoMatrix3DString(matrix));
    host.style.setProperty('--pax-native-surface-inverse', inverse
        ? packAffineCoeffsIntoMatrix3DString(inverse) : 'matrix(1,0,0,1,0,0)');
    host.style.transform = '';
}
