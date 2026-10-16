//! Actual Pax/WGPU capability checks, separate from native composition and performance qualification.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static ACTIVE: AtomicBool = AtomicBool::new(false);

pub(crate) fn record_event(event: Value) {
    if ACTIVE.load(Ordering::Relaxed) {
        // Flush each event so the process supervisor retains the last completed
        // stage even if a driver hangs or terminates the process without unwinding.
        let mut output = std::io::stdout().lock();
        let _ = serde_json::to_writer(&mut output, &event);
        let _ = writeln!(output);
        let _ = output.flush();
    }
}

fn adapter_class(kind: wgpu::DeviceType) -> &'static str {
    match kind {
        wgpu::DeviceType::IntegratedGpu => "integrated",
        wgpu::DeviceType::DiscreteGpu => "discrete",
        wgpu::DeviceType::VirtualGpu => "virtual",
        wgpu::DeviceType::Cpu => "software",
        wgpu::DeviceType::Other => "other",
    }
}

fn is_hardware(class: &str) -> bool {
    matches!(class, "integrated" | "discrete")
}

fn adapter_evidence(info: &wgpu::AdapterInfo) -> Value {
    json!({
        "name": info.name,
        "vendorId": info.vendor,
        "deviceId": info.device,
        "deviceType": format!("{:?}", info.device_type),
        "class": adapter_class(info.device_type),
        "driver": info.driver,
        "driverInfo": info.driver_info,
        "pciBusId": info.device_pci_bus_id,
        "backend": format!("{:?}", info.backend),
    })
}

fn try_adapter(instance: wgpu::Instance, adapter: wgpu::Adapter) -> Result<()> {
    // Exercise the same device setup, retained Pax scene/mask renderers, export
    // copies and COPY_SOURCE transition as the interactive composition producer.
    let mut context = super::ProbeContext::with_adapter(instance, adapter, 256, 192, 256, 96, 32)?;
    let handles = context.info()?;
    if [handles.device, handles.queue, handles.scene, handles.mask]
        .iter()
        .any(|handle| handle.is_null())
    {
        bail!("Pax producer returned a null DX12 export handle");
    }
    context.render(0.5)?;
    context.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })?;
    context.check_errors()?;
    drop(context);
    Ok(())
}

fn panic_detail(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .unwrap_or_else(|| "non-string panic".into())
}

fn summarize(attempts: Vec<Value>, allow_software: bool, error: Option<String>) -> Value {
    let succeeded = |class: &str| {
        attempts.iter().any(|attempt| {
            attempt["passed"] == true && attempt["adapter"]["class"].as_str() == Some(class)
        })
    };
    let hardware_passed = error.is_none() && (succeeded("integrated") || succeeded("discrete"));
    let software_passed = error.is_none() && allow_software && succeeded("software");
    let passed = hardware_passed || software_passed;
    let reason = error.unwrap_or_else(|| {
        if hardware_passed {
            "An integrated or discrete WGPU DX12 adapter completed actual Pax rendering and GPU export copies."
        } else if software_passed {
            "Software diagnostic passed; no hardware adapter qualified. Hardware gate remains failed."
        } else {
            "No eligible adapter completed actual Pax rendering. Review HAL rejections and candidate failures."
        }.into()
    });
    json!({
        "event": "result",
        "schemaVersion": 1,
        "mode": if allow_software { "hardware-or-software-diagnostic" } else { "hardware-only" },
        "passed": passed,
        "hardwarePassed": hardware_passed,
        "softwareDiagnosticPassed": software_passed,
        "nativeCompositionQualified": false,
        "physicalPerformanceQualified": false,
        "reason": reason,
        "attempts": attempts,
    })
}

/// Runs device and rendering checks without a window or WinUI runtime.
///
/// Use the PowerShell supervisor for a wall-clock deadline, crash detection and
/// artifact collection. Driver enumeration and device creation can block inside
/// native code; the GPU poll timeout alone cannot bound the whole operation.
pub fn run(allow_software: bool) -> Value {
    ACTIVE.store(true, Ordering::Relaxed);
    record_event(json!({
        "event": "start",
        "schemaVersion": 1,
        "backend": "Dx12",
        "operatingSystem": std::env::consts::OS,
        "processArchitecture": std::env::consts::ARCH,
        "allowSoftwareDiagnostic": allow_software,
        "qualification": "capability-preflight",
    }));
    let mut attempts = Vec::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
        let instance = super::dx12_instance()?;
        super::report("enumerating_adapters");
        let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12));
        // Hardware is always tried first, even when a software diagnostic is requested.
        adapters.sort_by_key(|adapter| !is_hardware(adapter_class(adapter.get_info().device_type)));
        for (index, adapter) in adapters.into_iter().enumerate() {
            let info = adapter.get_info();
            let class = adapter_class(info.device_type);
            let eligible = is_hardware(class) || (allow_software && class == "software");
            let evidence = adapter_evidence(&info);
            record_event(json!({
                "event": "candidate", "index": index,
                "adapter": evidence, "eligible": eligible,
            }));
            if !eligible {
                attempts.push(json!({
                    "adapter": evidence, "passed": false, "attempted": false,
                    "reason": if class == "software" {
                        "Software excluded; use --allow-software-diagnostic explicitly."
                    } else {
                        "VirtualGpu/Other classification does not establish hardware availability."
                    },
                }));
                continue;
            }
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                try_adapter(instance.clone(), adapter)
            }));
            let error = match result {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(format!("{error:#}")),
                Err(payload) => Some(format!("panic: {}", panic_detail(payload))),
            };
            let attempt = json!({
                "event": "attempt", "index": index, "adapter": evidence,
                "attempted": true, "passed": error.is_none(), "error": error,
            });
            record_event(attempt.clone());
            attempts.push(attempt);
        }
        Ok(())
    }));
    let error = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(format!("{error:#}")),
        Err(payload) => Some(format!("panic: {}", panic_detail(payload))),
    };
    let result = summarize(attempts, allow_software, error);
    record_event(result.clone());
    ACTIVE.store(false, Ordering::Relaxed);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(class: &str, passed: bool) -> Value {
        json!({ "adapter": { "class": class }, "passed": passed })
    }

    #[test]
    fn software_success_cannot_pass_the_default_hardware_gate() {
        let attempts = vec![attempt("integrated", false), attempt("software", true)];
        let strict = summarize(attempts.clone(), false, None);
        assert_eq!(strict["passed"], false);
        assert_eq!(strict["hardwarePassed"], false);
        assert_eq!(strict["softwareDiagnosticPassed"], false);
        let diagnostic = summarize(attempts, true, None);
        assert_eq!(diagnostic["passed"], true);
        assert_eq!(diagnostic["hardwarePassed"], false);
        assert_eq!(diagnostic["softwareDiagnosticPassed"], true);
        assert_eq!(diagnostic["physicalPerformanceQualified"], false);
    }

    #[test]
    fn virtual_and_unknown_adapters_never_establish_hardware_availability() {
        for kind in [wgpu::DeviceType::VirtualGpu, wgpu::DeviceType::Other] {
            let class = adapter_class(kind);
            assert!(!is_hardware(class));
            let result = summarize(vec![attempt(class, true)], true, None);
            assert_eq!(result["passed"], false);
            assert_eq!(result["hardwarePassed"], false);
        }
    }

    #[test]
    fn successful_hardware_is_still_only_capability_evidence() {
        for class in ["integrated", "discrete"] {
            let result = summarize(vec![attempt(class, true)], false, None);
            assert_eq!(result["passed"], true);
            assert_eq!(result["hardwarePassed"], true);
            assert_eq!(result["nativeCompositionQualified"], false);
            assert_eq!(result["physicalPerformanceQualified"], false);
        }
    }

    #[test]
    fn incomplete_run_cannot_reuse_an_earlier_candidate_success() {
        let result = summarize(
            vec![attempt("discrete", true)],
            false,
            Some("adapter enumeration interrupted".into()),
        );
        assert_eq!(result["passed"], false);
        assert_eq!(result["hardwarePassed"], false);
    }
}
