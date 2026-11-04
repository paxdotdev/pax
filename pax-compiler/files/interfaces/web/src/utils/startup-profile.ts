import type {PaxChassisWeb, StartupProfile} from '../types/pax-chassis-web';

export interface StartupCapture {
    application: 'loader' | 'main';
    completed_at_ms: number;
    report: StartupProfile;
}

declare global {
    interface Window { paxStartupProfiles?: StartupCapture[]; }
}

/** Export once per mounted host; keep separate loader/main captures across retries. */
export function captureStartupProfile(chassis: PaxChassisWeb, loader: boolean) {
    if (new URLSearchParams(window.location.search).get('pax_startup_profile') !== '1') return;
    const report = chassis.take_startup_profile();
    if (!report) return;
    const captures = window.paxStartupProfiles ??= [];
    captures.push({application: loader ? 'loader' : 'main', completed_at_ms: performance.now(), report});
    // A long-lived host can mount/retry repeatedly. The latest eight captures suffice
    // for comparison without retaining every application's diagnostic strings.
    if (captures.length > 8) captures.splice(0, captures.length - 8);
    console.info('[PaxStartupProfile]', loader ? 'loader' : 'main', {
        services: report.services, service_ms: report.service_ms, reactive: report.reactive.totals,
    });
}
