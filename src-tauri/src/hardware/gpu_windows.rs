//! GPU enumeration through DXGI. Unlike WMI's `AdapterRAM` (a 32-bit field that
//! caps at 4 GB), DXGI reports dedicated VRAM as a 64-bit value.

use super::{is_integrated, GpuInfo, GpuVendor};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND,
};

pub fn enumerate() -> Result<Vec<GpuInfo>, String> {
    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.map_err(|e| format!("CreateDXGIFactory1: {e}"))?;

    let mut out: Vec<GpuInfo> = Vec::new();
    let mut i = 0u32;
    loop {
        let adapter = match unsafe { factory.EnumAdapters1(i) } {
            Ok(a) => a,
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(e) => return Err(format!("EnumAdapters1({i}): {e}")),
        };
        i += 1;

        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(d) => d,
            Err(_) => continue,
        };
        // Skip "Microsoft Basic Render Driver" and other software adapters.
        if desc.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
            continue;
        }

        let len = desc.Description.iter().position(|&c| c == 0).unwrap_or(desc.Description.len());
        let name = String::from_utf16_lossy(&desc.Description[..len]).trim().to_string();
        let dedicated = desc.DedicatedVideoMemory as u64;

        // The same physical GPU can appear more than once (e.g. hybrid setups).
        if out.iter().any(|g| g.name == name && g.dedicated_vram_bytes == dedicated) {
            continue;
        }

        out.push(GpuInfo {
            name,
            vendor: GpuVendor::from_pci_id(desc.VendorId),
            vendor_id: desc.VendorId,
            dedicated_vram_bytes: dedicated,
            shared_memory_bytes: desc.SharedSystemMemory as u64,
            integrated: is_integrated(dedicated),
        });
    }
    Ok(out)
}
