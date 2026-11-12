use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::Imaging::{BitmapBounds, BitmapEncoder, SoftwareBitmap};
use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};
use windows::Win32::Foundation::{E_POINTER, HMODULE, HWND, LPARAM};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter, IDXGIDevice};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible,
};
use windows::core::{BOOL, IInspectable, Interface, Owned, PWSTR};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

fn matching_process(hwnd: HWND, expected_app: &Path, title: &[u16]) -> Result<Option<u32>> {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return Ok(None);
        }
        // One extra character beyond the terminator distinguishes a longer matching prefix.
        let mut actual_title = vec![0; title.len() + 2];
        let count = GetWindowTextW(hwnd, &mut actual_title) as usize;
        if &actual_title[..count] != title {
            return Ok(None);
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = Owned::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?);
        let mut path = vec![0; 32768];
        let mut len = path.len() as u32;
        QueryFullProcessImageNameW(
            *process,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut len,
        )?;
        let actual_app = fs::canonicalize(OsString::from_wide(&path[..len as usize]))?;
        Ok((actual_app == expected_app).then_some(pid))
    }
}

fn foreground_match(app: &Path, title: &[u16]) -> Result<Option<(HWND, u32)>> {
    unsafe extern "system" fn collect_window(hwnd: HWND, param: LPARAM) -> BOOL {
        // EnumWindows calls this synchronously while the local vector is exclusively borrowed.
        unsafe { &mut *(param.0 as *mut Vec<HWND>) }.push(hwnd);
        true.into()
    }

    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() || unsafe { IsIconic(foreground).as_bool() } {
        return Ok(None);
    }
    let Some(pid) = matching_process(foreground, app, title)? else {
        return Ok(None);
    };
    let mut windows = Vec::<HWND>::new();
    unsafe { EnumWindows(Some(collect_window), LPARAM((&raw mut windows) as isize))? };
    for hwnd in windows {
        if hwnd != foreground && matching_process(hwnd, app, title)?.is_some() {
            return Err(
                "Multiple visible windows match the specified application and title".into(),
            );
        }
    }
    Ok((unsafe { GetForegroundWindow() } == foreground).then_some((foreground, pid)))
}

fn capture_device() -> Result<IDirect3DDevice> {
    let mut device: Option<ID3D11Device> = None;
    unsafe {
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )?;
        let dxgi: IDXGIDevice = device.ok_or("No D3D11 capture device")?.cast()?;
        Ok(CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?)
    }
}

fn encode_frame(frame: Direct3D11CaptureFrame) -> Result<Vec<u8>> {
    let content = frame.ContentSize()?;
    let surface = frame.Surface()?;
    let bitmap = SoftwareBitmap::CreateCopyFromSurfaceAsync(&surface)?.join()?;
    if content.Width <= 0
        || content.Height <= 0
        || content.Width > bitmap.PixelWidth()?
        || content.Height > bitmap.PixelHeight()?
    {
        return Err("The window changed size during capture; retry".into());
    }
    let stream = InMemoryRandomAccessStream::new()?;
    let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &stream)?.join()?;
    encoder.SetSoftwareBitmap(&bitmap)?;
    encoder.BitmapTransform()?.SetBounds(BitmapBounds {
        X: 0,
        Y: 0,
        Width: content.Width as u32,
        Height: content.Height as u32,
    })?;
    encoder.FlushAsync()?.join()?;
    let length = u32::try_from(stream.Size()?)?;
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    if length == 0 || reader.LoadAsync(length)?.join()? != length {
        return Err("Incomplete in-memory PNG".into());
    }
    let mut bytes = vec![0; length as usize];
    reader.ReadBytes(&mut bytes)?;
    frame.Close()?;
    Ok(bytes)
}

fn capture(app: &Path, title: &[u16]) -> Result<Vec<u8>> {
    // A caller invoking the tool from a terminal has a moment to foreground the target.
    let deadline = Instant::now() + Duration::from_secs(2);
    let target = loop {
        if let Some(target) = foreground_match(app, title)? {
            break target;
        }
        if Instant::now() >= deadline {
            return Err(
                "No visible foreground window matched the specified application and title".into(),
            );
        }
        thread::sleep(Duration::from_millis(60));
    };
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(target.0)? };
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &capture_device()?,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        item.Size()?,
    )?;
    let session = pool.CreateCaptureSession(&item)?;
    let (send, receive) = mpsc::channel();
    let claimed = AtomicBool::new(false);
    let handler =
        TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |sender, _| {
            if !claimed.swap(true, Ordering::Relaxed) {
                let frame = sender
                    .as_ref()
                    .ok_or_else(|| windows::core::Error::from_hresult(E_POINTER))
                    .and_then(|pool| pool.TryGetNextFrame());
                let _ = send.send(frame);
            }
            Ok(())
        });
    let token = pool.FrameArrived(&handler)?;
    let png = (|| -> Result<Vec<u8>> {
        if foreground_match(app, title)? != Some(target) {
            return Err("The foreground window changed before capture".into());
        }
        session.StartCapture()?;
        let frame = receive
            .recv_timeout(Duration::from_secs(3))
            .map_err(|e| format!("Could not receive the first frame: {e}"))??;
        let bytes = encode_frame(frame)?;
        if foreground_match(app, title)? != Some(target) {
            return Err("The foreground window changed during capture".into());
        }
        Ok(bytes)
    })();
    let closed = session.Close();
    let unsubscribed = pool.RemoveFrameArrived(token);
    let pool_closed = pool.Close();
    let bytes = png?;
    closed?;
    unsubscribed?;
    pool_closed?;
    Ok(bytes)
}

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: window-capture <absolute application path> <exact foreground window title> <new PNG path>".into());
    }
    let app = PathBuf::from(&args[0]);
    let title: Vec<_> = args[1].encode_wide().collect();
    let output = PathBuf::from(&args[2]);
    if !app.is_absolute() || title.is_empty() {
        return Err("Supply an absolute application path and a nonempty window title".into());
    }
    if !output
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(OsStr::new("png")))
    {
        return Err("The output filename must end in .png".into());
    }
    if output.try_exists()? {
        return Err("The output already exists".into());
    }
    let app = fs::canonicalize(app)?;
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let result = (|| {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
            let result = capture(&app, &title);
            unsafe { RoUninitialize() };
            result
        })();
        let _ = send.send(result);
    });
    // Some graphics drivers can block even in Close(). Publish only after the worker returns.
    let bytes = receive
        .recv_timeout(Duration::from_secs(12))
        .map_err(|e| format!("Window capture did not finish: {e}"))??;
    let dir = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = NamedTempFile::new_in(dir)?;
    file.write_all(&bytes)?;
    file.flush()?;
    file.persist_noclobber(&output)?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("window-capture: {error}");
            ExitCode::FAILURE
        }
    }
}
